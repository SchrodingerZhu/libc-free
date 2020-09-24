use core::mem::size_of;
use syscalls::*;
use crate::flag;
use core::alloc::Layout;
use crate::write::{WRITER, EWRITER};


struct NaiveAllocator;

#[global_allocator]
static ALLOC: NaiveAllocator = NaiveAllocator;

unsafe impl core::alloc::GlobalAlloc for NaiveAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        syscall!(
            SYS_mmap,
            0,
            layout.size(),
            flag::PROT_READ | flag::PROT_WRITE,
            flag::MAP_PRIVATE | flag::MAP_ANON
        ).unwrap_or(0) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match syscall!(SYS_munmap, ptr, layout.size()) {
            _ => ()
        }
    }
}

#[no_mangle]
unsafe extern "C" fn memcpy(dst: *mut u8,
                            src: *const u8,
                            size: usize) -> *mut u8 {
    use core::arch::x86_64::*;
    const MASK: usize = 31;
    let preset = MASK & size;
    for i in 0..preset {
        *dst.add(i as usize) = *src.add(i as usize);
    }
    for i in (preset..size).step_by(32) {
        let v = _mm_lddqu_si128(&*(src.add(i as usize) as *const _));
        _mm_storeu_si128(&mut *(dst.add(i as usize) as *mut _), v);
    }
    dst
}

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    unsafe {
        syscall!(SYS_exit, 1).unwrap();
        core::hint::unreachable_unchecked();
    }
}