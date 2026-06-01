/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use core::ffi::{c_char, c_int};
use core::fmt::{Error, Write};
use core::sync::atomic::{AtomicU8, Ordering};

use crate::generated::UART_BASE_ADDRESS;

// ═══════════════════════════════════════════════════════════════════════════════
// Uart struct — already clean Rust, no changes needed.
// ═══════════════════════════════════════════════════════════════════════════════

pub struct Uart {
    base_address: usize,
}

impl Uart {
    pub const fn new() -> Self {
        Uart {
            base_address: UART_BASE_ADDRESS,
        }
    }

    pub fn putc(&mut self, c: u8) {
        let ptr = self.base_address as *mut u8;
        // SAFETY: write_volatile is required for MMIO — the compiler must not
        // optimize away the write even though the value is never read back.
        unsafe {
            ptr.write_volatile(c);
        }
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        for byte in s.bytes() {
            self.putc(byte);
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// UART initialization state
// ═══════════════════════════════════════════════════════════════════════════════

#[unsafe(link_section = ".jumpstart.cpu.data.privileged")]
static UART_INITIALIZED: AtomicU8 = AtomicU8::new(0);

// ═══════════════════════════════════════════════════════════════════════════════
// Safe Rust core — no `extern "C"`, no `no_mangle`, no `link_section`.
// ═══════════════════════════════════════════════════════════════════════════════

#[inline]
fn mark_uart_enabled() {
    UART_INITIALIZED.store(1, Ordering::SeqCst);
}

#[inline]
fn init_uart() {
    mark_uart_enabled();
}

#[inline]
fn uart_enabled() -> bool {
    UART_INITIALIZED.load(Ordering::SeqCst) != 0
}

/// # Safety
///
/// `UART_BASE_ADDRESS` must point to valid MMIO.
#[inline]
unsafe fn put_char(c: u8) -> i32 {
    let ptr = UART_BASE_ADDRESS as *mut u8;
    // SAFETY: UART_BASE_ADDRESS is guaranteed to point to valid MMIO
    unsafe {
        ptr.write_volatile(c);
    }
    c as i32
}

/// # Safety
///
/// `s` must be a valid pointer to a null-terminated C string.
#[inline]
unsafe fn put_str(s: *const c_char) -> c_int {
    if !uart_enabled() {
        return 0;
    }
    if s.is_null() {
        return -1;
    }
    let mut p = s as *const u8;
    // SAFETY: caller guarantees s is a valid, non-null C string pointer
    while unsafe { *p } != 0 {
        crate::print!("{}", unsafe { *p } as char);
        p = unsafe { p.add(1) };
    }
    crate::println!();
    0
}

/// # Safety
///
/// `fmt` must be a valid pointer to a null-terminated format string.
/// The variadic arguments `args` are interpreted according to `fmt`.
#[inline]
unsafe fn format_printk(fmt: *const c_char, args: &[usize; 7]) -> c_int {
    if !uart_enabled() {
        return 0;
    }
    if fmt.is_null() {
        return -1;
    }

    let mut count = 0;
    let mut arg_idx = 0;
    let mut p = fmt as *const u8;

    // SAFETY: caller guarantees fmt is a valid, non-null C string pointer
    while unsafe { *p } != 0 {
        let ch = unsafe { *p };
        if ch == b'%' {
            p = unsafe { p.add(1) };
            match unsafe { *p } {
                b's' => {
                    let s = if arg_idx < args.len() {
                        args[arg_idx] as *const c_char
                    } else {
                        core::ptr::null()
                    };
                    arg_idx += 1;
                    if !s.is_null() {
                        let mut sp = s as *const u8;
                        while unsafe { *sp } != 0 {
                            crate::print!("{}", unsafe { *sp } as char);
                            sp = unsafe { sp.add(1) };
                            count += 1;
                        }
                    } else {
                        crate::print!("(null)");
                        count += 6;
                    }
                }
                b'd' | b'i' => {
                    let val = if arg_idx < args.len() {
                        args[arg_idx] as i32
                    } else {
                        0
                    };
                    arg_idx += 1;
                    crate::print!("{}", val);
                    count += 1;
                }
                b'x' => {
                    let val = if arg_idx < args.len() {
                        args[arg_idx] as u32
                    } else {
                        0
                    };
                    arg_idx += 1;
                    crate::print!("{:x}", val);
                    count += 1;
                }
                b'%' => {
                    crate::print!("%");
                    count += 1;
                }
                _ => {
                    crate::print!("%{}", unsafe { *p } as char);
                    count += 2;
                }
            }
        } else {
            crate::print!("{}", ch as char);
            count += 1;
        }
        p = unsafe { p.add(1) };
    }
    count
}

// ═══════════════════════════════════════════════════════════════════════════════
// FFI shims — one-liners that carry ONLY the C ABI baggage.
// ═══════════════════════════════════════════════════════════════════════════════

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn setup_uart() {
    init_uart();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn mark_uart_as_enabled() {
    mark_uart_enabled();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn is_uart_enabled() -> c_int {
    if uart_enabled() { 1 } else { 0 }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn putch(c: u8) -> i32 {
    // SAFETY: UART_BASE_ADDRESS is guaranteed to point to valid MMIO
    unsafe { put_char(c) }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn puts(s: *const c_char) -> c_int {
    // SAFETY: the C caller is responsible for passing a valid string pointer
    unsafe { put_str(s) }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn printk(
    fmt: *const c_char,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
) -> c_int {
    let args = [a1, a2, a3, a4, a5, a6, a7];
    // SAFETY: the C caller is responsible for passing a valid format string
    unsafe { format_printk(fmt, &args) }
}
