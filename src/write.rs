use syscalls::*;

static WRITER: Writer = Writer;


pub struct Writer;


impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe {
            match syscall!(SYS_write, 1, s.as_ptr(), s.len()) {
                _ => ()
            }
            Ok(())
        }
    }
}