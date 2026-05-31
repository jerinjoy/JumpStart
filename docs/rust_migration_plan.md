<!--
SPDX-FileCopyrightText: 2026 Jerin Joy
SPDX-FileCopyrightText: 2026 Rivos Inc.

SPDX-License-Identifier: Apache-2.0
-->

# JumpStart: C to no_std Rust Migration Plan

This document tracks the ongoing effort to port the JumpStart bare-metal
RISC-V validation project from C to idiomatic `#![no_std]` Rust. It is
intended to be checked into the repository and serve as a living handoff
document for any agent or contributor picking up this work.

## Architecture & Gotchas

> **Linker Sections:** The existing C code uses `__attr_mtext` and
> `__attr_stext` macros (defined in `jumpstart.h`) to place functions into
> specific linker sections (e.g. `.jumpstart.cpu.text.mmode`,
> `.jumpstart.cpu.text.smode`).
>
> **Every Rust function that replaces a C function MUST carry the equivalent
> attribute:**
> ```rust
> #[unsafe(link_section = ".jumpstart.cpu.text.smode")]
> pub fn my_function() { ... }
> ```
> Without this, the Rust function will land in the default `.text` section and
> break the existing linker script memory layout.

Additional cross-cutting rules:
- All `extern "C"` blocks must be `unsafe extern "C"` (Rust 2024 edition).
- All exported C-compatible functions must use `#[unsafe(no_mangle)]`.
- All Rust structs that cross the FFI boundary must be `#[repr(C)]`.
- The crate is `#![no_std]`. Bring in the `alloc` crate via
  `extern crate alloc;` in `lib.rs` once the heap is set up.
- **DO NOT remove any existing C files or headers during the migration.** We still want the C code to compile and work until the entire project is converted to Rust.

---

## Phase 1: Cargo Build System — COMPLETED

**Goal:** Replace the Meson build with Cargo while keeping the existing C code
compilable.

Key files:
| Rust file | Replaces / Adds |
|---|---|
| `Cargo.toml` | New — defines crate, RISC-V target, `no_std` |
| `build.rs` | New — drives C compilation & diag source generation |
| `.cargo/config.toml` | New — sets default RISC-V target, linker flags |

---

## Phase 2: Volatile UART — COMPLETED

**Goal:** Port `uart.smode.c` / `uart.mmode.c` to safe Rust with volatile
MMIO semantics.

Key files:
| Rust file | Replaces |
|---|---|
| `src/uart.rs` | `src/common/uart.smode.c` |

Notable decisions:
- Used raw-pointer volatile reads/writes (`core::ptr::read_volatile`,
  `core::ptr::write_volatile`) to prevent the compiler from optimising away
  MMIO accesses.
- Implemented `core::fmt::Write` on `Uart` so `write!` / `writeln!` work
  directly.

---

## Phase 3: Atomic Spinlock — COMPLETED

**Goal:** Port `lock.smode.c` / `lock.mmode.c` to a generic Rust type that
wraps any `T` and is safe to share across harts.

Key files:
| Rust file | Replaces |
|---|---|
| `src/lock.rs` | `src/common/lock.smode.c`, `src/common/lock.mmode.c` |

Notable decisions:
- `Spinlock<T>` wraps `UnsafeCell<T>` and an `AtomicBool`.
- `SpinlockGuard<T>` implements `Deref`, `DerefMut`, and `Drop` (RAII unlock).
- `unsafe impl<T> Sync for Spinlock<T>` tells the compiler it is safe to share
  across harts.

---

## Phase 4: Automated FFI Struct Generation — COMPLETED

**Goal:** Eliminate hand-written FFI headers by making the Python diagnostic
source generator emit `#[repr(C)]` Rust structs and constants directly.

Key changes:
- `scripts/generate_diag_sources.py` updated to emit Rust alongside C headers.

---

## Phase 5: Trap Handling — COMPLETED

**Goal:** Replace `trap_handler.mmode.c` and `trap_handler.smode.c`.

Key files:
| Rust file | Replaces |
|---|---|
| `src/trap.rs` | `src/common/trap_handler.mmode.c`, `src/common/trap_handler.smode.c` |
| `src/cpu_bits.rs` | Shared CPU constants (CSR addresses, exception codes, etc.) |

Notable decisions:
- All trap handler entry points use `#[unsafe(no_mangle)]` and
  `#[unsafe(link_section = ".jumpstart.cpu.text.{mmode,smode}")]`.
- Exception code matching uses exhaustive Rust `match` instead of C `switch`.

---

## Phase 6: Inline Assembly Macros — COMPLETED

