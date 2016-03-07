
/// Why did we shirk of the spin crate in crates.io
/// 1. This can modified to suit my needs
/// 2. I need to be able to force unlock during a kernel panic caused by an exception
///    That code never returns so it doesn't matter that we stole the lock.
/// 3. If coroutines are ever implemented we need to put a yeild call in between locks.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};
use core::ops::{Deref, DerefMut};

pub struct Mutex<T> {
    lock: AtomicBool,
    value: UnsafeCell<T>,
}

unsafe impl <T: Sized + Send> Send for Mutex<T> {}
unsafe impl <T: Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T>
{
    pub const fn new(value: T) -> Mutex<T> {
        Mutex { lock: AtomicBool::new(false),
                value: UnsafeCell::new(value) }
    }

    /// Try the lock once and return it on success.
    pub fn try_lock<'a>(&'a self) -> Option<LockGuard<'a, T>>
    {
        match self.lock.swap(true, Ordering::SeqCst) {
            // If the lock previously contained false (ie. Was unlocked)
            false => Some(LockGuard { locked: &self }),
            // Someone else had set the lock to true
            true => None
        }
    }

    /// Constantly try to unlock mutex until success.
    pub fn lock<'a>(&'a self) -> LockGuard<'a, T>
    {
        loop {
            if let Some(lock) = self.try_lock() {
                return lock;
            }
        }
    }

    /// Try a given number of times to obtain the lock.
    pub fn try_lock_times<'a>(&'a self, times: usize) -> Option<LockGuard<'a, T>>
    {
        for _ in 0..times {
            if let Some(lock) = self.try_lock() {
                return Some(lock)
            }
        }
        None
    }

    /// Succesfully unlock. Possible breaking any sort of concurrency
    /// guarentees this is supposed to provide. This is for
    /// unrecoverably situations that need to gain access to objects
    /// in order to print debug info and what not.
    pub unsafe fn force_lock<'a>(&'a self) -> LockGuard<'a, T>
    {
        self.lock.store(true, Ordering::SeqCst);
        LockGuard { locked: &self }
    }
}

pub struct LockGuard<'a, T: 'a> {
    locked: &'a Mutex<T>
}

impl<'a, T> Deref for LockGuard<'a, T>
{
    type Target = T;
    fn deref(&self) -> &T
    {
        unsafe { &*self.locked.value.get() }
    }
}

impl<'a, T> DerefMut for LockGuard<'a, T>
{
    fn deref_mut(&mut self) -> &mut T
    {
        unsafe { &mut *self.locked.value.get()  }
    }
}

impl<'a, T> Drop for LockGuard<'a, T>
{
    fn drop(&mut self)
    {
        self.locked.lock.store(false, Ordering::SeqCst)
    }
}
