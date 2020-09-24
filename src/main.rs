#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(test, feature(test))]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(alloc_prelude)]
#![feature(llvm_asm)]


extern crate alloc;
use syscalls::*;
use core::fmt::Write;

mod flag;
mod write;
mod sync;
mod memory;

#[cfg(not(test))]
mod runtime;

#[cfg(test)]
extern crate test;



#[no_mangle]
fn main() -> i32 {
    panic!("123");
    0
}
