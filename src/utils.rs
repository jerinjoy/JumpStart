/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::cpu_bits;

// FFI struct to match `struct bit_range` in C
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BitRange {
    pub msb: u8,
    pub lsb: u8,
}

const RAND_MAX: u64 = 0x7fffffff;

// ═══════════════════════════════════════════════════════════════════════════════
// Per-mode seed statics (placed in privileged data)
// ═══════════════════════════════════════════════════════════════════════════════

#[unsafe(link_section = ".jumpstart.cpu.data.privileged")]
static SMODE_NEXT_SEED: AtomicU64 = AtomicU64::new(1);

#[unsafe(link_section = ".jumpstart.cpu.data.privileged")]
static MMODE_NEXT_SEED: AtomicU64 = AtomicU64::new(1);

// ═══════════════════════════════════════════════════════════════════════════════
// Safe Rust core — no `extern "C"`, no `no_mangle`, no `link_section`.
// ═══════════════════════════════════════════════════════════════════════════════

#[inline]
fn extract_bits_inner(value: u64, range: BitRange) -> u64 {
    let msb = range.msb;
    let lsb = range.lsb;
    (value >> lsb) & ((1u64 << (msb - lsb + 1)) - 1)
}

#[inline]
fn place_bits_inner(value: u64, bits: u64, range: BitRange) -> u64 {
    let msb = range.msb;
    let lsb = range.lsb;
    (value & !(((1u64 << (msb - lsb + 1)) - 1) << lsb)) | (bits << lsb)
}

/// Shared LCG-based pseudo-random number generator.
///
/// Uses a lock-free compare-exchange loop to advance the seed atomically.
#[inline]
fn random_next(seed: &AtomicU64) -> u64 {
    loop {
        let current = seed.load(Ordering::SeqCst);
        let val = current.wrapping_mul(6364136223846793005).wrapping_add(1);
        let ret = ((val >> 32) & RAND_MAX) as i64;

        if seed
            .compare_exchange_weak(current, val, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return ret as u64;
        }
    }
}

#[inline]
fn set_seed(seed: &AtomicU64, value: i32) {
    seed.store(value as u64, Ordering::SeqCst);
}

/// # Safety
///
/// `fail_fn` must be a valid function pointer that never returns.
#[inline]
unsafe fn try_get_seed(fail_fn: unsafe extern "C" fn() -> !) -> i32 {
    let mut seed: u32 = 0;

    for _ in 0..100 {
        // SAFETY: reading the seed CSR is always safe on RISC-V
        unsafe {
            asm!(
                "csrrw {0}, seed, x0",
                out(reg) seed,
                options(nostack, nomem),
            );
        }

        let opst = cpu_bits::get_field(seed as u64, cpu_bits::SEED_OPST);
        if opst == cpu_bits::SEED_OPST_ES16 {
            return cpu_bits::get_field(seed as u64, cpu_bits::SEED_ENTROPY_MASK) as i32;
        } else if opst == cpu_bits::SEED_OPST_WAIT || opst == cpu_bits::SEED_OPST_BIST {
            continue;
        } else {
            // SAFETY: caller provides the correct fail function for the current mode
            unsafe {
                fail_fn();
            }
        }
    }

    cpu_bits::get_field(seed as u64, cpu_bits::SEED_ENTROPY_MASK) as i32
}

// ═══════════════════════════════════════════════════════════════════════════════
// FFI shims — one-liners that carry ONLY the C ABI baggage.
// ═══════════════════════════════════════════════════════════════════════════════

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn extract_bits(value: u64, range: BitRange) -> u64 {
    extract_bits_inner(value, range)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn place_bits(value: u64, bits: u64, range: BitRange) -> u64 {
    place_bits_inner(value, bits, range)
}

// --- S-Mode random ---

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn smode_try_get_seed() -> i32 {
    // SAFETY: jumpstart_smode_fail is the correct fail handler for S-mode
    unsafe { try_get_seed(jumpstart_smode_fail) }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn __smode_random() -> u64 {
    random_next(&SMODE_NEXT_SEED)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn get_random_number_from_smode() -> i32 {
    random_next(&SMODE_NEXT_SEED) as i32
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn set_random_seed_from_smode(seed: i32) {
    set_seed(&SMODE_NEXT_SEED, seed);
}

// --- M-Mode random ---

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
pub extern "C" fn mmode_try_get_seed() -> i32 {
    // SAFETY: jumpstart_mmode_fail is the correct fail handler for M-mode
    unsafe { try_get_seed(jumpstart_mmode_fail) }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
pub extern "C" fn __mmode_random() -> u64 {
    random_next(&MMODE_NEXT_SEED)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
pub extern "C" fn get_random_number_from_mmode() -> i32 {
    random_next(&MMODE_NEXT_SEED) as i32
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
pub extern "C" fn set_random_seed_from_mmode(seed: i32) {
    set_seed(&MMODE_NEXT_SEED, seed);
}

// ----------------------------------------------------------------------------
// External C functions
// ----------------------------------------------------------------------------

unsafe extern "C" {
    fn jumpstart_smode_fail() -> !;
    fn jumpstart_mmode_fail() -> !;
}
