/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use core::arch::asm;
use core::ffi::c_void;

// The frequency of the CPU clock in MHz, matching jumpstart_public_source_attributes.yaml
pub const CPU_CLOCK_FREQUENCY_IN_MHZ: u64 = 1;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[allow(non_camel_case_types)]
pub type time_t = i64;

// ═══════════════════════════════════════════════════════════════════════════════
// Safe Rust core — no `extern "C"`, no `no_mangle`, no `link_section`.
// ═══════════════════════════════════════════════════════════════════════════════

#[inline]
fn read_time_internal() -> u64 {
    let time_val: u64;
    // SAFETY: rdtime is a RISC-V instruction that always returns a valid value
    unsafe {
        asm!("rdtime {0}", out(reg) time_val, options(nomem, nostack));
    }
    time_val
}

#[inline]
fn delay_us(delay_in_useconds: u32) {
    let start_time = read_time_internal();
    let iter_count: u32 = 10;

    for _ in 0..iter_count {
        // SAFETY: pause is a RISC-V hint instruction; always safe to execute
        unsafe {
            asm!("pause", options(nomem, nostack));
        }
    }

    let end_time = read_time_internal();
    // Use max(..., 1) to prevent division by zero in case the timer is slow
    let avg_lat = core::cmp::max((end_time - start_time) / (iter_count as u64), 1);

    if ((delay_in_useconds as u64) / avg_lat) <= (iter_count as u64) {
        // Delay already completed, no additional iterations needed
    } else {
        let latency_iter_count = ((delay_in_useconds as u64) / avg_lat) - (iter_count as u64);
        for _ in 0..latency_iter_count {
            unsafe {
                asm!("pause", options(nomem, nostack));
            }
        }
    }
}

/// # Safety
///
/// `tv` must be null or a valid pointer to a `timeval` struct.
#[inline]
unsafe fn get_time_of_day(tv: *mut timeval) -> i32 {
    let timer_ticks = read_time_internal();

    let seconds = timer_ticks / (CPU_CLOCK_FREQUENCY_IN_MHZ * 1_000_000);
    let microseconds = (timer_ticks / CPU_CLOCK_FREQUENCY_IN_MHZ) % 1_000_000;

    if !tv.is_null() {
        // SAFETY: caller guarantees tv is a valid pointer when non-null
        unsafe {
            (*tv).tv_sec = seconds as i64;
            (*tv).tv_usec = microseconds as i64;
        }
    }

    0
}

/// # Safety
///
/// `tloc` must be null or a valid pointer to a `time_t`.
#[inline]
unsafe fn get_time(tloc: *mut time_t) -> time_t {
    let mut tv = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };

    // SAFETY: tv is a valid stack-allocated timeval
    if unsafe { get_time_of_day(&mut tv) } != 0 {
        return -1;
    }

    let current_time = tv.tv_sec;

    if !tloc.is_null() {
        // SAFETY: caller guarantees tloc is a valid pointer when non-null
        unsafe {
            *tloc = current_time;
        }
    }

    current_time
}

// ═══════════════════════════════════════════════════════════════════════════════
// FFI shims — one-liners that carry ONLY the C ABI baggage.
// ═══════════════════════════════════════════════════════════════════════════════

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn read_time() -> u64 {
    read_time_internal()
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn delay_us_from_smode(delay_in_useconds: u32) {
    delay_us(delay_in_useconds);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
pub extern "C" fn delay_us_from_mmode(delay_in_useconds: u32) {
    delay_us(delay_in_useconds);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn gettimeofday(tv: *mut timeval, _tz: *mut c_void) -> i32 {
    // SAFETY: the C caller is responsible for passing valid pointers
    unsafe { get_time_of_day(tv) }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn time(tloc: *mut time_t) -> time_t {
    // SAFETY: the C caller is responsible for passing a valid pointer
    unsafe { get_time(tloc) }
}
