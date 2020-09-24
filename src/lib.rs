#![cfg_attr(not(test), no_std)]
#![cfg_attr(test, feature(test))]
#![feature(llvm_asm)]


use core::mem::size_of;
use syscalls::*;
mod flag;
mod write;
mod sync;

#[cfg(not(test))]
mod runtime;

#[cfg(test)]
extern crate test;

unsafe fn system_malloc(size: usize) -> Result<*mut (), i64> {
    let padding = size_of::<usize>();
    let result = syscall!(
        SYS_mmap,
        0,
        padding + size,
        flag::PROT_READ | flag::PROT_WRITE,
        flag::MAP_PRIVATE | flag::MAP_ANON
    )? as *mut usize;
    *result = size;
    Ok((result as usize + 1) as *mut ())
}

unsafe fn system_dealloc(ptr: *mut ()) -> Result<(), i64> {
    let real_ptr = (ptr as usize - 1) as *mut usize;
    let real_size = *real_ptr;
    syscall!(SYS_munmap, real_ptr, real_size)?;
    Ok(())
}



struct Thread<'a> {
    operation: &'a dyn FnOnce() -> (),
    stack_size: usize,
    ppid: i64,
    tid: i64
}

impl<'a> Thread<'a> {
    unsafe fn spawn(&mut self) -> Result<(), i64> {
        let stack_region = system_malloc(self.stack_size);
        unimplemented!()
    }
}