**Goal:** Provide ergonomic CSR access and formatted printing in Rust.

Key additions in `src/lib.rs`:
- `read_csr!(csr_name)` macro — wraps `core::arch::asm!` with the `csrr`
  instruction.
- `println!(...)` and `print!(...)` macros — use `UART` (a global
  `Spinlock<Uart>`) and `core::fmt::Write`.

---

## Phase 7: Memory Allocation / Heap — COMPLETED

**Goal:** Replace `heap.smode.c` with a Rust `GlobalAlloc` implementation,
unlocking `Box`, `Vec`, `String`, etc. in `no_std`.

Key files:
| Rust file | Replaces |
|---|---|
| `src/heap.rs` | `src/common/heap.smode.c` |

### Key types

```rust
// Linked-list node (mirrors C `struct memchunk`)
#[repr(C)]
pub struct MemChunk { pub next: *mut MemChunk, pub size: usize }

// Per-heap metadata (lock is now OUTSIDE — wrapped in Spinlock<HeapInfo>)
pub struct HeapInfo {
    pub backing_memory: u8,
    pub memory_type: u8,
    pub head: *mut MemChunk,
    pub last_allocated: *mut MemChunk,
    pub size: usize,
    pub setup_done: bool,
}

// Three heaps: WB, WC, UC
pub struct JumpStartHeaps {
    heaps: [Spinlock<HeapInfo>; NUM_HEAPS_SUPPORTED],
}

// Declared as the global allocator
#[global_allocator]
pub static HEAPS: JumpStartHeaps = JumpStartHeaps::new();
```

### Key decisions
- **`Spinlock<HeapInfo>`** instead of an embedded `spinlock_t` field — Rust
  guarantees you cannot read any `HeapInfo` field without first locking.
- **`core::alloc::GlobalAlloc`** implements `alloc()` and `dealloc()`. Both
  are tagged `#[unsafe(link_section = ".jumpstart.cpu.text.smode")]`.
- `alloc()` delegates to the WB heap by default (mirrors C `malloc()`).
- `ChunkIterator` mirrors `chunk_iterator_t` — two-pass search starting from
  `last_allocated` to reduce fragmentation.
- `alloc()` handles aligned allocation inline (mirrors `memalign_from_memory`).
- `dealloc()` coalesces adjacent free chunks forward and backward.
- To activate: call `HEAPS.setup_heap(start, end, BACKING_MEMORY_DDR, MEMORY_TYPE_WB)`
  before any `Box`/`Vec` usage.
- Helper `heap::test_allocation()` validates `Vec`, `Box`, and `String`.

---

## Phase 8: Timers & Utilities — COMPLETED

**Goal:** Replace `time.smode.c`, `time.mmode.c`, `utils.smode.c`, and
`utils.mmode.c`.

### 8a — Time (`src/time.rs`) — COMPLETED
Replaces `src/common/time.smode.c` and `src/common/time.mmode.c`.

| C function | Notes |
|---|---|
| `read_time()` | `core::arch::asm!("rdtime {0}", ...)` |
| `delay_us_from_smode(us)` | Spin-wait on `rdtime`; calls `read_time()` |
| `delay_us_from_mmode(us)` | Same, in mmode section |
| `gettimeofday(tv, tz)` | Divide rdtime ticks by `CPU_CLOCK_FREQUENCY_IN_MHZ` |
| `time(tloc)` | Wrapper around `gettimeofday` |

All functions tagged with appropriate `link_section`.

### 8b — Utilities & Entropy (`src/utils.rs`) — COMPLETED
Replaces `src/common/utils.smode.c` and `src/common/utils.mmode.c`.

| C function | Notes |
|---|---|
| `extract_bits(value, range)` | Pure bit-shift math; `#[inline]` is enough |
| `place_bits(value, bits, range)` | Pure bit-shift math |
| `smode_try_get_seed()` | `csrrw` on the `seed` CSR; retry loop on `OPST` |
| `mmode_try_get_seed()` | Same, mmode section |
| `__smode_random()` | LCG using `AtomicU64::compare_exchange` (replaces `LR/SC`) |
| `get_random_number_from_smode()` | Thin wrapper |
| `set_random_seed_from_smode(seed)` | `AtomicU64::store` |

> **`LR/SC` → `AtomicU64`:** The C code uses custom `load_reserved_64` /
> `store_conditional_64` intrinsics to implement a lock-free LCG.  In Rust,
> `core::sync::atomic::AtomicU64::compare_exchange(Ordering::SeqCst, ...)` in
> a loop compiles to the same `lr.d` / `sc.d` pair on RISC-V.

---

