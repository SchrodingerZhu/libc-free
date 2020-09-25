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

fn elision_cas(target: &AtomicU64, current: u64, next: u64) -> u64 {
    unsafe {
        let prev: u64;
        llvm_asm!("xacquire; lock; cmpxchgq $2, $1"
                      : "={rax}" (prev), "+*m" (target)
                      : "r" (next), "{rax}" (current)
                      : "memory"
                      : "volatile");
        prev
    }
}

fn elision_fetch_sub(target: &AtomicU64, delta: u64) -> u64 {
    unsafe {
        let prev: u64;
        llvm_asm!("xrelease; lock; xaddq $2, $1"
                      : "=r" (prev), "+*m" (target)
                      : "0" (delta.wrapping_neg())
                      : "memory"
                      : "volatile");
        prev
    }
}

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
            && elision_cas(&self._flag, FREE, LOCKED) == FREE {
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
        if elision_fetch_sub(&self._flag, 1) == FUTEX_MODE {
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

const WRITE_LOCKED: u64 = 0;
const RW_OPEN: u64 = 1;

pub struct RwFutex<T> {
    pub(crate) _flag: AtomicU64,
    pub(crate) item: core::cell::UnsafeCell<T>,
}

pub struct RwFutexWriteHandle<'a, T> {
    _futex: &'a RwFutex<T>,
    item: &'a mut T,
}

pub struct RwFutexReadHandle<'a, T> {
    _futex: &'a RwFutex<T>,
    item: &'a T,
}

unsafe impl<T> Sync for RwFutex<T> {}

unsafe impl<T> Send for RwFutex<T> {}

impl<T> RwFutex<T> {
    fn new(item: T) -> Self {
        RwFutex {
            _flag: AtomicU64::new(RW_OPEN),
            item: UnsafeCell::new(item),
        }
    }
    #[inline(always)]
    fn raw_unlock(&self) {
        let mut wanted: u64;
        let mut current: u64;
        loop {
            current = self._flag.load(Ordering::Relaxed);
            if current == RW_OPEN {
                return;
            }
            if current == WRITE_LOCKED {
                wanted = RW_OPEN;
            } else {
                wanted = current - 1;
            }
            if self._flag.compare_exchange_weak(current, wanted, Ordering::Release, Ordering::Relaxed).is_ok() {
                break;
            }
            spin_loop_hint();
        }
        futex_wake_one(&self._flag);
    }

    #[inline(always)]
    fn raw_read_lock(&self) {
        let mut current: u64 = self._flag.load(Ordering::Relaxed);
        if current == RW_OPEN && elision_cas(&self._flag, current, current + 1) == current {
            return;
        }
        let mut counter = 0;
        loop {
            current = self._flag.load(Ordering::Relaxed);
            if current == WRITE_LOCKED || self._flag.compare_exchange_weak(current, current + 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
                if counter <= SPIN_LIMIT {
                    spin_loop_hint();
                    counter += 1;
                    continue;
                }
                while unsafe { syscall!(SYS_futex, &self._flag as *const AtomicU64, FUTEX_WAIT_PRIVATE, current, 0, 0, 0).is_err() } {
                    if self._flag.load(Ordering::Relaxed) >= RW_OPEN {
                        break;
                    }
                    spin_loop_hint();
                }
            } else {
                break;
            }
        }
    }

    #[inline(always)]
    fn raw_write_lock(&self) {
        if self._flag.load(Ordering::Relaxed) == RW_OPEN && elision_cas(&self._flag, RW_OPEN, WRITE_LOCKED) == RW_OPEN {
            return;
        }
        let mut counter = 0;
        loop {
            match self._flag.compare_exchange_weak(RW_OPEN, WRITE_LOCKED, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => break,
                Err(current) => {
                    if counter <= SPIN_LIMIT {
                        spin_loop_hint();
                        counter += 1;
                        continue;
                    }
                    while unsafe { syscall!(SYS_futex, &self._flag as *const AtomicU64, FUTEX_WAIT_PRIVATE, current, 0, 0, 0).is_err() } {
                        if self._flag.load(Ordering::Relaxed) == RW_OPEN { break; }
                        spin_loop_hint();
                    }
                    if self._flag.load(Ordering::Relaxed) != RW_OPEN {
                        futex_wake_one(&self._flag);
                        unsafe {
                            match syscall!(SYS_sched_yield) {
                                _ => ()
                            }
                        }
                    }
                }
            }
        }
    }

    fn read_lock(&self) -> RwFutexReadHandle<T> {
        self.raw_read_lock();
        RwFutexReadHandle {
            _futex: &self,
            item: unsafe { &*self.item.get() },
        }
    }

    fn write_lock(&self) -> RwFutexWriteHandle<T> {
        self.raw_write_lock();
        RwFutexWriteHandle {
            _futex: &self,
            item: unsafe { &mut *self.item.get() },
        }
    }
}

impl<'a, T> Drop for RwFutexReadHandle<'a, T> {
    fn drop(&mut self) {
        self._futex.raw_unlock();
    }
}

impl<'a, T> Drop for RwFutexWriteHandle<'a, T> {
    fn drop(&mut self) {
        self._futex.raw_unlock();
    }
}

impl<'a, T> Deref for RwFutexReadHandle<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        return self.item;
    }
}

impl<'a, T> Deref for RwFutexWriteHandle<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        return self.item;
    }
}


impl<'a, T> DerefMut for RwFutexWriteHandle<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return self.item;
    }
}

#[cfg(test)]
mod test {
    use ::test::Bencher;
    use std::sync::*;

    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[bench]
    fn test_futex(bencher: &mut Bencher) {
        bencher.iter(|| {
            let data = Arc::new(Futex::new(0));
            let mut handles = Vec::new();
            for _ in 0..100 {
                let data = data.clone();
                handles.push(std::thread::spawn(move || {
                    let mut handle = data.lock();
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

    #[bench]
    fn test_rwfutex_write(bencher: &mut Bencher) {
        bencher.iter(|| {
            let data = Arc::new(RwFutex::new(0));
            let mut handles = Vec::new();
            for _ in 0..100 {
                let data = data.clone();
                handles.push(std::thread::spawn(move || {
                    let mut handle = data.write_lock();
                    *handle += 1;
                }));
            }
            for i in handles {
                i.join();
            }
            {
                let handle = data.read_lock();
                assert_eq!(*handle, 100);
            }
        });
    }

    #[bench]
    fn test_rwfutex_read_write(bencher: &mut Bencher) {
        bencher.iter(|| {
            let data = Arc::new(RwFutex::new(0));
            let mut handles = Vec::new();
            for _ in 0..100 {
                let data = data.clone();
                handles.push(std::thread::spawn(move || {
                    data.read_lock();
                    {
                        let mut handle = data.write_lock();
                        *handle += 1;
                    }
                    data.read_lock();
                }));
            }
            for i in handles {
                i.join();
            }
            {
                let handle = data.read_lock();
                assert_eq!(*handle, 100);
            }
        });
    }

    #[bench]
    fn test_rwlock_read_write(bencher: &mut Bencher) {
        bencher.iter(|| {
            let data = Arc::new(RwLock::new(0));
            let mut handles = Vec::new();
            for _ in 0..100 {
                let data = data.clone();
                handles.push(std::thread::spawn(move || {
                    data.read().unwrap();
                    {
                        let mut handle = data.write().unwrap();
                        *handle += 1;
                    }
                    data.read().unwrap();
                }));
            }
            for i in handles {
                i.join();
            }
            {
                let handle = data.read().unwrap();
                assert_eq!(*handle, 100);
            }
        });
    }
}

