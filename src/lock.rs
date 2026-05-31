/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// A simple Spinlock.
/// We use `AtomicBool` to track whether the lock is held.
///
/// # Layout
/// `#[repr(C)]` ensures a deterministic field order: `locked` at offset 0,
/// `data` at offset 8 (after alignment padding).  Without this annotation
/// the compiler is free to reorder the fields, which could cause the
/// inlined lock/unlock sequences to disagree on the offset of the lock byte.
#[repr(C)]
pub struct Spinlock<T> {
    locked: AtomicBool,
    // UnsafeCell is Rust's way of saying "I might mutate this data even if
    // I only have an immutable reference to the struct." It's required for locks!
    data: UnsafeCell<T>,
}

// We must explicitly tell Rust it's safe to share this lock across CPU cores (threads).
unsafe impl<T> Sync for Spinlock<T> {}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquires the lock. Spins until the lock is free.
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        // This maps exactly to your while loop with amoswap.d.aq!
        // `swap` atomically sets the value to `true` and returns the old value.
        // If the old value was `true`, someone else has it, so we keep spinning.
        while self.locked.swap(true, Ordering::Acquire) {
            // A hint to the CPU that we are in a spin-loop (maps to `pause` or `nop`)
            core::hint::spin_loop();
        }

        // We got the lock! Return a guard that gives access to the data.
        SpinlockGuard { lock: self }
    }
}

/// A "Guard" that represents the acquired lock.
/// When this guard goes out of scope, Rust automatically drops it and unlocks!
pub struct SpinlockGuard<'a, T> {
    lock: &'a Spinlock<T>,
}

impl<T> core::ops::Deref for SpinlockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // Safe because we hold the lock!
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> core::ops::DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safe because we hold the lock!
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinlockGuard<'_, T> {
    fn drop(&mut self) {
        // This is called automatically when the guard goes out of scope!
        // It maps exactly to your amoswap.d.rl releasing the lock.
        self.lock.locked.store(false, Ordering::Release);
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn acquire_lock(lock: *mut u64) {
    let atomic_lock = unsafe { &*(lock as *const AtomicU64) };

    // Standard spinlock acquisition
    while atomic_lock.swap(1, Ordering::Acquire) != 0 {
        while atomic_lock.load(Ordering::Relaxed) != 0 {
            core::hint::spin_loop();
        }
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn release_lock(lock: *mut u64) {
    let atomic_lock = unsafe { &*(lock as *const AtomicU64) };
    atomic_lock.store(0, Ordering::Release);
}
