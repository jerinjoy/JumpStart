/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use core::ffi::{c_char, c_int};

// ═══════════════════════════════════════════════════════════════════════════════
// Safe Rust core — no `extern "C"`, no `no_mangle`, no `link_section`.
// These functions contain the actual logic and can (in principle) be
// unit-tested on the host.
// ═══════════════════════════════════════════════════════════════════════════════

#[inline]
fn is_lower(c: c_int) -> c_int {
    if (c >= b'a' as c_int) && (c <= b'z' as c_int) {
        1
    } else {
        0
    }
}

#[inline]
fn is_upper(c: c_int) -> c_int {
    if (c >= b'A' as c_int) && (c <= b'Z' as c_int) {
        1
    } else {
        0
    }
}

#[inline]
fn to_lower(c: c_int) -> c_int {
    if is_upper(c) != 0 {
        c + (b'a' as c_int - b'A' as c_int)
    } else {
        c
    }
}

#[inline]
fn to_upper(c: c_int) -> c_int {
    if is_lower(c) != 0 {
        c - (b'a' as c_int - b'A' as c_int)
    } else {
        c
    }
}

/// # Safety
///
/// `dest` and `src` must be valid pointers to null-terminated C strings.
/// `dest` must point to a buffer large enough to hold the copied string.
#[inline]
unsafe fn str_copy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    if dest.is_null() || src.is_null() {
        return core::ptr::null_mut();
    }

    let mut d = dest;
    let mut s = src;
    // SAFETY: caller guarantees dest and src are valid, non-null pointers
    while unsafe { *s } != 0 {
        unsafe {
            *d = *s;
            d = d.add(1);
            s = s.add(1);
        }
    }
    // SAFETY: d points to a valid, writable location within the dest buffer
    unsafe { *d = 0 };
    dest
}

/// # Safety
///
/// `s1` and `s2` must be valid pointers to null-terminated C strings.
#[inline]
unsafe fn str_cmp(s1: *const c_char, s2: *const c_char) -> c_int {
    if s1.is_null() || s2.is_null() {
        return -1;
    }

    let mut p1 = s1;
    let mut p2 = s2;
    // SAFETY: caller guarantees s1 and s2 are valid, non-null pointers
    // to null-terminated C strings
    loop {
        let c1 = unsafe { *p1 };
        let c2 = unsafe { *p2 };
        if c1 != c2 || c1 == 0 {
            return (c1 as c_int) - (c2 as c_int);
        }
        p1 = unsafe { p1.add(1) };
        p2 = unsafe { p2.add(1) };
    }
}

/// # Safety
///
/// `s` must be a valid pointer to a null-terminated C string.
#[inline]
unsafe fn str_len(s: *const c_char) -> usize {
    if s.is_null() {
        return 0;
    }
    let mut p = s;
    let mut len: usize = 0;
    // SAFETY: caller guarantees s is a valid, non-null pointer to a
    // null-terminated C string; the loop stops at the NUL terminator
    while unsafe { *p } != 0 {
        len += 1;
        p = unsafe { p.add(1) };
    }
    len
}

// ═══════════════════════════════════════════════════════════════════════════════
// FFI shims — one-liners that carry ONLY the C ABI baggage.
// These are the functions C code actually calls.
// ═══════════════════════════════════════════════════════════════════════════════

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn islower(c: c_int) -> c_int {
    is_lower(c)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn isupper(c: c_int) -> c_int {
    is_upper(c)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn tolower(c: c_int) -> c_int {
    to_lower(c)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn toupper(c: c_int) -> c_int {
    to_upper(c)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    // SAFETY: the C caller is responsible for passing valid pointers
    unsafe { str_copy(dest, src) }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int {
    // SAFETY: the C caller is responsible for passing valid pointers
    unsafe { str_cmp(s1, s2) }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn strlen(s: *const c_char) -> usize {
    // SAFETY: the C caller is responsible for passing a valid pointer
    unsafe { str_len(s) }
}
