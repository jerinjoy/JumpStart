/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use core::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn islower(c: c_int) -> c_int {
    if (c >= b'a' as c_int) && (c <= b'z' as c_int) {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn isupper(c: c_int) -> c_int {
    if (c >= b'A' as c_int) && (c <= b'Z' as c_int) {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn tolower(c: c_int) -> c_int {
    if isupper(c) != 0 {
        c + (b'a' as c_int - b'A' as c_int)
    } else {
        c
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn toupper(c: c_int) -> c_int {
    if islower(c) != 0 {
        c - (b'a' as c_int - b'A' as c_int)
    } else {
        c
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    if dest.is_null() || src.is_null() {
        return core::ptr::null_mut();
    }

    let mut d = dest;
    let mut s = src;
    while unsafe { *s } != 0 {
        unsafe {
            *d = *s;
            d = d.add(1);
            s = s.add(1);
        }
    }
    unsafe {
        *d = 0;
    }
    dest
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int {
    if s1.is_null() || s2.is_null() {
        return -1;
    }

    let mut p1 = s1 as *const u8;
    let mut p2 = s2 as *const u8;

    while unsafe { *p1 != 0 && *p1 == *p2 } {
        p1 = unsafe { p1.add(1) };
        p2 = unsafe { p2.add(1) };
    }

    unsafe { (*p1 as c_int) - (*p2 as c_int) }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn strlen(s: *const c_char) -> usize {
    if s.is_null() {
        return 0;
    }
    let mut len = 0;
    let mut p = s;
    while unsafe { *p } != 0 {
        len += 1;
        p = unsafe { p.add(1) };
    }
    len
}
