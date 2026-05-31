/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

// 1. Declare the module-level items first to avoid name collisions with the included struct.
use crate::cpu_bits::{MCAUSE_EC_MASK, MCAUSE_INT_FLAG};
use crate::thread_attr_fns;

// 2. Pull in the generated data structures from OUT_DIR
// This gives us `trap_override_attributes`, `NUM_MMODE_EXCEPTION_HANDLER_OVERRIDES`, etc.
include!(concat!(env!("OUT_DIR"), "/jumpstart_data_structures.rs"));

// In RISC-V, the highest bit of mcause indicates an interrupt,
// and the lower bits indicate the exception code.

// 3. Export our Rust functions to C/Assembly
#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
#[unsafe(no_mangle)]
pub extern "C" fn register_mmode_trap_handler_override(mcause: u64, handler_address: u64) {
    // Calling an external C function requires an `unsafe` block
    let struct_addr =
        unsafe { thread_attr_fns::get_thread_attributes_trap_override_struct_address_from_mmode() };

    // Cast the raw `u64` address into a mutable raw pointer to our generated struct
    let trap_overrides = struct_addr as *mut trap_override_attributes;

    // Convert the exception code to a `usize` because Rust requires array indices to be `usize`
    let exception_code = (mcause & MCAUSE_EC_MASK) as usize;
    let is_interrupt = (mcause & MCAUSE_INT_FLAG) != 0;

    // Dereferencing a raw pointer requires an `unsafe` block because the compiler
    // cannot statically guarantee the memory address is valid.
    unsafe {
        if is_interrupt {
            if exception_code >= NUM_MMODE_INTERRUPT_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_mmode_fail();
            }
            // `(*ptr).field` is how we dereference and access fields of a raw pointer
            (*trap_overrides).mmode_interrupt_handler_overrides[exception_code] = handler_address;
        } else {
            if exception_code >= NUM_MMODE_EXCEPTION_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_mmode_fail();
            }
            (*trap_overrides).mmode_exception_handler_overrides[exception_code] = handler_address;
        }
    }
}

#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
#[unsafe(no_mangle)]
pub extern "C" fn deregister_mmode_trap_handler_override(mcause: u64) {
    let struct_addr =
        unsafe { thread_attr_fns::get_thread_attributes_trap_override_struct_address_from_mmode() };
    let trap_overrides = struct_addr as *mut trap_override_attributes;

    let exception_code = (mcause & MCAUSE_EC_MASK) as usize;
    let is_interrupt = (mcause & MCAUSE_INT_FLAG) != 0;

    unsafe {
        if is_interrupt {
            if exception_code >= NUM_MMODE_INTERRUPT_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_mmode_fail();
            }
            if (*trap_overrides).mmode_interrupt_handler_overrides[exception_code] == 0 {
                thread_attr_fns::jumpstart_mmode_fail();
            }
            (*trap_overrides).mmode_interrupt_handler_overrides[exception_code] = 0;
        } else {
            if exception_code >= NUM_MMODE_EXCEPTION_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_mmode_fail();
            }
            if (*trap_overrides).mmode_exception_handler_overrides[exception_code] == 0 {
                thread_attr_fns::jumpstart_mmode_fail();
            }
            (*trap_overrides).mmode_exception_handler_overrides[exception_code] = 0;
        }
    }
}

