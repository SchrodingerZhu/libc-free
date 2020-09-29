use core::mem::size_of;
use syscalls::*;
use crate::flag;
use core::alloc::Layout;
use crate::write::{WRITER, EWRITER};
use std::sync::atomic::AtomicPtr;


pub struct NaiveAllocator;

#[cfg_attr(not(test), global_allocator)]
pub static NAIVE_ALLOC: NaiveAllocator = NaiveAllocator;

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

#[cfg(not(test))]
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
        let v = _mm256_loadu_si256(&mut *((src as usize + i) as *mut _));
        _mm256_stream_si256(&mut *((dst as usize + i) as *mut _), v);
    }
    dst
}

#[cfg(not(test))]
#[no_mangle]
unsafe extern "C" fn memset(dst: *mut u8,
                            value: i8,
                            size: usize) {
    use core::arch::x86_64::*;
    const MASK: usize = 31;
    let preset = MASK & size;
    for i in 0..preset {
        *dst.add(i as usize) = 0;
    }
    for i in (preset..size).step_by(32) {
        _mm256_stream_si256(&mut *((dst as usize + i) as *mut _), _mm256_set1_epi8(value));
    }
}




#[cfg(not(test))]
#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    unsafe {
        crate::eprintln!("unable to alloc {:#?}", layout);
        syscall!(SYS_exit, 1).unwrap();
        core::hint::unreachable_unchecked();
    }
}

pub const SEGMENT_MASK : usize = 0xffffffffffc00000;
pub const SEGMENT_SHIFT : usize = 22;
pub const SEGMENT_SIZE : usize = 4194304;
pub const SMALL_PAGE_SHIFT : usize = 16;
pub const SMALL_PAGE_SIZE : usize = 65536;
pub const MID_PAGE_SHIFT : usize = 19;
pub const MID_PAGE_SIZE : usize = 524288;
pub const HUGE_PAGE_SHIFT : usize = 22;
pub const HUGE_PAGE_SIZE : usize = 4194304;

enum PageType {
    SMALL, MID, HUGE
}

#[repr(C)]
struct LocalBlock {
    next: *mut Block,
}

#[repr(C)]
struct Block {
    next: AtomicPtr<Block>,
}

#[repr(C, align(4096))]
struct Page {
    thread_free: AtomicPtr<Block>,
    local_free: *mut LocalBlock,
    free: *mut LocalBlock,
    page_type: PageType,
    next: *mut Page,
    prev: *mut Page,
    block_size: usize,
    segment_idx: usize
}

#[repr(C, align(4194304))]
struct Segment {
    thread_id: u64,
    page_shift: usize,
    page_start: *mut Page,
}

unsafe fn locate_page(ptr: *mut u8) -> &'static mut Page {
    let ptr = ptr as usize;
    let segment = &mut *((ptr & MASK) as *mut Segment);
    let
}

#[repr(C)]
struct Heap {
    next: *mut Block,
    page_direct: [*mut Page; 128],
    page_queues: [PageQueue; ]
}