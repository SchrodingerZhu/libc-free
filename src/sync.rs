#![allow(unused)]

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::*;

use syscalls::*;


pub const FUTEX_WAIT: u64 = 0;
pub const FUTEX_WAKE: u64 = 1;
pub const FUTEX_FD: u64 = 2;
pub const FUTEX_REQUEUE: u64 = 3;
pub const FUTEX_CMP_REQUEUE: u64 = 4;
pub const FUTEX_WAKE_OP: u64 = 5;
pub const FUTEX_LOCK_PI: u64 = 6;
pub const FUTEX_UNLOCK_PI: u64 = 7;
pub const FUTEX_TRYLOCK_PI: u64 = 8;
pub const FUTEX_WAIT_BITSET: u64 = 9;
pub const FUTEX_WAKE_BITSET: u64 = 10;
pub const FUTEX_WAIT_REQUEUE_PI: u64 = 11;
pub const FUTEX_CMP_REQUEUE_PI: u64 = 12;
pub const FUTEX_PRIVATE_FLAG: u64 = 128;
pub const FUTEX_CLOCK_REALTIME: u64 = 256;
pub const FUTEX_CMD_MASK: u64 = !(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME);
pub const FUTEX_WAIT_PRIVATE: u64 = (FUTEX_WAIT | FUTEX_PRIVATE_FLAG);
pub const FUTEX_WAKE_PRIVATE: u64 = (FUTEX_WAKE | FUTEX_PRIVATE_FLAG);
pub const FUTEX_REQUEUE_PRIVATE: u64 = (FUTEX_REQUEUE | FUTEX_PRIVATE_FLAG);
pub const FUTEX_CMP_REQUEUE_PRIVATE: u64 = (FUTEX_CMP_REQUEUE | FUTEX_PRIVATE_FLAG);
pub const FUTEX_WAKE_OP_PRIVATE: u64 = (FUTEX_WAKE_OP | FUTEX_PRIVATE_FLAG);
pub const FUTEX_LOCK_PI_PRIVATE: u64 = (FUTEX_LOCK_PI | FUTEX_PRIVATE_FLAG);
pub const FUTEX_UNLOCK_PI_PRIVATE: u64 = (FUTEX_UNLOCK_PI | FUTEX_PRIVATE_FLAG);
pub const FUTEX_TRYLOCK_PI_PRIVATE: u64 = (FUTEX_TRYLOCK_PI | FUTEX_PRIVATE_FLAG);
pub const FUTEX_WAIT_BITSET_PRIVATE: u64 = (FUTEX_WAIT_BITSET | FUTEX_PRIVATE_FLAG);
pub const FUTEX_WAKE_BITSET_PRIVATE: u64 = (FUTEX_WAKE_BITSET | FUTEX_PRIVATE_FLAG);

const FREE: u64 = 0;
const LOCKED: u64 = 1;
const FUTEX_MODE: u64 = 2;
const SPIN_LIMIT: usize = 200;
const ONE_RESIDENT: u64 = 1;

#[inline(always)]
pub fn futex_wait(target: &AtomicU64, target_value: u64) {
    unsafe {
        match syscall!(SYS_futex, target as *const AtomicU64, FUTEX_WAIT_PRIVATE, target_value, 0, 0, 0) {
            _ => ()
        }
    }
}

#[inline(always)]
pub fn futex_wake_one(target: &AtomicU64) {
    unsafe {
        syscall!(SYS_futex, target as *const AtomicU64, FUTEX_WAKE_PRIVATE, 1, 0, 0, 0).unwrap();
    }
}


pub struct Futex<T> {
    pub(crate) _flag: AtomicU64,
    pub(crate) item: core::cell::UnsafeCell<T>,
}

pub struct FutexHandle<'a, T> {
    _futex: &'a Futex<T>,
    item: &'a mut T,
}

unsafe impl<T> Sync for Futex<T> {}

unsafe impl<T> Send for Futex<T> {}

impl<T> Futex<T> {
    pub fn new(item: T) -> Self {
        Futex {
            _flag: AtomicU64::new(FREE),
            item: UnsafeCell::new(item),
        }
    }

    #[inline(always)]
    fn raw_lock(&self) {
        // try elision lock
        if self._flag.load(Ordering::Relaxed) == FREE
            && FREE == unsafe {
            let prev: u64;
            llvm_asm!("xacquire; lock; cmpxchgq $2, $1"
                      : "={rax}" (prev), "+*m" (&self._flag)
                      : "r" (LOCKED), "{rax}" (FREE)
                      : "memory"
                      : "volatile");
            prev
        } {
            return;
        }
        for i in 0..SPIN_LIMIT {
            if self._flag.compare_exchange_weak(FREE, LOCKED, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                return;
            }
            spin_loop_hint(); // CPU relaxation
            // enter slow path spin lock
            if i > SPIN_LIMIT - 20 {
                unsafe {
                    syscall!(SYS_sched_yield).unwrap();
                }
            }
        }
        // enter futex path
        loop {
            if self._flag.load(Ordering::Relaxed) == FUTEX_MODE
                || self._flag.compare_exchange_weak(LOCKED, FUTEX_MODE, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                futex_wait(&self._flag, FUTEX_MODE);
            }
            if self._flag.compare_exchange_weak(FREE, FUTEX_MODE, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                break;
            }
        }
    }

    #[inline(always)]
    fn raw_unlock(&self) {
        let previous = unsafe {
            let prev: u64;
            llvm_asm!("xrelease; lock; xaddq $2, $1"
                      : "=r" (prev), "+*m" (&self._flag)
                      : "0" (1_u64.wrapping_neg())
                      : "memory"
                      : "volatile");
            prev
        };
        if previous == FUTEX_MODE {
            self._flag.store(FREE, Ordering::Relaxed);
            futex_wake_one(&self._flag);
        }
    }

    pub fn lock(&self) -> FutexHandle<T> {
        self.raw_lock();
        unsafe {
            FutexHandle {
                _futex: &self,
                item: &mut *self.item.get(),
            }
        }
    }
}

impl<'a, T> Drop for FutexHandle<'a, T> {
    fn drop(&mut self) {
        self._futex.raw_unlock();
    }
}

impl<'a, T> Deref for FutexHandle<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        return self.item;
    }
}

impl<'a, T> DerefMut for FutexHandle<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return self.item;
    }
}

#[cfg(test)]
mod test {
    use ::test::Bencher;
    use std::sync::*;

    use super::*;

    #[bench]
    fn test_futex(bencher: &mut Bencher) {
        bencher.iter(|| {
            let data = Arc::new(Futex::new(0));
            let mut handles = Vec::new();
            for _ in 0..100 {
                let data = data.clone();
                handles.push(std::thread::spawn(move || {
                    let mut handle = data.lock();
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    *handle += 1;
                }));
            }
            for i in handles {
                i.join();
            }
            {
                let handle = data.lock();
                assert_eq!(*handle, 100);
            }
        });
    }

    #[bench]
    fn test_mutex(bencher: &mut Bencher) {
        bencher.iter(|| {
            let data = Arc::new(Mutex::new(0));
            let mut handles = Vec::new();
            for _ in 0..100 {
                let data = data.clone();
                handles.push(std::thread::spawn(move || {
                    let mut handle = data.lock().unwrap();
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    *handle += 1;
                }));
            }
            for i in handles {
                i.join();
            }
            {
                let handle = data.lock().unwrap();
                assert_eq!(*handle, 100);
            }
        });
    }
}