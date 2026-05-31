/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::cpu_bits;
use crate::thread_attr_fns;
use crate::utils::{self, BitRange};

#[repr(C, packed)]
pub struct TranslationInfo {
    pub va: u64,
    pub pa: u64,
    pub pte_address: [u64; cpu_bits::MAX_NUM_PAGE_TABLE_LEVELS],
    pub pte_value: [u64; cpu_bits::MAX_NUM_PAGE_TABLE_LEVELS],
    pub xatp_mode: u8,
    pub levels_traversed: u8,
    pub walk_successful: u8,
    pub pbmt_mode: u8,
}

pub struct MmuModeAttribute {
    pub xatp_mode: u8,
    pub pte_size_in_bytes: u8,
    pub num_levels: u8,
    pub va_vpn_bits: [BitRange; cpu_bits::MAX_NUM_PAGE_TABLE_LEVELS],
    pub pa_ppn_bits: [BitRange; cpu_bits::MAX_NUM_PAGE_TABLE_LEVELS],
    pub pte_ppn_bits: [BitRange; cpu_bits::MAX_NUM_PAGE_TABLE_LEVELS],
    pub pbmt_mode_bits: BitRange,
}

pub const MMU_HSMODE_ATTRIBUTES: [MmuModeAttribute; 2] = [
    MmuModeAttribute {
        xatp_mode: cpu_bits::VM_1_10_SV39X4,
        pte_size_in_bytes: 8,
        num_levels: 3,
        va_vpn_bits: [
            BitRange { msb: 40, lsb: 30 },
            BitRange { msb: 29, lsb: 21 },
            BitRange { msb: 20, lsb: 12 },
            BitRange { msb: 0, lsb: 0 },
        ],
        pa_ppn_bits: [
            BitRange { msb: 55, lsb: 30 },
            BitRange { msb: 29, lsb: 21 },
            BitRange { msb: 20, lsb: 12 },
            BitRange { msb: 0, lsb: 0 },
        ],
        pte_ppn_bits: [
            BitRange { msb: 53, lsb: 28 },
            BitRange { msb: 27, lsb: 19 },
            BitRange { msb: 18, lsb: 10 },
            BitRange { msb: 0, lsb: 0 },
        ],
        pbmt_mode_bits: BitRange { msb: 62, lsb: 61 },
    },
    MmuModeAttribute {
        xatp_mode: cpu_bits::VM_1_10_SV48X4,
        pte_size_in_bytes: 8,
        num_levels: 4,
        va_vpn_bits: [
            BitRange { msb: 49, lsb: 39 },
            BitRange { msb: 38, lsb: 30 },
            BitRange { msb: 29, lsb: 21 },
            BitRange { msb: 20, lsb: 12 },
        ],
        pa_ppn_bits: [
            BitRange { msb: 55, lsb: 39 },
            BitRange { msb: 38, lsb: 30 },
            BitRange { msb: 29, lsb: 21 },
            BitRange { msb: 20, lsb: 12 },
        ],
        pte_ppn_bits: [
            BitRange { msb: 53, lsb: 37 },
            BitRange { msb: 36, lsb: 28 },
            BitRange { msb: 27, lsb: 19 },
            BitRange { msb: 18, lsb: 10 },
        ],
        pbmt_mode_bits: BitRange { msb: 62, lsb: 61 },
    },
];

