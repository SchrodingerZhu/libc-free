use core::mem::size_of;
use syscalls::*;
use crate::flag;
use core::alloc::Layout;
use crate::write::{WRITER, EWRITER};
use core::sync::atomic::*;

const NUMA_LIMIT : usize = 256;

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



#[inline]
fn address_hint(alignment: usize, size: usize) -> usize {
    static ALIGNED_BASE : AtomicUsize = AtomicUsize::new(0);
    if alignment == 0 || alignment > SEGMENT_SIZE {
        return 0;
    }
    if alignment % SEGMENT_SIZE != 0 {
        return 0;
    }
    let mut hint = ALIGNED_BASE.fetch_add(size, Ordering::AcqRel);
    if hint == 0 || hint > 30 << 40 {
        ALIGNED_BASE.compare_and_swap(hint + size, 4 << 40, Ordering::AcqRel);
        hint = ALIGNED_BASE.fetch_add(size, Ordering::AcqRel);
    }
    if hint % alignment != 0 {
        0
    } else {
        hint
    }
}

unsafe fn hinted_mmap(addr: *mut u8, size: usize, alignment: usize, prot_flags: i64, map_flags: i64, fd: i64) -> *mut u8 {
    let mut result = core::ptr::null_mut();
    let hint = address_hint(alignment, size);
    if addr.is_null() && hint != 0 {
        if let Ok(res) = syscall!(SYS_mmap, hint, size, prot_flags, map_flags, fd, 0) {
            result = res as *mut u8;
        }
    }
    if result.is_null() {
        if let Ok(res) = syscall!(SYS_mmap, addr, size, prot_flags, map_flags, fd, 0) {
            result = res as *mut u8;
        }
    }
    result
}

unsafe fn munmap(addr: *mut u8, size: usize) -> bool {
    syscall!(SYS_munmap, addr, size).is_ok()
}

unsafe fn numa_count() -> usize {
    static mut NUMA_COUNT : usize = 0;
    static mut BUFFER : [u8;33] =  [0; 33];
    static PREFIX : &'static [u8] = b"/sys/devices/system/node/node";
    unsafe fn set(mut i: u8) {
        // we do not want to fuck up with IO operations,
        // so we write a temporary format function for file access
        let mut cursor = PREFIX.len();
        if i >= 100 {
            BUFFER[cursor] = i / 100 + 48;
            cursor += 1;
        }
        if i >= 10 {
            BUFFER[cursor] = (i % 100) / 10 + 48;
            cursor += 1;
        }
        BUFFER[cursor] = i % 10 + 48;
        BUFFER[cursor + 1] = 0;
    }
    if core::intrinsics::unlikely(NUMA_COUNT == 0) {
        core::ptr::copy_nonoverlapping(PREFIX.as_ptr() as *mut u8, BUFFER.as_mut_ptr() as *mut u8, 29);
        for i in 0..NUMA_LIMIT {
            set(i as u8);
            #[cfg(test)]
                {
                    print!("checking {:?}", std::ffi::CStr::from_ptr(BUFFER.as_ptr() as *mut i8));
                }
            if let Ok(_) = syscall!(SYS_access, &BUFFER as *const u8 as usize, 4) {
                #[cfg(test)]
                    {
                        println!(" [SUCCESS]");
                    }
                NUMA_COUNT += 1;
            }
            else {
                #[cfg(test)]
                    {
                        println!(" [FAILED]");
                    }
            }
        }
    }
    NUMA_COUNT
}

unsafe fn current_numa_node() -> usize {
    if numa_count() <= 1 {
        return 0;
    }
    let mut node = 0;
    let mut ncpu = 0;
    if syscall!(SYS_getcpu, &ncpu as *const _, &node as *const _, 0).is_ok() {
        node
    } else {
        0
    }
}

#[repr(C)]
struct Heap {
    next: *mut Block,
    page_direct: [*mut Page; 128]
}

#[cfg(test)]
mod test {
    #[test]
    fn test_numa() {
        unsafe {
            println!("{}, {}", super::numa_count(), super::numa_count());
        }
    }
    #[test]
    fn test_numa_node() {
        unsafe {
            println!("{}", super::current_numa_node());
        }
    }
}