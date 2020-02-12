#![no_std]

use core::{fmt, slice, ptr, mem};
use core::cmp::min;
use core::fmt::Write;
//use core::ptr::copy_nonoverlapping;

#[repr(C)]
pub struct Stlinky {
    magic: u32,
    buf_size: u32,
    up_tail: u32,
    up_head: u32,
    dw_tail: u32,
    dw_head: u32,
}

const HEADER_SIZE: usize = mem::size_of::<Stlinky>();

const STLINKY_MAGIC: u32 = 0xdeadf00d;

#[repr(align(4))]
#[derive(Copy, Clone)]
pub struct StlinkyBuffer(pub u8);

pub static mut STDOUT: Option<&'static mut Stlinky> = None;

fn _is_full(head: usize, tail: usize, buf_size: usize) -> bool {
    (tail == 0 && head == buf_size - 1) || (tail > head && tail - head == 1)
}

impl Stlinky {
    pub fn new_at<'a>(buffer: &'a mut [StlinkyBuffer]) -> &mut Stlinky {
        let buf_size = (buffer.len() - HEADER_SIZE) / 2;

        assert!(buf_size > 1);

        let stl = unsafe { mem::transmute::<&mut StlinkyBuffer, &mut Stlinky>(&mut buffer[0]) };
        stl.buf_size = buf_size as u32;
        stl.up_tail = 0;
        stl.up_head = 0;
        stl.dw_tail = 0;
        stl.dw_head = 0;
        stl.magic = STLINKY_MAGIC;
        //unsafe { ptr::write_volatile(&mut stl.magic, STLINKY_MAGIC); }

        stl
    }

    pub fn _read(&mut self, _buf: &mut [u8]) -> usize {
        unimplemented!();
    }

    pub fn write(&mut self, mut buf: &[u8]) -> usize {
        let tail = unsafe { ptr::read_volatile(&self.up_tail) } as usize;
        let mut head = unsafe { ptr::read_volatile(&self.up_head) } as usize;
        let buf_size = self.buf_size as usize;

        if (tail == 0 && head == buf_size - 1) || (tail > head && tail - head == 1) {
            // no space left

            return 0;
        }

        let mut total: usize = 0;

        if head >= tail {
            // can write from head up to end of buffer (maybe exclusive)

            let count = min(buf.len(), buf_size - head - if tail == 0 { 1 } else { 0 });

            self.up_buf()[head..head+count].copy_from_slice(&buf[0..count]);

            total += count;
            head += count;

            if head >= buf_size {
                head = 0;
            }

            buf = &buf[count..];
        }

        if buf.len() > 0 && tail > head {
            // can write from head to tail (exclusive)

            let count = min(buf.len(), tail - head - 1);

            self.up_buf()[head..head+count].copy_from_slice(&buf[0..count]);

            total += count;
            head += count;
        }

        //unsafe { ptr::write_volatile(&mut self.up_head, (head + total) as u32); }
        unsafe { ptr::write_volatile(&mut self.up_head, head as u32); }

        total
    }

    fn up_buf(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                (self as *mut Stlinky as *mut u8).offset(HEADER_SIZE as isize),
                self.buf_size as usize)
        }
    }

    fn _dw_buf(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                (self as *mut Stlinky as *mut u8).offset(HEADER_SIZE as isize + self.buf_size as isize),
                self.buf_size as usize)
        }
    }
}

impl Write for Stlinky {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        let bytes = s.as_bytes();
        let mut total: usize = 0;

        loop {
            let n = self.write(&bytes[total..]);
            total += n;

            if total == bytes.len() {
                break;
            }

            // TODO: non-blocking write
        }

        Ok(())
    }
}

/*struct NonBlockingStlinky<'a>(&'a mut Stlinky);

impl<'a> fmt::Write for NonBlockingStlinky<'a> {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        self.0.write(&s.as_bytes());
        Ok(())
    }
}*/

pub fn set_stdout_at(buffer: &'static mut [StlinkyBuffer]) {
    set_stdout(Stlinky::new_at(buffer));
}

pub fn set_stdout(stlinky: &'static mut Stlinky) {
    unsafe { STDOUT = Some(stlinky) };
}

pub fn stdout() -> Option<&'static mut Stlinky> {
    match unsafe { &mut STDOUT } {
        Some(ref mut stdout) => Some(stdout),
        None => None
    }
}

#[macro_export]
macro_rules! stlinky_buffer {
    ($name:ident, $buf_size:expr) => {
        pub static mut $name: [$crate::StlinkyBuffer; $buf_size] = [$crate::StlinkyBuffer(0); $buf_size];
    }
}

#[macro_export]
macro_rules! sprint {
    ($s:expr) => {
        if let Some(ref mut stdout) = unsafe { &mut $crate::STDOUT } {
            ::core::fmt::Write::write_str(stdout, $s).unwrap();
        }
    };
    ($($arg:tt)*) => {
        if let Some(ref mut stdout) = unsafe { &mut $crate::STDOUT } {
            ::core::fmt::Write::write_fmt(stdout, format_args!($($arg)*)).unwrap();
        }
    };
}

#[macro_export]
macro_rules! sprintln {
    () => { $crate::sprint!("\n"); };
    ($fmt:expr) => { $crate::sprint!(concat!($fmt, "\n")); };
    ($fmt:expr, $($arg:tt)*) => { $crate::sprint!(concat!($fmt, "\n"), $($arg)*); };
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    sprintln!("{}", info);

    loop {}
}