pub const MMU_SMODE_ATTRIBUTES: [MmuModeAttribute; 2] = [
    MmuModeAttribute {
        xatp_mode: cpu_bits::VM_1_10_SV39,
        pte_size_in_bytes: 8,
        num_levels: 3,
        va_vpn_bits: [
            BitRange { msb: 38, lsb: 30 },
            BitRange { msb: 29, lsb: 21 },
            BitRange { msb: 20, lsb: 12 },
            BitRange { msb: 0, lsb: 0 },
        ],
        pa_ppn_bits: [
            BitRange { msb: 55, lsb: 30 },
            BitRange { msb: 29, lsb: 21 },
            BitRange { msb: 20, lsb: 12 },
            BitRange { msb: 0, lsb: 0 },
        ],
        pte_ppn_bits: [
            BitRange { msb: 53, lsb: 28 },
            BitRange { msb: 27, lsb: 19 },
            BitRange { msb: 18, lsb: 10 },
            BitRange { msb: 0, lsb: 0 },
        ],
        pbmt_mode_bits: BitRange { msb: 62, lsb: 61 },
    },
    MmuModeAttribute {
        xatp_mode: cpu_bits::VM_1_10_SV48,
        pte_size_in_bytes: 8,
        num_levels: 4,
        va_vpn_bits: [
            BitRange { msb: 47, lsb: 39 },
            BitRange { msb: 38, lsb: 30 },
            BitRange { msb: 29, lsb: 21 },
            BitRange { msb: 20, lsb: 12 },
        ],
        pa_ppn_bits: [
            BitRange { msb: 55, lsb: 39 },
            BitRange { msb: 38, lsb: 30 },
            BitRange { msb: 29, lsb: 21 },
            BitRange { msb: 20, lsb: 12 },
        ],
        pte_ppn_bits: [
            BitRange { msb: 53, lsb: 37 },
            BitRange { msb: 36, lsb: 28 },
            BitRange { msb: 27, lsb: 19 },
            BitRange { msb: 18, lsb: 10 },
        ],
        pbmt_mode_bits: BitRange { msb: 62, lsb: 61 },
    },
];

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
fn translate(
    xatp: u64,
    mmu_mode_attribute: &MmuModeAttribute,
    va: u64,
    xlate_info: &mut TranslationInfo,
) {
    xlate_info.xatp_mode = cpu_bits::get_field(xatp, cpu_bits::SATP64_MODE) as u8;
    xlate_info.va = va;
    xlate_info.pa = 0;
    xlate_info.levels_traversed = 0;
    xlate_info.walk_successful = 0;
    for i in 0..cpu_bits::MAX_NUM_PAGE_TABLE_LEVELS {
        xlate_info.pte_address[i] = 0;
        xlate_info.pte_value[i] = 0;
    }

    if xlate_info.xatp_mode == cpu_bits::VM_1_10_MBARE {
        xlate_info.pa = va;
        xlate_info.walk_successful = 1;
        return;
    }

    // Step 1
    let mut a: u64 = (xatp & cpu_bits::SATP64_PPN) << cpu_bits::PAGE_OFFSET;
    let mut current_level: u8 = 0;

    // Step 2
    loop {
        let pte_addr =
            a + utils::extract_bits(va, mmu_mode_attribute.va_vpn_bits[current_level as usize])
                * (mmu_mode_attribute.pte_size_in_bytes as u64);

        xlate_info.pte_address[current_level as usize] = pte_addr;

        let pte_value = unsafe { core::ptr::read_volatile(pte_addr as *const u64) };
        xlate_info.pte_value[current_level as usize] = pte_value;

        xlate_info.levels_traversed += 1;

        if cpu_bits::get_field(pte_value, cpu_bits::PTE_V) == 0 {
            // PTE is not valid. stop the walk.
            return;
        }

        let xwr = cpu_bits::get_field(
            pte_value,
            cpu_bits::PTE_R | cpu_bits::PTE_W | cpu_bits::PTE_X,
        ) as u8;

        if (xwr & 0x3) == 0x2 {
            // PTE at pte_address has R=0 and W=1.
            unsafe {
                thread_attr_fns::jumpstart_smode_fail();
            }
        }

        a = 0;

        for ppn_id in 0..mmu_mode_attribute.num_levels {
            let ppn_value =
                utils::extract_bits(pte_value, mmu_mode_attribute.pte_ppn_bits[ppn_id as usize]);
            a = utils::place_bits(
                a,
                ppn_value,
                mmu_mode_attribute.pa_ppn_bits[ppn_id as usize],
            );
        }

        if (xwr & 0x6) != 0 || (xwr & 0x1) != 0 {
            // This is a Leaf PTE. Done with the walk.
            break;
        } else if cpu_bits::get_field(pte_value, cpu_bits::PTE_A) != 0 {
            // PTE has A=1 but is not a Leaf PTE.
            unsafe {
                thread_attr_fns::jumpstart_smode_fail();
            }
        } else if cpu_bits::get_field(pte_value, cpu_bits::PTE_D) != 0 {
            // PTE has D=1 but is not a Leaf PTE
            unsafe {
                thread_attr_fns::jumpstart_smode_fail();
            }
        }

        current_level += 1;
        if current_level >= mmu_mode_attribute.num_levels {
            // Ran out of levels
            unsafe {
                thread_attr_fns::jumpstart_smode_fail();
            }
        }
    }

    xlate_info.pbmt_mode = utils::extract_bits(
        xlate_info.pte_value[(xlate_info.levels_traversed - 1) as usize],
        mmu_mode_attribute.pbmt_mode_bits,
    ) as u8;
    xlate_info.pa = a + utils::extract_bits(
        va,
        BitRange {
            msb: (cpu_bits::PAGE_OFFSET - 1) as u8,
            lsb: 0,
        },
    );
    xlate_info.walk_successful = 1;
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn translate_GVA(gva: u64, xlate_info: *mut TranslationInfo) {
    let vsatp_value = crate::read_csr!(vsatp);
    let mode = cpu_bits::get_field(vsatp_value, cpu_bits::VSATP64_MODE) as u8;

    let mut attribute: Option<&MmuModeAttribute> = None;
    for attr in MMU_SMODE_ATTRIBUTES.iter() {
        if attr.xatp_mode == mode {
            attribute = Some(attr);
            break;
        }
    }

    if let Some(attr) = attribute {
        unsafe {
            translate(vsatp_value, attr, gva, &mut *xlate_info);
        }
    } else {
        unsafe {
            thread_attr_fns::jumpstart_smode_fail();
        }
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn translate_GPA(gpa: u64, xlate_info: *mut TranslationInfo) {
    let hgatp_value = crate::read_csr!(hgatp);
    let mode = cpu_bits::get_field(hgatp_value, cpu_bits::HGATP64_MODE) as u8;

    let mut attribute: Option<&MmuModeAttribute> = None;
    for attr in MMU_HSMODE_ATTRIBUTES.iter() {
        if attr.xatp_mode == mode {
            attribute = Some(attr);
            break;
        }
    }

    if let Some(attr) = attribute {
        unsafe {
            translate(hgatp_value, attr, gpa, &mut *xlate_info);
        }
    } else {
        unsafe {
            thread_attr_fns::jumpstart_smode_fail();
        }
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn translate_VA(va: u64, xlate_info: *mut TranslationInfo) {
    let satp_value = crate::read_csr!(satp);
    let mode = cpu_bits::get_field(satp_value, cpu_bits::SATP64_MODE) as u8;

    let mut attribute: Option<&MmuModeAttribute> = None;
    for attr in MMU_SMODE_ATTRIBUTES.iter() {
        if attr.xatp_mode == mode {
            attribute = Some(attr);
            break;
        }
    }

    if let Some(attr) = attribute {
        unsafe {
            translate(satp_value, attr, va, &mut *xlate_info);
        }
    } else {
        unsafe {
            thread_attr_fns::jumpstart_smode_fail();
        }
    }
}
