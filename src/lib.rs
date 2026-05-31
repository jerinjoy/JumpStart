/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#![no_std]

pub mod cpu_bits;
pub mod heap;
pub mod lock;
pub mod string;
pub mod tablewalk;
pub mod thread_attr_fns;
pub mod time;
pub mod trap;
pub mod uart;
pub mod utils;

extern crate alloc;

use crate::lock::Spinlock;
use crate::uart::Uart;
use core::panic::PanicInfo; // <--- Import the lock

// Create a globally accessible, thread-safe UART protected by a Spinlock!
#[unsafe(link_section = ".jumpstart.cpu.data.privileged")]
static UART: Spinlock<Uart> = Spinlock::new(Uart::new());

// 1. The helper function that actually does the printing
#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    // Lock the UART, write the formatted arguments, and then drop the lock.
    //
    // SAFETY: we use `let _ = ...` instead of `.unwrap()` so that a
    // formatting error cannot leak the lock.  With `panic = "abort"` the
    // lock-guard's `Drop` is never executed if `.unwrap()` panics, which
    // causes the panic handler itself to deadlock when it tries to print.
    let _ = UART.lock().write_fmt(args);
}

// 2. Define the `print!` macro
#[macro_export]
macro_rules! print {
    // This looks like gibberish, but it basically says:
    // "Take any arguments, format them, and pass them to _print"
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}
// 3. Define the `println!` macro (same as print!, but adds a newline)
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! read_csr {
    ($csr:ident) => {
        {
            let value: u64;
            unsafe {
                core::arch::asm!(
                    concat!("csrr {0}, ", stringify!($csr)),
                    out(reg) value,
                    options(nomem, nostack)
                );
            }
            value
        }
    };
}

/// Direct unbuffered UART output that bypasses the Spinlock.
///
/// SAFETY: this is only for emergency use (panic handler, early boot)
/// where the lock might be held or corrupted.  It writes straight to
/// the UART register without any synchronisation.
#[doc(hidden)]
pub fn uart_write_str_direct(s: &str) {
    let ptr = crate::uart::UART_BASE_ADDRESS as *mut u8;
    for byte in s.bytes() {
        unsafe {
            ptr.write_volatile(byte);
        }
    }
}

#[doc(hidden)]
pub fn uart_write_fmt_direct(args: core::fmt::Arguments) {
    use core::fmt::Write;
    struct DirectWriter;
    impl Write for DirectWriter {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            uart_write_str_direct(s);
            Ok(())
        }
    }
    let _ = DirectWriter.write_fmt(args);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Use direct UART writes so we never attempt to acquire the lock.
    // If the lock was already held when the panic occurred, calling
    // `println!` (which uses the lock) would deadlock.
    uart_write_str_direct("KERNEL PANIC!\n");
    uart_write_fmt_direct(format_args!("{}\n", info));

    loop {}
}

unsafe extern "C" {
    pub fn _mmode_start();
}