#[unsafe(link_section = ".jumpstart.cpu.text.mmode")]
#[unsafe(no_mangle)]
pub extern "C" fn get_mmode_trap_handler_override(mcause: u64) -> u64 {
    let struct_addr =
        unsafe { thread_attr_fns::get_thread_attributes_trap_override_struct_address_from_mmode() };
    // Notice this is `*const` instead of `*mut` because we are only reading
    let trap_overrides = struct_addr as *const trap_override_attributes;

    let exception_code = (mcause & MCAUSE_EC_MASK) as usize;
    let is_interrupt = (mcause & MCAUSE_INT_FLAG) != 0;

    unsafe {
        if is_interrupt {
            if exception_code >= NUM_MMODE_INTERRUPT_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_mmode_fail();
            }
            // Rust implicitly returns the last expression in a block (no `return` keyword needed)
            (*trap_overrides).mmode_interrupt_handler_overrides[exception_code]
        } else {
            if exception_code >= NUM_MMODE_EXCEPTION_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_mmode_fail();
            }
            (*trap_overrides).mmode_exception_handler_overrides[exception_code]
        }
    }
}

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
#[unsafe(no_mangle)]
fn get_exception_name(exception_id: u64) -> &'static str {
    use crate::cpu_bits::*; // Bring in the constants for this function

    match exception_id {
        RISCV_EXCP_INST_ADDR_MIS => "Instruction Address Misaligned",
        RISCV_EXCP_INST_ACCESS_FAULT => "Instruction Access Fault",
        RISCV_EXCP_ILLEGAL_INST => "Illegal Instruction",
        RISCV_EXCP_BREAKPOINT => "Breakpoint",
        RISCV_EXCP_LOAD_ADDR_MIS => "Load Address Misaligned",
        RISCV_EXCP_LOAD_ACCESS_FAULT => "Load Access Fault",
        RISCV_EXCP_STORE_AMO_ADDR_MIS => "Store/AMO Address Misaligned",
        RISCV_EXCP_STORE_AMO_ACCESS_FAULT => "Store/AMO Access Fault",
        RISCV_EXCP_U_ECALL => "User ECALL",
        RISCV_EXCP_S_ECALL => "Supervisor ECALL",
        RISCV_EXCP_VS_ECALL => "Virtual Supervisor ECALL",
        RISCV_EXCP_M_ECALL => "Machine ECALL",
        RISCV_EXCP_INST_PAGE_FAULT => "Instruction Page Fault",
        RISCV_EXCP_LOAD_PAGE_FAULT => "Load Page Fault",
        RISCV_EXCP_STORE_PAGE_FAULT => "Store Page Fault",
        RISCV_EXCP_SW_CHECK => "SW check",
        RISCV_EXCP_HW_ERR => "HW Error",
        // The `_` acts as the `default:` case
        _ => "Unknown Exception",
    }
}

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
#[unsafe(no_mangle)]
pub extern "C" fn default_smode_exception_handler() {
    // 1. Get current CPU ID using the safe wrapper
    let cpu_id = thread_attr_fns::CurrentHart::cpu_id();

    // 2. Use our shiny new inline assembly macro!
    let scause = crate::read_csr!(scause);
    let sepc = crate::read_csr!(sepc);
    let stval = crate::read_csr!(stval);
    let sstatus = crate::read_csr!(sstatus);

    let exception_id = scause & crate::cpu_bits::SCAUSE_EC_MASK;

    // 3. Use direct UART writes to avoid deadlocking on the UART lock.
    //    If this exception was taken while the lock was held (e.g. during
    //    a print operation), calling `println!` would spin forever.
    let exc_name = get_exception_name(exception_id);
    crate::uart_write_str_direct("CPU_");
    crate::uart_write_fmt_direct(format_args!(
        "{}_LOG: ERROR: Unexpected exception occurred!\n",
        cpu_id
    ));
    crate::uart_write_fmt_direct(format_args!(
        "CPU_{}_LOG: Exception ID: 0x{:x} ({})\n",
        cpu_id, exception_id, exc_name
    ));
    crate::uart_write_fmt_direct(format_args!(
        "CPU_{}_LOG: Program Counter (sepc): 0x{:x}\n",
        cpu_id, sepc
    ));
    crate::uart_write_fmt_direct(format_args!(
        "CPU_{}_LOG: Trap Value (stval): 0x{:x}\n",
        cpu_id, stval
    ));
    crate::uart_write_fmt_direct(format_args!(
        "CPU_{}_LOG: Status Register (sstatus): 0x{:x}\n",
        cpu_id, sstatus
    ));
    crate::uart_write_fmt_direct(format_args!(
        "CPU_{}_LOG: Status bits: SPP={} | SIE={} | SPIE={} | UBE={} | SBE={}\n",
        cpu_id,
        (sstatus >> 8) & 1,  // SPP
        (sstatus >> 1) & 1,  // SIE
        (sstatus >> 5) & 1,  // SPIE
        (sstatus >> 6) & 1,  // UBE
        (sstatus >> 36) & 1  // SBE
    ));

    unsafe { thread_attr_fns::jumpstart_smode_fail() };
}

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
#[unsafe(no_mangle)]
pub extern "C" fn register_default_smode_exception_handlers() {
    use crate::cpu_bits::*;
    let handler_addr = default_smode_exception_handler as *const () as u64;

    register_smode_trap_handler_override(RISCV_EXCP_INST_ADDR_MIS, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_INST_ACCESS_FAULT, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_ILLEGAL_INST, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_BREAKPOINT, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_LOAD_ADDR_MIS, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_LOAD_ACCESS_FAULT, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_STORE_AMO_ADDR_MIS, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_STORE_AMO_ACCESS_FAULT, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_INST_PAGE_FAULT, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_LOAD_PAGE_FAULT, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_STORE_PAGE_FAULT, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_SW_CHECK, handler_addr);
    register_smode_trap_handler_override(RISCV_EXCP_HW_ERR, handler_addr);
}

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
#[unsafe(no_mangle)]
pub extern "C" fn register_smode_trap_handler_override(mcause: u64, handler_address: u64) {
    // Calling an external C function requires an `unsafe` block
    let struct_addr =
        unsafe { thread_attr_fns::get_thread_attributes_trap_override_struct_address_from_smode() };

    // Cast the raw `u64` address into a mutable raw pointer to our generated struct
    let trap_overrides = struct_addr as *mut trap_override_attributes;

    // Convert the exception code to a `usize` because Rust requires array indices to be `usize`
    let exception_code = (mcause & MCAUSE_EC_MASK) as usize;
    let is_interrupt = (mcause & MCAUSE_INT_FLAG) != 0;

    // Dereferencing a raw pointer requires an `unsafe` block because the compiler
    // cannot statically guarantee the memory address is valid.
    unsafe {
        if is_interrupt {
            if exception_code >= NUM_SMODE_INTERRUPT_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_smode_fail();
            }
            // `(*ptr).field` is how we dereference and access fields of a raw pointer
            (*trap_overrides).smode_interrupt_handler_overrides[exception_code] = handler_address;
        } else {
            if exception_code >= NUM_SMODE_EXCEPTION_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_smode_fail();
            }
            (*trap_overrides).smode_exception_handler_overrides[exception_code] = handler_address;
        }
    }
}

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
#[unsafe(no_mangle)]
pub extern "C" fn deregister_smode_trap_handler_override(mcause: u64) {
    let struct_addr =
        unsafe { thread_attr_fns::get_thread_attributes_trap_override_struct_address_from_smode() };
    let trap_overrides = struct_addr as *mut trap_override_attributes;

    let exception_code = (mcause & MCAUSE_EC_MASK) as usize;
    let is_interrupt = (mcause & MCAUSE_INT_FLAG) != 0;

    unsafe {
        if is_interrupt {
            if exception_code >= NUM_SMODE_INTERRUPT_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_smode_fail();
            }
            if (*trap_overrides).smode_interrupt_handler_overrides[exception_code] == 0 {
                thread_attr_fns::jumpstart_smode_fail();
            }
            (*trap_overrides).smode_interrupt_handler_overrides[exception_code] = 0;
        } else {
            if exception_code >= NUM_SMODE_EXCEPTION_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_smode_fail();
            }
            if (*trap_overrides).smode_exception_handler_overrides[exception_code] == 0 {
                thread_attr_fns::jumpstart_smode_fail();
            }
            (*trap_overrides).smode_exception_handler_overrides[exception_code] = 0;
        }
    }
}

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
#[unsafe(no_mangle)]
pub extern "C" fn get_smode_trap_handler_override(mcause: u64) -> u64 {
    let struct_addr =
        unsafe { thread_attr_fns::get_thread_attributes_trap_override_struct_address_from_smode() };
    let trap_overrides = struct_addr as *const trap_override_attributes;

    let exception_code = (mcause & MCAUSE_EC_MASK) as usize;
    let is_interrupt = (mcause & MCAUSE_INT_FLAG) != 0;

    unsafe {
        if is_interrupt {
            if exception_code >= NUM_SMODE_INTERRUPT_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_smode_fail();
            }
            (*trap_overrides).smode_interrupt_handler_overrides[exception_code]
        } else {
            if exception_code >= NUM_SMODE_EXCEPTION_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_smode_fail();
            }
            (*trap_overrides).smode_exception_handler_overrides[exception_code]
        }
    }
}

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
#[unsafe(no_mangle)]
pub extern "C" fn register_vsmode_trap_handler_override(mcause: u64, handler_address: u64) {
    if thread_attr_fns::CurrentHart::v_bit() != 1 {
        unsafe { thread_attr_fns::jumpstart_vsmode_fail() };
    }

    // Calling an external C function requires an `unsafe` block
    let struct_addr =
        unsafe { thread_attr_fns::get_thread_attributes_trap_override_struct_address_from_smode() };

    // Cast the raw `u64` address into a mutable raw pointer to our generated struct
    let trap_overrides = struct_addr as *mut trap_override_attributes;

    // Convert the exception code to a `usize` because Rust requires array indices to be `usize`
    let exception_code = (mcause & MCAUSE_EC_MASK) as usize;
    let is_interrupt = (mcause & MCAUSE_INT_FLAG) != 0;

    // Dereferencing a raw pointer requires an `unsafe` block because the compiler
    // cannot statically guarantee the memory address is valid.
    unsafe {
        if is_interrupt {
            if exception_code >= NUM_VSMODE_INTERRUPT_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_vsmode_fail();
            }
            // `(*ptr).field` is how we dereference and access fields of a raw pointer
            (*trap_overrides).vsmode_interrupt_handler_overrides[exception_code] = handler_address;
        } else {
            if exception_code >= NUM_VSMODE_EXCEPTION_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_vsmode_fail();
            }
            (*trap_overrides).vsmode_exception_handler_overrides[exception_code] = handler_address;
        }
    }
}

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
#[unsafe(no_mangle)]
pub extern "C" fn deregister_vsmode_trap_handler_override(mcause: u64) {
    if thread_attr_fns::CurrentHart::v_bit() != 1 {
        unsafe { thread_attr_fns::jumpstart_vsmode_fail() };
    }

    let struct_addr =
        unsafe { thread_attr_fns::get_thread_attributes_trap_override_struct_address_from_smode() };
    let trap_overrides = struct_addr as *mut trap_override_attributes;

    let exception_code = (mcause & MCAUSE_EC_MASK) as usize;
    let is_interrupt = (mcause & MCAUSE_INT_FLAG) != 0;

    unsafe {
        if is_interrupt {
            if exception_code >= NUM_VSMODE_INTERRUPT_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_vsmode_fail();
            }
            if (*trap_overrides).vsmode_interrupt_handler_overrides[exception_code] == 0 {
                thread_attr_fns::jumpstart_vsmode_fail();
            }
            (*trap_overrides).vsmode_interrupt_handler_overrides[exception_code] = 0;
        } else {
            if exception_code >= NUM_VSMODE_EXCEPTION_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_vsmode_fail();
            }
            if (*trap_overrides).vsmode_exception_handler_overrides[exception_code] == 0 {
                thread_attr_fns::jumpstart_vsmode_fail();
            }
            (*trap_overrides).vsmode_exception_handler_overrides[exception_code] = 0;
        }
    }
}

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
#[unsafe(no_mangle)]
pub extern "C" fn get_vsmode_trap_handler_override(mcause: u64) -> u64 {
    if thread_attr_fns::CurrentHart::v_bit() != 1 {
        unsafe { thread_attr_fns::jumpstart_vsmode_fail() };
    }

    let struct_addr =
        unsafe { thread_attr_fns::get_thread_attributes_trap_override_struct_address_from_smode() };
    let trap_overrides = struct_addr as *const trap_override_attributes;

    let exception_code = (mcause & MCAUSE_EC_MASK) as usize;
    let is_interrupt = (mcause & MCAUSE_INT_FLAG) != 0;

    unsafe {
        if is_interrupt {
            if exception_code >= NUM_VSMODE_INTERRUPT_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_vsmode_fail();
            }
            (*trap_overrides).vsmode_interrupt_handler_overrides[exception_code]
        } else {
            if exception_code >= NUM_VSMODE_EXCEPTION_HANDLER_OVERRIDES {
                thread_attr_fns::jumpstart_vsmode_fail();
            }
            (*trap_overrides).vsmode_exception_handler_overrides[exception_code]
        }
    }
}
