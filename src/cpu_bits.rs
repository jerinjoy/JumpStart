/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#[inline]
pub const fn bit(nr: u32) -> u64 {
    1u64 << nr
}

#[inline]
pub const fn bit_mask(start: u32, end: u32) -> u64 {
    (!0u64 >> (64 - (end - start + 1))) << start
}

#[inline]
pub const fn get_field(reg: u64, mask: u64) -> u64 {
    (reg & mask) / (mask & mask.wrapping_neg())
}

#[inline]
pub const fn set_field(reg: u64, mask: u64, val: u64) -> u64 {
    (reg & !mask) | ((val * (mask & mask.wrapping_neg())) & mask)
}

#[inline]
pub const fn align_up_size(base: u64, size: u64) -> u64 {
    (base + size - 1) & !(size - 1)
}

pub const EXT_STATUS_MASK: u64 = 0x3;

pub const FSR_RD_SHIFT: u32 = 5;
pub const FSR_RD: u64 = 0x7 << FSR_RD_SHIFT;

pub const FPEXC_NX: u64 = 0x01;
pub const FPEXC_UF: u64 = 0x02;
pub const FPEXC_OF: u64 = 0x04;
pub const FPEXC_DZ: u64 = 0x08;
pub const FPEXC_NV: u64 = 0x10;

pub const FSR_AEXC_SHIFT: u32 = 0;
pub const FSR_NVA: u64 = FPEXC_NV << FSR_AEXC_SHIFT;
pub const FSR_OFA: u64 = FPEXC_OF << FSR_AEXC_SHIFT;
pub const FSR_UFA: u64 = FPEXC_UF << FSR_AEXC_SHIFT;
pub const FSR_DZA: u64 = FPEXC_DZ << FSR_AEXC_SHIFT;
pub const FSR_NXA: u64 = FPEXC_NX << FSR_AEXC_SHIFT;
pub const FSR_AEXC: u64 = FSR_NVA | FSR_OFA | FSR_UFA | FSR_DZA | FSR_NXA;

pub const FSR_VXRM_SHIFT: u32 = 9;
pub const FSR_VXRM: u64 = 0x3 << FSR_VXRM_SHIFT;

pub const FSR_VXSAT_SHIFT: u32 = 8;
pub const FSR_VXSAT: u64 = 0x1 << FSR_VXSAT_SHIFT;

pub const CSR_SSP: u16 = 0x011;

pub const CSR_USTATUS: u16 = 0x000;
pub const CSR_UIE: u16 = 0x004;
pub const CSR_UTVEC: u16 = 0x005;

pub const CSR_USCRATCH: u16 = 0x040;
pub const CSR_UEPC: u16 = 0x041;
pub const CSR_UCAUSE: u16 = 0x042;
pub const CSR_UTVAL: u16 = 0x043;
pub const CSR_UIP: u16 = 0x044;

pub const CSR_FFLAGS: u16 = 0x001;
pub const CSR_FRM: u16 = 0x002;
pub const CSR_FCSR: u16 = 0x003;

pub const CSR_VSTART: u16 = 0x008;
pub const CSR_VXSAT: u16 = 0x009;
pub const CSR_VXRM: u16 = 0x00a;
pub const CSR_VCSR: u16 = 0x00f;
pub const CSR_VL: u16 = 0xc20;
pub const CSR_VTYPE: u16 = 0xc21;
pub const CSR_VLENB: u16 = 0xc22;

pub const VCSR_VXSAT_SHIFT: u32 = 0;
pub const VCSR_VXSAT: u64 = 0x1 << VCSR_VXSAT_SHIFT;
pub const VCSR_VXRM_SHIFT: u32 = 1;
pub const VCSR_VXRM: u64 = 0x3 << VCSR_VXRM_SHIFT;

