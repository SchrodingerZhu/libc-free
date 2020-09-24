use core::panic::PanicInfo;

use syscalls::*;
use crate::write::*;

extern "C" {
    fn main() -> isize;
}


#[no_mangle]
unsafe extern "C" fn _start() {
    let result = main();
    syscall!(SYS_exit, result).unwrap();
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        WRITER.lock()._write_str("[EXCEPTION]\n");
        crate::eprintln!("{}", info);
        syscall!(SYS_exit, 1).unwrap();
        core::hint::unreachable_unchecked()
    }
}