// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::sync::atomic::AtomicU64;

// Extension trait to add lock elision primitives to atomic types
pub trait AtomicElisionExt {
    type IntType;

    // Perform a compare_exchange and start a transaction
    fn elision_compare_exchange_acquire(
        &self,
        current: Self::IntType,
        new: Self::IntType,
    ) -> Result<Self::IntType, Self::IntType>;

    // Perform a fetch_sub and end a transaction
    fn elision_fetch_sub_release(&self, val: Self::IntType) -> Self::IntType;
}


impl AtomicElisionExt for AtomicU64 {
    type IntType = u64;

    #[cfg(target_pointer_width = "32")]
    #[inline]
    fn elision_compare_exchange_acquire(&self, current: u64, new: u64) -> Result<u64, u64> {
        unsafe {
            let prev: u64;
            llvm_asm!("xacquire; lock; cmpxchgl $2, $1"
                      : "={eax}" (prev), "+*m" (self)
                      : "r" (new), "{eax}" (current)
                      : "memory"
                      : "volatile");
            if prev == current {
                Ok(prev)
            } else {
                Err(prev)
            }
        }
    }
    #[cfg(target_pointer_width = "64")]
    #[inline]
    fn elision_compare_exchange_acquire(&self, current: u64, new: u64) -> Result<u64, u64> {
        unsafe {
            let prev: u64;
            llvm_asm!("xacquire; lock; cmpxchgq $2, $1"
                      : "={rax}" (prev), "+*m" (self)
                      : "r" (new), "{rax}" (current)
                      : "memory"
                      : "volatile");
            if prev == current {
                Ok(prev)
            } else {
                Err(prev)
            }
        }
    }

    #[cfg(target_pointer_width = "32")]
    #[inline]
    fn elision_fetch_sub_release(&self, val: u64) -> u64 {
        unsafe {
            let prev: u64;
            llvm_asm!("xrelease; lock; xaddl $2, $1"
                      : "=r" (prev), "+*m" (self)
                      : "0" (val.wrapping_neg())
                      : "memory"
                      : "volatile");
            prev
        }
    }
    #[cfg(target_pointer_width = "64")]
    #[inline]
    fn elision_fetch_sub_release(&self, val: u64) -> u64 {
        unsafe {
            let prev: u64;
            llvm_asm!("xrelease; lock; xaddq $2, $1"
                      : "=r" (prev), "+*m" (self)
                      : "0" (val.wrapping_neg())
                      : "memory"
                      : "volatile");
            prev
        }
    }
}