pub const CSR_MCYCLE: u16 = 0xb00;
pub const CSR_MINSTRET: u16 = 0xb02;
pub const CSR_MCYCLEH: u16 = 0xb80;
pub const CSR_MINSTRETH: u16 = 0xb82;

pub const CSR_MVENDORID: u16 = 0xf11;
pub const CSR_MARCHID: u16 = 0xf12;
pub const CSR_MIMPID: u16 = 0xf13;
pub const CSR_MHARTID: u16 = 0xf14;
pub const CSR_MCONFIGPTR: u16 = 0xf15;

pub const CSR_MSTATUS: u16 = 0x300;
pub const CSR_MISA: u16 = 0x301;
pub const CSR_MEDELEG: u16 = 0x302;
pub const CSR_MIDELEG: u16 = 0x303;
pub const CSR_MIE: u16 = 0x304;
pub const CSR_MTVEC: u16 = 0x305;
pub const CSR_MCOUNTEREN: u16 = 0x306;

pub const MCAUSE_INT_FLAG: u64 = 0x8000_0000_0000_0000;
pub const MCAUSE_EC_MASK: u64 = 0x7FFF_FFFF_FFFF_FFFF;

pub const CSR_SSTATUS: u16 = 0x100;
pub const CSR_SIE: u16 = 0x104;
pub const CSR_STVEC: u16 = 0x105;
pub const CSR_SCOUNTEREN: u16 = 0x106;

pub const CSR_SSCRATCH: u16 = 0x140;
pub const CSR_SEPC: u16 = 0x141;
pub const CSR_SCAUSE: u16 = 0x142;
pub const CSR_STVAL: u16 = 0x143;
pub const CSR_SIP: u16 = 0x144;

pub const SCAUSE_INT_FLAG: u64 = 0x8000_0000_0000_0000;
pub const SCAUSE_EC_MASK: u64 = 0x7FFF_FFFF_FFFF_FFFF;

pub const CSR_SATP: u16 = 0x180;

pub const PTE_V: u64 = 0x001;
pub const PTE_R: u64 = 0x002;
pub const PTE_W: u64 = 0x004;
pub const PTE_X: u64 = 0x008;
pub const PTE_U: u64 = 0x010;
pub const PTE_G: u64 = 0x020;
pub const PTE_A: u64 = 0x040;
pub const PTE_D: u64 = 0x080;
pub const PTE_SOFT: u64 = 0x300;
pub const PTE_PBMT: u64 = 0x6000_0000_0000_0000;
pub const PTE_N: u64 = 0x8000_0000_0000_0000;
pub const PTE_ATTR: u64 = PTE_N | PTE_PBMT;

pub const PTE_PPN_SHIFT: u32 = 10;
pub const PTE_PPN_MASK: u64 = 0x3FFF_FFFF_FFFC_00;
pub const PGSHIFT: u32 = 12;

pub const MSTATUS_UIE: u64 = 0x0000_0001;
pub const MSTATUS_SIE: u64 = 0x0000_0002;
pub const MSTATUS_MIE: u64 = 0x0000_0008;
pub const MSTATUS_SPIE: u64 = 0x0000_0020;
pub const MSTATUS_SPP: u64 = 0x0000_0100;
pub const MSTATUS_FS: u64 = 0x0000_6000;
pub const MSTATUS_SUM: u64 = 0x0004_0000;
pub const MSTATUS_MXR: u64 = 0x0008_0000;
pub const MSTATUS_UXL: u64 = 0x3_0000_0000;
pub const MSTATUS_SXL: u64 = 0xC_0000_0000;
pub const MSTATUS64_SD: u64 = 0x8000_0000_0000_0000;

