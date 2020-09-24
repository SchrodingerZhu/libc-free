use syscalls::*;
use crate::sync::Futex;
use core::cell::UnsafeCell;
use core::sync::atomic::AtomicU64;

pub struct Writer {
    fd: u64,
}

impl Writer {
    pub fn _write_str(&mut self, s: &str){
        unsafe {
            match syscall!(SYS_write, self.fd, s.as_ptr(), s.len()) {
                _ => ()
            }
        }
    }
}

#[no_mangle]
pub static WRITER: Futex<Writer> = Futex {
    _flag: AtomicU64::new(0),
    item: UnsafeCell::new(Writer {
        fd: 1,
    })
};

#[no_mangle]
pub static EWRITER: Futex<Writer> = Futex {
    _flag: AtomicU64::new(0),
    item: UnsafeCell::new(Writer {
        fd: 2,
    })
};


impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self._write_str(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::write::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {
        ($crate::write::_print(format_args!($($arg)*)));
        ($crate::print!("\n"));
    }
}


#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => ($crate::write::_eprint(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprintln {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {
        ($crate::write::_eprint(format_args!($($arg)*)));
        ($crate::print!("\n"));
    }
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _eprint(args: core::fmt::Arguments) {
    use core::fmt::Write;

    EWRITER.lock().write_fmt(args).unwrap();
}