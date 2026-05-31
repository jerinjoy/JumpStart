/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use core::arch::asm;
use core::ffi::c_void;
use core::ptr;

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

#[inline(always)]
fn read_time_internal() -> u64 {
    let time_val: u64;
    unsafe {
        asm!("rdtime {0}", out(reg) time_val, options(nomem, nostack));
    }
    time_val
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn read_time() -> u64 {
    read_time_internal()
}

#[inline(always)]
fn delay_us(delay_in_useconds: u32) {
    let start_time = read_time_internal();
    let iter_count: u32 = 10;

    for _ in 0..iter_count {
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
    let timer_ticks = read_time_internal();

    let seconds = timer_ticks / (CPU_CLOCK_FREQUENCY_IN_MHZ * 1_000_000);
    let microseconds = timer_ticks / CPU_CLOCK_FREQUENCY_IN_MHZ;

    if !tv.is_null() {
        unsafe {
            (*tv).tv_sec = seconds as i64;
            (*tv).tv_usec = microseconds as i64;
        }
    }

    0
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn time(tloc: *mut time_t) -> time_t {
    let mut tv = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };

    if unsafe { gettimeofday(&mut tv, ptr::null_mut()) } != 0 {
        return -1;
    }

    let current_time = tv.tv_sec;

    if !tloc.is_null() {
        unsafe {
            *tloc = current_time;
        }
    }

    current_time
}