## Phase 9: MMU Page Table Walking — COMPLETED

**Goal:** Replace `tablewalk.smode.c`.

Key file:
| Rust file | Replaces |
|---|---|
| `src/tablewalk.rs` | `src/common/tablewalk.smode.c` |

### Key structures

```rust
pub struct BitRange { pub msb: u8, pub lsb: u8 }

pub struct MmuModeAttribute {
    pub xatp_mode:        u8,
    pub pte_size_in_bytes: u8,
    pub num_levels:       u8,
    pub va_vpn_bits:  [BitRange; MAX_NUM_PAGE_TABLE_LEVELS],
    pub pa_ppn_bits:  [BitRange; MAX_NUM_PAGE_TABLE_LEVELS],
    pub pte_ppn_bits: [BitRange; MAX_NUM_PAGE_TABLE_LEVELS],
    pub pbmt_mode_bits: BitRange,
}
```

Static tables `MMU_SMODE_ATTRIBUTES` and `MMU_HSMODE_ATTRIBUTES` mirror the C
`const struct` arrays for SV39, SV48, SV39x4, SV48x4.

### Key functions

| C function | Notes |
|---|---|
| `translate(xatp, attr, va, xlate_info)` | Private; raw page-table walk using `read_volatile` |
| `translate_VA(va, xlate_info)` | Reads `satp` CSR, dispatches to `translate` |
| `translate_GVA(gva, xlate_info)` | Reads `vsatp` CSR |
| `translate_GPA(gpa, xlate_info)` | Reads `hgatp` CSR |

`translate()` must use `core::ptr::read_volatile` when dereferencing PTE
addresses because the memory is live hardware state.

---

## Phase 10: Thread Attribute Functions — COMPLETED

**Goal:** Port `thread_attributes.mmode.c` and `thread_attributes.smode.c` and centralize access to hart-local metadata.

Key files:
| Rust file | Replaces |
|---|---|
| `src/thread_attr_fns.rs` | `src/common/thread_attributes.mmode.c`, `src/common/thread_attributes.smode.c` |

### Key decisions
- **Centralized Externs:** Moved all `get_thread_attributes_*` assembly function declarations from `src/trap.rs` into `src/thread_attr_fns.rs` to provide a single source of truth for hart-local attributes.
- **Safe Wrappers:** Introduced the `CurrentHart` struct with safe methods (e.g., `CurrentHart::cpu_id()`) to avoid repeated `unsafe` blocks and raw FFI calls in high-level Rust code.
- **Struct Inclusion:** Included the generated `thread_attributes` struct directly to allow cross-CPU attribute lookups using the pointer-based assembly getters.
- **Refactored Consumers:** Updated `src/trap.rs` and `src/tablewalk.rs` to use the new module. Renaming the file to `thread_attr_fns.rs` resolved the name collision with the `thread_attributes` struct, making the code much cleaner and removing the need for module aliasing.

---

## Phase 11: Validation on Spike — COMPLETED

**Goal:** Successfully build a diagnostic test that links against the Rust version of the JumpStart library and execute it on the Spike simulator.

### Approach: The Hybrid Link Strategy
Instead of immediately replacing the entire Meson build, we will integrate the Rust library into a validation binary.

1.  **Static Library Generation:** Update `Cargo.toml` to use `crate-type = ["staticlib", "rlib"]`. This generates `libjumpstart.a`, which contains all our ported Rust logic and the required FFI symbols for the C code to call.
2.  **Meson/Manual Integration:**
    - For a specific test (e.g., `test000`), we will modify the link command to include `libjumpstart.a` and the assembly archive `libjumpstart_asm.a` (generated by `build.rs`).
    - We will systematically "drop" C files from the link (like `uart.smode.c`, `lock.smode.c`) and ensure the linker finds the Rust equivalents via the `#[unsafe(no_mangle)]` symbols.
3.  **Linker Layout Verification:** Use `objdump -h` and `nm` to ensure Rust functions carrying `#[unsafe(link_section = "...")]` are correctly placed in the specific memory sections (e.g., `.jumpstart.cpu.text.smode`) defined by the diagnostic's generated linker script.
4.  **Spike Execution:**
    - Run the binary: `spike --isa=rv64gcvh_zba_zbb_zbs_zkr_svpbmt_smstateen_zicntr_zicclsm <test>.elf`.
    - Validate that `println!` output appears on the console (verifying `Uart` + `Spinlock` + `GlobalAlloc`).
    - Validate that traps are handled by the Rust handlers (verifying `trap.rs`).

