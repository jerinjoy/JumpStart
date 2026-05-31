/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use core::ffi::{c_char, c_int};
use core::fmt::{Error, Write};
use core::sync::atomic::{AtomicU8, Ordering};

// Pull in the generated constants from OUT_DIR.
// This gives us UART_BASE_ADDRESS and other defines as Rust constants.
include!(concat!(env!("OUT_DIR"), "/jumpstart_data_structures.rs"));

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
        // Pointer math in Rust is explicit. We cast the usize to a raw mutable pointer.
        let ptr = self.base_address as *mut u8;

        // We MUST use write_volatile here. If we used standard assignment (*ptr = c),
        // the compiler might optimize it away because it thinks "you wrote to this memory
        // but never read it back, so I'll just delete this code to save space!"
        // Volatile tells the compiler: "Hardware is watching this address, DO NOT optimize!"
        unsafe {
            ptr.write_volatile(c);
        }
    }
}

// By implementing core::fmt::Write, we unlock Rust's powerful `write!` and `writeln!`
// formatting macros for our custom hardware!
impl Write for Uart {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        for byte in s.bytes() {
            self.putc(byte);
        }
        Ok(())
    }
}

#[unsafe(link_section = ".jumpstart.cpu.data.privileged")]
static UART_INITIALIZED: AtomicU8 = AtomicU8::new(0);

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn setup_uart() {
    // The UART MMIO page is now mapped in S-stage page tables
    // when enable_uart is true, so it's safe to enable.
    mark_uart_as_enabled();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn mark_uart_as_enabled() {
    UART_INITIALIZED.store(1, Ordering::SeqCst);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn is_uart_enabled() -> c_int {
    UART_INITIALIZED.load(Ordering::SeqCst) as c_int
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn putch(c: u8) -> i32 {
    let ptr = UART_BASE_ADDRESS as *mut u8;
    unsafe {
        ptr.write_volatile(c);
    }
    c as i32
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn puts(s: *const c_char) -> c_int {
    if is_uart_enabled() == 0 {
        return 0;
    }
    if s.is_null() {
        return -1;
    }
    let mut p = s as *const u8;
    while unsafe { *p } != 0 {
        crate::print!("{}", unsafe { *p } as char);
        p = unsafe { p.add(1) };
    }
    crate::println!();
    0
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
    if is_uart_enabled() == 0 {
        return 0;
    }
    if fmt.is_null() {
        return -1;
    }

    let mut count = 0;
    let mut arg_idx = 0;
    let args = [a1, a2, a3, a4, a5, a6, a7];
    let mut p = fmt as *const u8;

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
