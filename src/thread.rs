use crate::memory::NAIVE_ALLOC;
use core::alloc::*;
use syscalls::*;

pub const ARCH_SET_GS : u64 = 		0x1001;
pub const ARCH_SET_FS : u64 = 		0x1002;
pub const ARCH_GET_FS : u64 = 		0x1003;
pub const ARCH_GET_GS : u64 = 		0x1004;
// thread control block

#[repr(C)]
pub struct Thread {
    pub ppid: u64,
    pub tid: u64,
    tls_map: *const [u8],
    tls_block_start: *mut u8,
    tls_dtor_list: *mut u8,
    local_free_list: [*mut u8; 32]
}

#[repr(C)]
pub struct PaddedThread {
    __thread: *mut Thread,
    thread: Thread
}

pub unsafe fn init_main_thread() {
    let tcb = NAIVE_ALLOC.alloc(Layout::new::<PaddedThread>()) as *mut PaddedThread;
    core::ptr::write_bytes(tcb, 0, 1);
    (*tcb).__thread = (tcb as usize + 8) as *mut Thread;
    let thread = &mut (*tcb).thread;
    thread.ppid = syscall!(SYS_getpid).unwrap() as u64;
    thread.tid = syscall!(SYS_gettid).unwrap() as u64;
    syscall!(SYS_arch_prctl, ARCH_SET_FS, tcb).unwrap();
}

pub unsafe fn thread_self() -> &'static mut Thread {
    let thread : *mut Thread;
    llvm_asm!(
        "mov %fs:0x0, $0" :
         "=r"(thread) ::: "memory"
    );
    return &mut *thread;
}

pub unsafe fn munmap_self() {
    NAIVE_ALLOC.dealloc((thread_self() as *mut _ as usize - 8) as *mut u8, Layout::new::<PaddedThread>());
}