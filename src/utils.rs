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

// ----------------------------------------------------------------------------
// Utilities
// ----------------------------------------------------------------------------

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn extract_bits(value: u64, range: BitRange) -> u64 {
    let msb = range.msb;
    let lsb = range.lsb;
    (value >> lsb) & ((1u64 << (msb - lsb + 1)) - 1)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn place_bits(value: u64, bits: u64, range: BitRange) -> u64 {
    let msb = range.msb;
    let lsb = range.lsb;
    (value & !(((1u64 << (msb - lsb + 1)) - 1) << lsb)) | (bits << lsb)
}

// ----------------------------------------------------------------------------
// Entropy / Random Number Generation (S-Mode)
// ----------------------------------------------------------------------------

#[unsafe(link_section = ".jumpstart.cpu.data.privileged")]
static SMODE_NEXT_SEED: AtomicU64 = AtomicU64::new(1);

const RAND_MAX: u64 = 0x7fffffff;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn smode_try_get_seed() -> i32 {
    let mut seed: u32 = 0;

    for _ in 0..100 {
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
            unsafe {
                jumpstart_smode_fail();
            }
        }
    }

    cpu_bits::get_field(seed as u64, cpu_bits::SEED_ENTROPY_MASK) as i32
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn __smode_random() -> u64 {
    let mut val: u64;
    let mut ret: i64;

    // Lock-free loop imitating LR/SC for the LCG
    loop {
        let current = SMODE_NEXT_SEED.load(Ordering::SeqCst);
        val = current.wrapping_mul(6364136223846793005).wrapping_add(1);
        ret = ((val >> 32) & RAND_MAX) as i64;

        if SMODE_NEXT_SEED
            .compare_exchange_weak(current, val, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            break;
        }
    }

    ret as u64
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn get_random_number_from_smode() -> i32 {
    __smode_random() as i32
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn set_random_seed_from_smode(seed: i32) {
    SMODE_NEXT_SEED.store(seed as u64, Ordering::SeqCst);
}

// ----------------------------------------------------------------------------
// Entropy / Random Number Generation (M-Mode)
// ----------------------------------------------------------------------------

#[unsafe(link_section = ".jumpstart.cpu.data.privileged")]
static MMODE_NEXT_SEED: AtomicU64 = AtomicU64::new(1);

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
pub extern "C" fn mmode_try_get_seed() -> i32 {
    let mut seed: u32 = 0;

    for _ in 0..100 {
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
            unsafe {
                jumpstart_mmode_fail();
            }
        }
    }

    cpu_bits::get_field(seed as u64, cpu_bits::SEED_ENTROPY_MASK) as i32
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
pub extern "C" fn __mmode_random() -> u64 {
    let mut val: u64;
    let mut ret: i64;

    // Lock-free loop imitating LR/SC for the LCG
    loop {
        let current = MMODE_NEXT_SEED.load(Ordering::SeqCst);
        val = current.wrapping_mul(6364136223846793005).wrapping_add(1);
        ret = ((val >> 32) & RAND_MAX) as i64;

        if MMODE_NEXT_SEED
            .compare_exchange_weak(current, val, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            break;
        }
    }

    ret as u64
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
pub extern "C" fn get_random_number_from_mmode() -> i32 {
    __mmode_random() as i32
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
pub extern "C" fn set_random_seed_from_mmode(seed: i32) {
    MMODE_NEXT_SEED.store(seed as u64, Ordering::SeqCst);
}

// ----------------------------------------------------------------------------
// External C functions
// ----------------------------------------------------------------------------

unsafe extern "C" {
    fn jumpstart_smode_fail() -> !;
    fn jumpstart_mmode_fail() -> !;
}
