#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(test, feature(test))]
#![feature(core_intrinsics)]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(alloc_prelude)]
#![feature(llvm_asm)]
#![feature(asm)]


extern crate alloc;
use syscalls::*;
use core::fmt::Write;

mod flag;
mod write;
mod sync;
mod memory;
mod thread;
#[cfg(not(test))]
mod runtime;

#[cfg(test)]
extern crate test;

#[cfg(not(test))]
#[no_mangle]
fn main() -> i32 {
    let s = unsafe { thread::thread_self() };
    println!("{}", s.ppid);
    0
}