pub const RISCV_EXCP_NONE: u64 = 0xFFFFFFFFFFFFFFFF;
pub const RISCV_EXCP_INST_ADDR_MIS: u64 = 0x0;
pub const RISCV_EXCP_INST_ACCESS_FAULT: u64 = 0x1;
pub const RISCV_EXCP_ILLEGAL_INST: u64 = 0x2;
pub const RISCV_EXCP_BREAKPOINT: u64 = 0x3;
pub const RISCV_EXCP_LOAD_ADDR_MIS: u64 = 0x4;
pub const RISCV_EXCP_LOAD_ACCESS_FAULT: u64 = 0x5;
pub const RISCV_EXCP_STORE_AMO_ADDR_MIS: u64 = 0x6;
pub const RISCV_EXCP_STORE_AMO_ACCESS_FAULT: u64 = 0x7;
pub const RISCV_EXCP_U_ECALL: u64 = 0x8;
pub const RISCV_EXCP_S_ECALL: u64 = 0x9;
pub const RISCV_EXCP_VS_ECALL: u64 = 0xa;
pub const RISCV_EXCP_M_ECALL: u64 = 0xb;
pub const RISCV_EXCP_INST_PAGE_FAULT: u64 = 0xc;
pub const RISCV_EXCP_LOAD_PAGE_FAULT: u64 = 0xd;
pub const RISCV_EXCP_STORE_PAGE_FAULT: u64 = 0xf;
pub const RISCV_EXCP_SW_CHECK: u64 = 0x12;
pub const RISCV_EXCP_HW_ERR: u64 = 0x13;
pub const RISCV_EXCP_INST_GUEST_PAGE_FAULT: u64 = 0x14;
pub const RISCV_EXCP_LOAD_GUEST_ACCESS_FAULT: u64 = 0x15;
pub const RISCV_EXCP_VIRT_INSTRUCTION_FAULT: u64 = 0x16;
pub const RISCV_EXCP_STORE_GUEST_AMO_ACCESS_FAULT: u64 = 0x17;
pub const RISCV_EXCP_SEMIHOST: u64 = 0x3f;

pub const IRQ_S_SOFT: u32 = 1;
pub const IRQ_S_TIMER: u32 = 5;
pub const IRQ_S_EXT: u32 = 9;

pub const MIP_SSIP: u64 = 1 << IRQ_S_SOFT;
pub const MIP_STIP: u64 = 1 << IRQ_S_TIMER;
pub const MIP_SEIP: u64 = 1 << IRQ_S_EXT;

pub const SEED_OPST: u64 = 3 << 30;
pub const SEED_OPST_BIST: u64 = 0;
pub const SEED_OPST_WAIT: u64 = 1;
pub const SEED_OPST_ES16: u64 = 2;
pub const SEED_OPST_DEAD: u64 = 3;
pub const SEED_ENTROPY_MASK: u64 = 0xFFFF;

pub const SATP64_MODE: u64 = 0xF000_0000_0000_0000;
pub const SATP64_ASID: u64 = 0x0FFF_F000_0000_0000;
pub const SATP64_PPN: u64 = 0x0000_0FFF_FFFF_FFFF;

pub const VSATP64_MODE: u64 = SATP64_MODE;
pub const VSATP64_ASID: u64 = SATP64_ASID;
pub const VSATP64_PPN: u64 = SATP64_PPN;

pub const HGATP64_MODE: u64 = SATP64_MODE;
pub const HGATP64_ASID: u64 = SATP64_ASID;
pub const HGATP64_PPN: u64 = SATP64_PPN;

pub const VM_1_10_MBARE: u8 = 0;
pub const VM_1_10_SV32: u8 = 1;
pub const VM_1_10_SV39: u8 = 8;
pub const VM_1_10_SV48: u8 = 9;
pub const VM_1_10_SV57: u8 = 10;
pub const VM_1_10_SV64: u8 = 11;

pub const VM_1_10_SV39X4: u8 = 8;
pub const VM_1_10_SV48X4: u8 = 9;
pub const VM_1_10_SV57X4: u8 = 10;

pub const MAX_NUM_PAGE_TABLE_LEVELS: usize = 4;
pub const PAGE_OFFSET: u32 = 12;