### Key Decisions
- **`staticlib` Output:** Added `crate-type = ["staticlib", "rlib"]` to `Cargo.toml` to allow the GCC-based diagnostic build to link against the Rust library.
- **Minimal `rustflags`:** Explicitly set `target-feature=+zihintpause` in `.cargo/config.toml`. Redundant flags like `+f` and `+d` were removed as they are already part of the `riscv64gc` target and caused unstable feature warnings during `cargo check`.
- **Validation Script:** Created `scripts/validate_rust_on_spike.sh` to automate the hybrid build, linking, and Spike execution with tracing enabled.
- **Dynamic Build System (Path 1):** Removed hardcoded test paths from `build.rs`. The build is now parameterized via the `DIAG_YAML` environment variable, forcing a specialized recompile of the library for each diagnostic to maintain static performance and layout optimizations.

- The way we build and run a test against the C variant is by using the scripts/build_diag.py script. Example:
```
❯ scripts/build_diag.py --diag_src_dir tests/common/test051 --diag_build_dir /tmp/diag --environment spike
INFO: [ThreadPoolExecutor-0_0]: Compiling 'tests/common/test051'
INFO: [ThreadPoolExecutor-1_0]: Running diag 'tests/common/test051'
INFO: [MainThread]:
Summary
Build root: /tmp/diag
Build Repro Manifest: /tmp/diag/build_manifest.repro.yaml
┏━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ Diag                 ┃ Build        ┃ Run [spike]  ┃ Result                        ┃
┡━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┩
│ tests/common/test051 │ PASS (2.15s) │ PASS (0.19s) │ /tmp/diag/test051/test051.elf │
└──────────────────────┴──────────────┴──────────────┴───────────────────────────────┘

Diagnostics built: 1
Diagnostics run: 1

Run Manifest:
/tmp/diag/run_manifest.yaml

STATUS: PASSED
```

This calls the underlying meson build system to build the ELF and then run on Spike.

---

## Phase 12: Integrated Meson + Cargo Build System — PLANNED

**Goal:** Unify the Rust and C build flows under the existing `build_diag.py` script by wrapping Cargo inside Meson.

### Key Changes
1. **Meson Backend Option:** Add a `jumpstart_backend` option in `meson.options` with choices `c` and `rust` (default: `c`).
2. **Meson `custom_target` Integration:**
    - Define a `custom_target` in `meson.build` that runs `cargo build`.
    - Pass the `DIAG_YAML` environment variable to the Cargo process.
    - Conditionally swap `jumpstart_sources` (C objects) with the static libraries produced by Cargo (`libjumpstart.a` and `libjumpstart_asm.a`).
3. **`build_diag.py` Update:** Add a `--backend` CLI argument to `build_diag.py` to allow users to easily switch backends.

### Benefits
- **Parity:** Run any diagnostic with either the C or Rust implementation using the same command.
- **Automation:** Eliminates the need for the manual `validate_rust_on_spike.sh` script.
- **Dependency Management:** Meson will correctly rebuild the Rust library if any `.rs` files change before linking the diagnostic ELF.

---

## Future Build System Evolution (Path 2)

While Phase 11 uses a "recompile-per-test" model (Path 1), the long-term goal is to decouple the library from the diagnostic attributes to allow for a single, generic `libjumpstart.a`.

**Proposed Changes:**
1. **Link-Time Interface:** Instead of using `include!` to pull in generated Rust structs in `src/trap.rs` and `src/thread_attr_fns.rs`, declare those symbols as `extern "C"`.
2. **External Data Definition:** The diagnostic build (or a separate assembly/C shim) will be responsible for defining the actual instances of these structures and symbols.
3. **Runtime Configuration:** Replace compile-time assembly macros (like `MMODE_ROLE_ENABLE`) with runtime checks or configuration variables passed in during initialization.

---

## Remaining C Files (Not Yet Planned)

| File | Description | Priority |
|---|---|---|
| `src/common/string.smode.c` | `vsnprintf`, `snprintf`, `strcpy`, `strcmp`, `strlen`, etc. | Low — superseded by Rust's `core::fmt` and our `println!` macro |
| `src/common/sbi_firmware_boot.smode.S` | Assembly SBI boot shim | Defer — keep in assembly |
| `src/common/jumpstart.{m,s,v}mode.S` | Core boot assembly | Defer — keep in assembly |

---

## How to Resume

1. Check out the branch, run `cargo check` — should pass with 0 errors.
2. Review the "Remaining C Files" section to determine the next migration
   target (e.g., `thread_attributes.{m,s}mode.c`).
3. When in doubt about linker section placement, grep for `__attr_stext` in
   the corresponding `.c` file to determine the correct section name.
