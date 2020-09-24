use core::panic::PanicInfo;

use syscalls::*;

extern "C" {
    fn main() -> isize;
}


#[no_mangle]
unsafe extern "system" fn _start() {
    let result = main();
    syscall!(SYS_exit, result).unwrap();
}

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        syscall!(SYS_exit, 114514).unwrap();
        core::hint::unreachable_unchecked()
    }
}