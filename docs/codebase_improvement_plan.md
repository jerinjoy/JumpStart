<!--
SPDX-FileCopyrightText: 2026 Jerin Joy
SPDX-FileCopyrightText: 2026 Rivos Inc.

SPDX-License-Identifier: Apache-2.0
-->

# JumpStart Codebase Improvement Plan

> **Revision 2** — Updated 2026-05-30 with dual-agent adversarial review
> findings.  New items added (N1–N13), priorities corrected, effort
> estimates revised, factual errors fixed, and work sequence reordered so
> Path 2 (link-time configuration) comes first.

This document captures a comprehensive review of the JumpStart codebase
(Spring 2026) and lays out a prioritized roadmap of improvements.  It is
intended to be worked through incrementally — each section can be tackled as
an independent work stream.

---

## 0. Architecture Overview (for context)

The project has four major layers:

1.  **Build orchestrator** — `scripts/build_diag.py` (Python).  Parses CLI
    args, resolves environments, spawns parallel Meson builds.
2.  **Build system** — Meson + Cargo.  Meson owns the top-level build; when
    `jumpstart_backend=rust`, Meson shells out to `cargo build` via a
    `custom_target`.
3.  **Source generator** — `scripts/generate_diag_sources.py` (Python).
    Takes a YAML description of the diagnostic (memory map, privilege modes,
    structs, defines) and emits: a generated assembly file (`.S`), a linker
    script (`.ld`), a C header (`.defines.h`), a C data-structures header
    (`.data_structures.h`), and a Rust data-structures file
    (`jumpstart_data_structures.rs`).
4.  **Runtime library** — `src/` (Rust, `#![no_std]`) with a parallel set of
    C sources under `src/common/` for the legacy C backend.  The Rust
    library is compiled to a static archive (`libjumpstart.a`) and linked
    into each diagnostic ELF alongside a separately-compiled assembly
    archive (`libjumpstart_asm.a`) produced by `build.rs`.

---

## 1. Rust Code Quality

### 1A. Unsafe Proliferation — Separate Safe Core from FFI Shims

**Problem:** Almost every Rust function is `#[unsafe(no_mangle)] pub extern "C" fn`.
The code reads like C-in-Rust: raw pointers, manual null checks, C-style
return codes, and pervasive `unsafe` blocks.

**Pattern to follow:** A *safe, idiomatic* Rust core with a thin `extern "C"`
shim layer on top.

```rust
// Safe core — pure Rust, testable on host
fn delay_us(delay_in_useconds: u32) {
    // ... implementation using rdtime + pause ...
}

// FFI shim — only this carries the C ABI baggage
#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub extern "C" fn delay_us_from_smode(delay_in_useconds: u32) {
    delay_us(delay_in_useconds)
}
```

This refactoring should be done **after** Path 2 (item 6CC) because several
modules (`trap.rs`, `uart.rs`, `thread_attr_fns.rs`) are coupled to
`include!`-generated structs.  Refactoring before decoupling would be re-done
after Path 2.

**Effort:** L (4–6 days) — touches every `.rs` file.  Start with one module
(e.g. `time.rs`) as a template, then roll out.

**Affected files:**
- `src/time.rs` — `delay_us_from_smode` / `delay_us_from_mmode` are identical
- `src/utils.rs` — entire m-mode / s-mode duplication
- `src/uart.rs` — `putch`, `puts`, `printk`, `setup_uart`, etc.
- `src/string.rs` — every function
- `src/heap.rs` — `malloc`, `free`, `calloc`, `memalign`, `memset`, `memcpy`

---

### 1B. Massive Code Duplication (m-mode vs s-mode)

**Problem:** `utils.rs` contains two nearly-identical copies of:
- `try_get_seed` (smode + mmode)
- `__random` (smode + mmode)
- `get_random_number_from_*` (smode + mmode)
- `set_random_seed_from_*` (smode + mmode)

The only differences are the `link_section` attribute and which `jumpstart_*_fail`
function is called on error.

**Fix:** Create a single parametric implementation. The mode-dependent
behavior can be injected via a const generic, a function pointer, or a
trait.  Estimated savings: ~100 lines removed.

De-duplication should be done **before** the safe-core extraction (1A)
because extracting one unified implementation is simpler than extracting two
near-identical ones.

**Same pattern in:** `src/time.rs` (`delay_us_from_smode` / `delay_us_from_mmode`).

---

### 1C. `string.rs` is a C Library, Not Rust

**Problem:** `src/string.rs` reimplements `strcpy`, `strcmp`, `strlen`,
`islower`, `isupper`, `tolower`, `toupper` as `extern "C"` functions
operating on raw `*const c_char` pointers.  In `no_std` Rust you would
typically use `core::str` and `core::char` methods.

**Fix:** Implement with safe `core` primitives and keep the `extern "C"`
wrappers only as thin adapters for the C ABI.  For example, `strlen` should
delegate to `CStr::from_ptr(s).to_bytes().len()`.

**Affected file:** `src/string.rs`

---

### 1D. `printk` is a Hand-Rolled `printf`

**Problem:** The `printk` function in `src/uart.rs` manually parses `%s`,
`%d`, `%i`, `%x` format specifiers out of a C string.  This is fragile,
incomplete (no `%p`, `%lu`, `%llx`, width/precision), and duplicates what
`core::fmt` already does.

**Fix:** Keep `printk` as an FFI entry point, but have it copy the format
string into a Rust `&str` (or use `CStr`) and delegate formatting to
`core::fmt::Write` / a custom writer.  This eliminates the manual parser
while preserving the C-callable symbol.  Must preserve exact C ABI semantics
for the 7-vararg calling convention.

**Effort:** M (1 day) — more complex than it appears due to ABI edge cases.

**Affected file:** `src/uart.rs` (function `printk`)

---

### 1E. Error Handling — Adopt `Result` Internally

**Problem:** Every failure path calls `jumpstart_smode_fail()` or
`jumpstart_mmode_fail()`, which immediately aborts.  There is no `Result`
anywhere in the codebase.  While bare-metal code has real constraints,
`Result` is zero-cost when used with `unwrap()` at the FFI boundary.

**Fix:** Use `Result<T, Error>` for internal functions.  Only call
`jumpstart_*_fail()` in the outermost `extern "C"` shim when a result is
`Err`.  This makes the logic testable on the host (`cargo test`) and
composable.

**Note:** `Result` is required before full host-side testing can cover error
paths, but pure-logic functions (bit math, LCG) can be tested without it.
Place 1E alongside 5Y (host tests) in the work sequence.

**Affected files:** `src/trap.rs`, `src/tablewalk.rs`, `src/heap.rs`,
`src/utils.rs`, `src/time.rs`, `src/string.rs`

---

### 1F. Missing `write_csr!` Macro

**Problem:** `read_csr!` exists in `src/lib.rs` but there is no corresponding
`write_csr!` or `set_csr_bits!` / `clear_csr_bits!` macro.

**Fix:** Add:
```rust
macro_rules! write_csr {
    ($csr:ident, $val:expr) => {
        unsafe { core::arch::asm!(
            concat!("csrw ", stringify!($csr), ", {0}"),
            in(reg) $val,
            options(nomem, nostack)
        ) }
    };
}
```

**Affected file:** `src/lib.rs`

---

### 1G. Magic Numbers

**Problem:** Several numeric literals have no name or documentation:

| Location | Value | Should Be |
|---|---|---|
| `src/utils.rs` LCG multiplier | `6364136223846793005` | `const LCG_MULTIPLIER: u64` |
| `src/utils.rs` LCG increment | `1` (implicit in `wrapping_add(1)`) | `const LCG_INCREMENT: u64` |
| `src/utils.rs` seed retry limit | `100` | `const MAX_SEED_CSR_RETRIES: u32` |
| `src/lock.rs` swap value | `1` | `const LOCK_ACQUIRED: u64` |
| `src/cpu_bits.rs` | various bitfields | Should reference RISC-V spec volume & section |

**Affected files:** `src/utils.rs`, `src/lock.rs`, `src/cpu_bits.rs`,
`src/heap.rs`

---

### 1H. Documentation

**Problem:** `src/cpu_bits.rs` has ~150 public constants with zero doc
comments.  Each CSR address, bitfield mask, and exception code should
reference the RISC-V privileged spec.

Other modules (`heap.rs`, `tablewalk.rs`, `trap.rs`) have good *internal*
comments explaining the Rust-to-C mapping, but no proper `///` doc comments
for public API items.

**Fix:** Add `///` doc comments to all `pub` items.  For CSRs, include the
spec reference (e.g. `/// mstatus CSR (RISC-V Privileged Spec §3.1.6)`).
Pair with `#![warn(missing_docs)]` (item 4W) and a `just doc` target (item
N12).

**Affected files:** `src/cpu_bits.rs` (primary), all other `src/*.rs`

---

### 1I. `extern "C"` Declarations Are Scattered

**Problem:** `jumpstart_smode_fail` and `jumpstart_mmode_fail` are declared
in `extern "C"` blocks in both `src/utils.rs` and
`src/thread_attr_fns.rs`.  The `thread_attr_fns.rs` file also declares
~20 additional assembly accessor functions.

**Fix:** Create a single `src/ffi.rs` (or `src/bindings.rs`) module that
contains *all* `extern "C"` declarations.  Every other module imports from
this central location.  This prevents accidental divergence and makes it
obvious exactly which external symbols the Rust code depends on.

**Affected files:** `src/utils.rs`, `src/thread_attr_fns.rs`

---

## 2. Build System

### 2J. Cargo Inside Meson is Brittle

**Problem:** `meson.build` uses a `custom_target` with a raw shell one-liner:

```meson
command : [
    'sh', '-c',
    '(cd ' + meson.project_source_root() + ' && ' +
    'DIAG_YAML=' + diag_yaml_full_path + ' ' + cargo.full_path() +
    ' build ' + cargo_build_flags + ') && ' +
    'cp ... libjumpstart.a @OUTPUT0@ && ' +
    'cp $(find ... -name libjumpstart_asm.a | head -n 1) @OUTPUT1@'
],
```

Issues:
1. Meson cannot track Rust source file changes — it only re-runs when the
   *YAML input* changes.
2. The `find | head -n 1` to locate the assembly archive is fragile (breaks
   if Cargo's output directory structure changes).
3. Error messages from `cargo build` are interleaved with Meson output and
   hard to read.

**Note:** If Path 2 (item 6CC) is implemented first, this `custom_target` is
eliminated entirely — the Rust library compiles once and links per-test,
making a wrapper script unnecessary.  If Path 2 is deferred, promote this
item to P1 and create a proper wrapper script.

**Affected files:** `meson.build` (repeated in two places — diag path and
unit-test path), `tests/meson.build`

---

### 2K. Two Sources of Truth for Compiler Flags

**Problem:** The RISC-V ISA string appears in two places with *different values*:

| File | Value |
|---|---|
| `build.rs` | `rv64gcv_zbb_zbs_zihintpause` |
| `cross_compile/public/gcc_options.txt` | `rv64gcvh_zba_zbb_zbs_zihintpause` |

`build.rs` is missing `h` (hypervisor) and `zba` (address-generation
extension).  The assembly archive compiled by `build.rs` may produce subtly
incorrect code for hypervisor or address-generation instructions.  This is a
**correctness bug**, not just cleanup.

**Fix:** Define the ISA string in a single file (e.g. a TOML config or the
top-level `Cargo.toml` metadata) and have both `build.rs` and
`gcc_options.txt` read from it.  Alternatively, have `build.rs` read
`gcc_options.txt`.

**Affected files:** `build.rs`, `cross_compile/public/gcc_options.txt`

---

### 2L. Per-Diagnostic Cargo Rebuild (Path 1 vs Path 2)

**Problem:** Every diagnostic triggers a full `cargo build` because the
`DIAG_YAML` environment variable changes, and `build.rs` uses `include!` to
pull generated Rust structs into the compiled code.  For 45 tests this means
45 full library compilations.

The migration plan (Phase 12 / "Future Build System Evolution") already
identifies the solution: **Path 2 — link-time configuration.**  Instead of
`include!`-ing generated structs at compile time, the Rust library exposes
`extern "C"` symbols that the diagnostic's generated assembly (or a thin C
shim) defines at link time.

**Note:** Path 2 must also address the **assembly archive** (`libjumpstart_asm.a`).
The `build.rs` compiles ~11 assembly files with `-include` flags that inject
per-diagnostic generated headers.  The assembly archive is *also* per-diagnostic.
See item N5 (Path 2 design document) for full coupling analysis.

This is the single highest-leverage build improvement available.  Effort is
**XL (2–3 weeks)** — see item 6CC for full scope breakdown.

**Affected files:** `build.rs`, `src/trap.rs`, `src/uart.rs`,
`src/thread_attr_fns.rs`, `meson.build`

---

### 2M. `justfile` Lacks Explicit Rust Shortcut Recipes

**Problem:** The `justfile` actually **does** default `backend="rust"` — so
`just test gcc release spike` already uses the Rust backend.  The real issue
is discoverability: there are no explicit `test-rust`, `build-rust`, or
`test-all-rust` recipes to make the Rust path obvious.

**Fix:** Add explicit shortcut recipes:
```just
test-rust compiler buildtype target: (test compiler buildtype target "rust")
build-rust compiler buildtype target: (build compiler buildtype target "rust")
```

**Affected file:** `justfile`

---

### 2N. Meson Has No Insight Into Rust Dependency Graph

**Problem:** The `custom_target` for Rust lists all `.rs` files as `input`,
but Meson can't parse Rust `mod` / `use` statements.  Changing `trap.rs`
won't trigger a rebuild unless the YAML input is also touched.

**Fix (short term):** Use `depfile` support in `custom_target` — have the
wrapper script emit a Make-format depfile listing every `.rs` file that
`cargo` compiles (obtainable via `cargo metadata` or `--emit=dep-info`).

**Note:** This is a band-aid.  Path 2 (item 6CC) eliminates the per-diag
`custom_target` entirely and is the real fix.

**Affected files:** `meson.build`, `tests/meson.build`

---

## 3. Python Code Quality

### 3O. `generate_diag_sources.py` is a 1643-Line God Class

**Problem:** The `SourceGenerator` class handles ~20 distinct concerns:
YAML parsing, memory map processing, address assignment, page table
creation, linker script generation, C defines, C data structures, Rust data
structures, stack generation, CPU sync functions, MMU functions, assembly
file generation, thread attributes, register context save/restore, C struct
defines, C struct data structures, offsetof assertions, assembly code for C
structs, and translation.

**Fix:** Extract separate classes, each in its own file under
`scripts/generators/`:

| Proposed file | Responsibility |
|---|---|
| `generators/linker_script.py` | Linker script generation |
| `generators/defines.py` | C/Rust constant generation |
| `generators/data_structures.py` | C struct + Rust struct generation |
| `generators/assembly.py` | Assembly file generation |
| `generators/stack.py` | Stack layout code |
| `generators/thread_attributes.py` | Thread attribute getters |
| `generators/reg_context.py` | Register save/restore |
| `generators/mmu.py` | MMU/page-table functions |
| `generators/cpu_sync.py` | CPU synchronization primitives |

The main script would then orchestrate these generators rather than
containing all the logic.

**Effort:** XL (2–3 weeks).  The class has shared parsed state (YAML tree,
memory map, address assignments); extraction requires designing a shared
context object and defining interfaces between generators.  Must also verify
output is byte-identical before and after.

**Warning:** Do not split before golden-file tests (5AA) are in place, or
you'll be refactoring untested code.

**Affected file:** `scripts/generate_diag_sources.py`

---

### 3P. `build_diag.py` `main()` is Too Large

**Problem:** The `main()` function does argument parsing, manifest
construction, filtering logic, backward-compatibility handling, and
orchestration all in one ~200-line function.

**Fix:** Extract:
- `parse_args()` — pure argument parsing
- `build_manifest_from_dirs(dir_list)` — convert `--diag_src_dir` to a YAML manifest
- `filter_manifest(manifest, include, exclude)` — apply include/exclude
- `run(factory, environment)` — the main orchestration block

**Affected file:** `scripts/build_diag.py`

---

### 3Q. `__pycache__` Directories in Repo

**Problem:** The `.gitignore` lists `__pycache__` but several
`scripts/**/__pycache__` directories appear in the repository listing.

**Fix:** Run `git rm -r` on the cache dirs and verify `.gitignore` is
working.  Consider adding `**/__pycache__/` to be explicit.  This is a
30-second fix — just do it.

---

### 3R. No Python Tests

**Problem:** There are zero tests for any of the Python modules.  The code
generator (`generate_diag_sources.py`) correctly generates assembly and
linker scripts that are critical to correctness, but there are no automated
checks that, e.g., "given this YAML, the linker script should contain these
sections."

**Fix:** Add `pytest` tests for:
- `dict_utils.py` — `override_dict`, `create_dict`
- `bitfield_utils.py` — `extract_bits`, `place_bits`, `find_lowest_set_bit`
- `cstruct.py` — field offset/size calculation
- `memory_mapping.py` — `MemoryMapping` field parsing and validation
- `page_tables.py` — PTE address calculation, page table walking
- `SourceGenerator` — integration tests with known YAML inputs and
  expected output fragments (linker script sections, define values)

**Effort:** L (2–3 weeks).  Writing tests for an untested 1643-line god class
is itself a significant effort.  Consider writing characterization tests
(snapshot current output) before refactoring (3O).  Add `pytest` to
`pyproject.toml` dev-dependencies.

**Affected files:** All `scripts/**/*.py`, `pyproject.toml`

---

### 3S. Import Ordering Inconsistency

**Problem:**
- `build_diag.py` has `import yaml` in the middle of `main()` (line ~195)
- `meson.py` does `sys.path.append` between standard library imports and
  local imports
- `build_diag.py` runs `sys.path.append` in `main()` to add the `scripts/`
  directory

**Fix:**
- All imports at the top of the file, in three blocks: stdlib, third-party,
  local.
- Add `scripts/` to `PYTHONPATH` in `pyproject.toml` or use a proper
  package install (`pip install -e .`) so `sys.path` manipulation isn't
  needed at runtime.
- The `import yaml` inside `main()` should be at the top of the file.

**Affected files:** `scripts/build_diag.py`, `scripts/build_tools/meson.py`

---

## 4. Project Structure

### 4T. Dead/Broken C Files Alongside Rust Equivalents

**Problem:** The migration plan says "DO NOT remove any existing C files,"
which is correct *during migration*.  But now that Phases 1–11 are marked
COMPLETED, the C originals for migrated modules are dead code when
`jumpstart_backend=rust`:

| C file | Rust replacement | Status |
|---|---|---|
| `src/common/uart.smode.c` | `src/uart.rs` | Migrated |
| `src/common/lock.smode.c` | `src/lock.rs` | Migrated |
| `src/common/lock.mmode.c` | `src/lock.rs` | Migrated |
| `src/common/heap.smode.c` | `src/heap.rs` | Migrated |
| `src/common/tablewalk.smode.c` | `src/tablewalk.rs` | Migrated |
| `src/common/trap_handler.mmode.c` | `src/trap.rs` | Migrated |
| `src/common/trap_handler.smode.c` | `src/trap.rs` | Migrated |
| `src/common/time.smode.c` | `src/time.rs` | Migrated |
| `src/common/time.mmode.c` | `src/time.rs` | Migrated |
| `src/common/utils.smode.c` | `src/utils.rs` | Migrated |
| `src/common/utils.mmode.c` | `src/utils.rs` | Migrated |
| `src/common/thread_attributes.mmode.c` | `src/thread_attr_fns.rs` | Migrated |
| `src/common/thread_attributes.smode.c` | `src/thread_attr_fns.rs` | Migrated |
| `src/common/string.smode.c` | `src/string.rs` | Migrated |

**Fix:** Gate deletion on cross-backend validation (item N8).  Once all 45
tests pass with identical output on both backends, delete these C files and
their corresponding headers from `include/common/`.  The C backend build in
`src/common/meson.build` would shrink dramatically.

**Warning:** Some C files in the tree (e.g. `uart.mmode.c`) were never
claimed as "migrated" and should not be deleted until their Rust equivalents
exist.

**Affected files:** `src/common/*.c`, `include/common/*.h`,
`src/common/meson.build`

---

### 4U. Empty `test_rust_000` Directory

**Problem:** `tests/common/test_rust_000/` exists but is empty.  This
suggests an intent to write a Rust-native diagnostic test that was never
completed.

**Fix:** Either write a minimal Rust diagnostic (e.g. "hello world" that
exercises `println!` and exits with `DIAG_PASSED`) or remove the empty
directory.

**Affected path:** `tests/common/test_rust_000/`

---

### 4V. No `rustfmt.toml`

**Problem:** There is no project-level Rust formatting configuration.  The
C code uses `.clang-format`, the Python code uses `black` (configured in
`pyproject.toml`), but Rust has nothing.

**Fix:** Add a `rustfmt.toml` with project conventions.  Minimal example:
```toml
edition = "2024"
max_width = 100
use_small_heuristics = "Max"
```

**Affected files:** None (new file: `rustfmt.toml`)

---

### 4W. No Clippy Configuration

**Problem:** No lints configured.  Specifically missing: `unsafe_op_in_unsafe_fn`,
which would catch unsafe operations inside unsafe functions that aren't
explicitly marked, and `undocumented_unsafe_blocks`, which would require
`// SAFETY:` comments on every unsafe block.

**Fix:** Add to `src/lib.rs`:
```rust
#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::undocumented_unsafe_blocks)]
```

The `#![forbid(unsafe_op_in_unsafe_fn)]` lint should be added **immediately**
(P0) as a safety guardrail — it enforces the safe/unsafe boundary that item
1A is trying to create.  It cannot be turned off later with `#[allow]` (unlike
`#[deny]`), making it the right choice for bare-metal safety-critical code.

Add to `.cargo/config.toml`:
```toml
[target.riscv64gc-unknown-none-elf]
rustflags = ["-C", "target-feature=+zihintpause"]
```

**Affected files:** `src/lib.rs`, `.cargo/config.toml`

---

### 4X. Duplicated Constants Between C Headers and Rust

**Problem:** `include/common/cpu_bits.h` and `src/cpu_bits.rs` define the
same CSR addresses, bit masks, and exception codes.  Any change requires
updating both files.  There are **three** sources of truth when the YAML
defines section is also counted.

**Fix:** Both could be generated from a shared source of truth (the
`defines` section of `jumpstart_public_source_attributes.yaml`).  The Python
generator already emits both C and Rust defines — the remaining constants
in `cpu_bits.{h,rs}` could be moved there.

**Affected files:** `include/common/cpu_bits.h`, `src/cpu_bits.rs`,
`src/public/jumpstart_public_source_attributes.yaml`

---

## 5. New Items from Review

### N1. Missing `rust-toolchain.toml` (P0, XS)

The project uses Rust 2024 edition `#[unsafe()]` attribute syntax and targets
`riscv64gc-unknown-none-elf`.  Without a pinned toolchain file, different
developers and CI will use different nightly/stable versions, leading to
non-reproducible builds.

**Fix:** Create `rust-toolchain.toml`:
```toml
[toolchain]
channel = "nightly-2026-05-01"   # or whichever is known to work
targets = ["riscv64gc-unknown-none-elf"]
```

---

### N2. `#![forbid(unsafe_op_in_unsafe_fn)]` — Immediate Add (P0, XS)

This is a safety enforcement mechanism that directly addresses the problems
described in 1A.  Without it, unsafe functions can contain unmarked unsafe
operations — exactly the pattern this plan criticizes.  Add it immediately,
**then** fix the resulting compilation errors.  Acts as a guardrail during
the safe-core extraction.

**Related:** See item 4W for full clippy configuration.

---

### N3. CI Pipeline (P0, M)

The plan discusses tests (3R, 5Y, 5AA) but never mentions when or how they
run.  A CI pipeline should enforce:
1. `cargo test --target=x86_64-unknown-linux-gnu` (once 5Y is implemented)
2. `pytest` (once 3R is implemented)
3. `cargo clippy --target=riscv64gc-unknown-none-elf`
4. `cargo fmt --check`
5. `just test-all` on Spike (or a subset for PR velocity)

Without CI, regressions in refactored code go unnoticed.

**Affected files:** `.github/workflows/` (new)

---

### N4. Correctness Audit — Fix `gettimeofday` Bug (P0, XS)

**Problem:** `src/time.rs::gettimeofday` computes:

```rust
let microseconds = timer_ticks / CPU_CLOCK_FREQUENCY_IN_MHZ;
```

This sets `tv_usec` to the **total** microsecond count, not the fractional
remainder.  POSIX requires `tv_usec` to be in range `[0, 999999]`.  The
correct computation is:

```rust
let microseconds = (timer_ticks / CPU_CLOCK_FREQUENCY_IN_MHZ) % 1_000_000;
```

Beyond this specific bug, do a targeted audit of all exported FFI functions
for semantic deviations from the C originals before any refactoring.

**Affected file:** `src/time.rs`

---

### N5. Path 2 Design Document (P0, S)

Before writing any Path 2 code, produce a design document that enumerates
**all** compile-time coupling points:

- **Rust `include!` × 3:** `uart.rs`, `trap.rs`, `thread_attr_fns.rs` each
  `include!` the generated data structures file
- **Assembly `-include` × ~11:** `build.rs` passes `-include` flags to
  `cc::Build` when compiling `.S` files (`.defines.h`, `.data_structures.h`)
- **`DIAG_YAML` env var:** drives the entire generation pipeline

The design must answer:
1. What `extern "C"` symbols will the Rust library expose?
2. What generated assembly/C shim defines them per-diagnostic?
3. How does `trap_override_attributes` (a struct with fixed-size arrays
   whose dimensions are generated constants) work at link time?
4. Does the assembly archive need to become generic too, or is its
   per-diagnostic compilation fast enough to ignore?

---

### N6. Panic/Allocator Circular Dependency Documentation (P0, XS)

**Problem:** If the heap lock is held or the heap is corrupted when a panic
occurs, any `println!` in the panic path will attempt to allocate (via
`format!`) and deadlock or double-fault.  The current panic handler uses
`uart_write_str_direct()` to bypass the UART lock, which is correct, but
this design choice is not documented.

**Fix:** Add a block comment in `src/lib.rs` above the panic handler
explaining why direct UART writes are used.  Consider a runtime guard:
`static HEAP_INITIALIZED: AtomicBool` that skip all formatting/allocation in
panic paths.

---

### N7. Host-Testability `#[cfg]` Strategy (P1, S)

**Problem:** Item 5Y proposes host-side unit tests with
`cargo test --target=x86_64-unknown-linux-gnu`, but several modules use
RISC-V inline assembly (`rdtime`, `csrrw`, `pause`) that won't compile on
x86.  Generated constants (`UART_BASE_ADDRESS`) via `include!` also won't
exist.

**Fix:** Document a strategy:
- Extract pure-logic functions (bit math, LCG, heap chunk splitting) into
  ISA-independent modules or paths.
- Use `#[cfg(target_arch = "riscv64")]` gating on functions with inline
  assembly, with host-side mocks under `#[cfg(not(target_arch = "riscv64"))]`.
- Gate `include!` lines with `#[cfg(not(test))]` and provide test stubs.

---

### N8. Cross-Backend Validation (P1, M)

**Problem:** The plan proposes deleting C files (4T) and making Rust the
default backend (6BB) without any verification that the Rust backend
produces identical behavior to the C backend.

**Fix:** Run the full 45-test suite with both backends:
```sh
./scripts/build_diag.py --build_manifest ... --jumpstart_backend c  --environment spike
./scripts/build_diag.py --build_manifest ... --jumpstart_backend rust --environment spike
```
Diff the Spike output (or exit codes) for each test.  This gates 4T and 6BB.

---

### N9. `include!` Consolidation (P1, S)

**Problem:** Three modules (`uart.rs`, `trap.rs`, `thread_attr_fns.rs`) each
`include!` the same generated file.  This means the generated structs and
constants are re-parsed and re-instantiated in every module, creating
duplicate symbol definitions and making the per-diagnostic coupling implicit.

**Fix:** Consolidate to a single inclusion point — either in `lib.rs` or a
dedicated `src/generated.rs` module that re-exports all symbols.  This makes
the coupling explicit in one place and is a stepping stone toward Path 2.

---

### N10. Heap Initialization Contract + Guard (P2, S)

**Problem:** `HEAPS.setup_heap()` must be called before any `Box::new()`,
`format!()`, or `Vec::push()`.  If any code path allocates before heap
initialization, it's undefined behavior.  This contract is not documented or
enforced at runtime.

**Fix:**
1. Document the contract in a `// SAFETY:` comment on `HEAPS`.
2. Add `static HEAP_INITIALIZED: AtomicBool` and check it in the
   `GlobalAlloc::alloc` implementation with a clear panic message if
   violated.  See also item N6 (panic/allocator circular dependency).

---

### N11. Struct Layout Verification (P2, M)

**Problem:** Rust structs marked `#[repr(C)]` cross the FFI boundary
(`MemChunk`, `HeapInfo`, `timeval`, `BitRange`, `TranslationInfo`,
`trap_override_attributes`, `thread_attributes`), but there is no automated
verification that their layout matches the C/assembly expectations.  A field
reordering or type mismatch could cause silent heap corruption or incorrect
trap dispatch.

**Fix:** Add compile-time assertions:
```rust
const _: () = assert!(core::mem::size_of::<thread_attributes>() == EXPECTED_SIZE);
const _: () = assert!(core::mem::offset_of!(thread_attributes, cpu_id) == 0);
```
Where `EXPECTED_SIZE` comes from the generated defines or C headers.

---

### N12. `cargo doc` / `just doc` Target (P2, XS)

**Problem:** If `#![warn(missing_docs)]` is enabled (4W) and `///` comments
are added (1H), there should be a way to view the generated documentation.

**Fix:** Add to `justfile`:
```just
doc:
    cargo doc --target=riscv64gc-unknown-none-elf --open
```

---

### N13. `Cargo.lock` Policy (P3, XS)

**Problem:** The plan doesn't state whether `Cargo.lock` should be committed.
For a `staticlib` crate that produces a binary artifact (linked into ELFs),
this is an application — `Cargo.lock` should be committed to ensure
reproducible builds.

**Fix:** Ensure `Cargo.lock` is tracked in git.  Verify `.gitignore` doesn't
exclude it.

---

## 6. Testing Infrastructure

### 5Y. No Rust-Side Unit Tests

**Problem:** `#[cfg(test)]` modules don't exist anywhere in the Rust source.
Functions like `heap::test_allocation()` exist as regular `pub` functions
rather than proper `#[test]` functions.  Pure-logic functions (bit
manipulation, LCG, string operations) could be tested on the host with:

```
cargo test --target=x86_64-unknown-linux-gnu
```

Candidates for immediate host-side testing:
- `src/cpu_bits.rs` — `bit()`, `bit_mask()`, `get_field()`, `set_field()`,
  `align_up_size()`
- `src/heap.rs` — chunk splitting, coalescing, iterator (with a
  pre-allocated buffer)
- `src/lock.rs` — `Spinlock` acquire/release (single-threaded tests)
- `src/string.rs` — all functions, with known inputs/outputs
- `src/utils.rs` — `extract_bits`, `place_bits`, LCG sequence

**Prerequisite:** See N7 (host-testability `#[cfg]` strategy) for how to
make inline-assembly and `include!`-dependent modules compile on x86.

**Fix:** Add `#[cfg(test)] mod tests { ... }` to each of the above modules.
Pair with 1E (Result) to enable testing error paths.

**Affected files:** `src/cpu_bits.rs`, `src/heap.rs`, `src/lock.rs`,
`src/string.rs`, `src/utils.rs`, `src/lib.rs` (to add `#[cfg(test)]`
support)

---

### 5Z. Test Definitions Outgrowing Meson Arrays

**Problem:** Tests are defined as raw Meson arrays in
`tests/common/meson.build`:

```meson
start_in_smode_tests += [
  ['test000', 'Enable MMU (SATP.mode = sv39), jump to main and exit.'],
  ['test001', 'Enable MMU (SATP.mode = sv48), jump to main and exit.'],
  ...
]
```

Adding a new test requires editing a Meson file and remembering the
implicit positional format (index 0 = name, 1 = description, 2 = spike
args, 3 = expected to fail).

**Fix:** Define tests in a YAML (or TOML) registry file:

```yaml
tests:
  - name: test000
    description: "Enable MMU (SATP.mode = sv39), jump to main and exit."
    start_mode: smode
  - name: test012
    description: "Exit with DIAG_FAILED to test fail path"
    start_mode: smode
    expected_to_fail: true
```

Have `tests/common/meson.build` (or a Python preprocessor) read this file
and generate the Meson test arrays.  This is also a step toward making the
test registry language-agnostic (it could drive both C and Rust test
execution).

**Note:** Design this **after** Path 2 (item 6CC) because the registry schema
may need to include per-diagnostic link-time data fields.

**Effort:** L (4–6 days) — schema design, Meson consumer, 45 test migrations,
`build_diag.py` compatibility.

**Affected files:** `tests/common/meson.build` (consumer), new YAML file

---

### 5AA. No Verification of Generated Code

**Problem:** The Python code generator has no automated verification.
Regressions in linker script generation, struct layout, or define values are
only caught when a diagnostic fails on Spike.

**Fix:** Add golden-file tests: run the generator with known input YAML and
compare output against committed "expected" files.  Run these as part of CI
(or at least `just test-generated`).

**Effort:** L (3–5 days) — depends on 3O (split SourceGenerator) or at
minimum requires characterization tests (snapshot current output) before the
refactoring begins.

**Affected files:** `scripts/generate_diag_sources.py`, CI configuration

---

## 7. Migration Roadmap Concerns

### 6BB. Phase 12 ("Integrated Meson + Cargo") — Completing the Integration

**Problem:** The `jumpstart_backend` Meson option exists, `build_diag.py`
accepts `--jumpstart_backend rust`, and `meson.build` has Rust support.  But
the integration still feels bolted-on:
- `build_diag.py` defaults to `--jumpstart_backend c`
- No explicit `just` shortcut recipes for the Rust backend
- The Rust `custom_target` is duplicated between the diag and unit-test
  paths in Meson

**Fix:** Once cross-backend validation (N8) passes:
1. Change `build_diag.py` default to `--jumpstart_backend rust`
2. Add `just test-rust` / `build-rust` shortcut recipes (2M)
3. Deduplicate the Rust `custom_target` in Meson

**Affected files:** `scripts/build_diag.py`, `justfile`, `meson.build`,
`tests/meson.build`

---

### 6CC. Path 2 (Link-Time Configuration) — Full Scope

**Problem:** Path 1 ("recompile Rust per diagnostic") works but is slow and
complex.  Path 2 ("compile Rust once, link with per-test data") is
described in the migration plan but not started.

**Full scope of Path 2 (2–3 week effort):**

1. **Audit all per-diagnostic data used by Rust** — currently spread across
   `include!` in `uart.rs`, `trap.rs`, `thread_attr_fns.rs`, plus the
   assembly `-include` compilation.  Identify every symbol.
2. **Analyze the assembly archive** — determine whether the ~11 `.S` files
   also need to become link-time generic, or whether their per-diagnostic
   compilation is fast enough to leave as-is (item N5).
3. **Convert Rust `include!` to `extern "C"`** — the `trap_override_attributes`
   struct is the hardest case because it contains fixed-size arrays whose
   dimensions are generated constants.  Either export with maximum dimensions
   or redesign the API.
4. **Generate per-diagnostic C/assembly shims** that define the `extern`
   symbols — modify `generate_diag_sources.py` to emit a new output file.
5. **Integrate the shims into the Meson link step** — the shim object file
   must be linked into each diagnostic ELF.
6. **Test all 45 diagnostics** — one at a time, verifying link-time
   configuration works for each.

**Benefits of Path 2:**
- Single `cargo build` for all tests — 45x speedup for full test runs
- Meson `custom_target` only runs once (or when Rust sources change)
- Cleaner separation: Rust library is generic, test-specific data is in
  assembly/C shims
- Enables incremental Rust compilation with proper dep-tracking

**Affected files:** `build.rs`, `src/trap.rs`, `src/uart.rs`,
`src/thread_attr_fns.rs`, `meson.build`, `scripts/generate_diag_sources.py`

---

## 8. Prioritized Roadmap

Items are ordered within each priority level.  Each item lists the
estimated effort and the files affected.

### P0 — Immediate (correctness, safety, foundations)

| # | Item | Est. | Description |
|---|---|---|---|
| N1 | `rust-toolchain.toml` | XS | Pin toolchain for reproducible builds |
| N2 | `#![forbid(unsafe_op_in_unsafe_fn)]` | XS | Safety guardrail — add before any refactoring |
| 2K | Unify ISA string | XS | Fix `h`/`zba` mismatch — correctness bug |
| N4 | Fix `gettimeofday` bug | XS | `tv_usec` modulo fix + targeted FFI audit |
| 3Q | Clean `__pycache__` | XS | `git rm -r` cache dirs |
| N5 | Path 2 design document | S | Enumerate all coupling points, incl. assembly archive |
| N6 | Panic/allocator doc + guard | XS | Document why direct UART writes are used |
| N3 | CI pipeline | M | `.github/workflows/` for tests, clippy, fmt |
| 1I | Centralize FFI declarations | S | Single `src/ffi.rs` for all `extern "C"` blocks |

### P1 — Foundation (enables other work)

| # | Item | Est. | Description |
|---|---|---|---|
| N5→6CC | Implement Path 2 | XL | Compile Rust once, link per-test (2–3 weeks) |
| N9 | `include!` consolidation | S | Single `generated.rs` module — stepping stone to Path 2 |
| N7 | Host-testability `#[cfg]` strategy | S | Document how to make modules compile on x86 |
| 5Y | Rust unit tests | M | `#[cfg(test)]` modules for pure logic functions |
| 1E | Error handling with `Result` | M | Internal `Result`, abort only at FFI boundary |
| 1B | De-duplicate m/smode | S | Single implementation for seed/random/delay |
| 1A | Safe core + FFI shims (template) | L | Start with one module, then roll out |
| N8 | Cross-backend validation | M | Run all 45 tests, diff C vs Rust output |
| 5Z | YAML test registry | L | Move test definitions out of Meson arrays |

### P2 — Quality and maintainability

| # | Item | Est. | Description |
|---|---|---|---|
| 1C | `string.rs` rewrite | S | Use `core` primitives, keep FFI shims |
| 1D | `printk` rewrite | M | Delegate to `core::fmt` — preserve vararg ABI |
| 1F | `write_csr!` macro | S | Add to `src/lib.rs` |
| 1G | Name magic numbers | S | Constants for LCG params, retry limits, etc. |
| N10 | Heap initialization guard | S | Runtime check + documented contract |
| N11 | Struct layout verification | M | Compile-time offset/size assertions |
| 2M | `just` Rust shortcut recipes | S | Add `test-rust` / `build-rust` recipes |
| N12 | `cargo doc` / `just doc` target | XS | Generate and view docs |
| 1H | Documentation | M | `///` comments on all `pub` items |
| 3P | Refactor `build_diag.py` main() | S | Extract helper functions |

### P3 — Polish and long-term health

| # | Item | Est. | Description |
|---|---|---|---|
| 6BB | First-class Rust backend | S | Change defaults after cross-backend validation |
| 4T | Delete dead C files | S | After N8 verification passes |
| 4U | `test_rust_000` | S | Write or delete the empty directory |
| 4V | `rustfmt.toml` | XS | Add formatting config |
| 3R | Python tests | L | `pytest` tests for code generator modules |
| 3O | Split SourceGenerator | XL | Break up the god class (2–3 weeks) |
| 5AA | Golden-file tests | L | Verify generator output against expected files |
| 4W | Clippy (remaining lints) | XS | Already added `forbid(unsafe_op)`, add rest |
| 2N | Meson dep-tracking | S | Only if Path 2 doesn't eliminate custom_target |
| 4X | Deduplicate constants | L | Generate `cpu_bits.rs` from YAML defines |
| 3S | Import ordering | XS | Fix import organization in Python files |
| N13 | `Cargo.lock` policy | XS | Commit `Cargo.lock` |

### Effort key:
- **XS** — < 30 minutes
- **S** — 1–3 hours
- **M** — 1–2 days
- **L** — 4–6 days
- **XL** — 2–3 weeks

---

## 9. Suggested Work Sequence

1.  **P0 quick wins:** N1 (toolchain) + N2 (`forbid(unsafe)`) + 2K (ISA string) + N4 (`gettimeofday` fix) + 3Q (`__pycache__`)
2.  **P0 design:** N5 (Path 2 design doc, incl. assembly-archive coupling analysis) + N6 (panic/allocator doc)
3.  **P0 infra:** N3 (CI pipeline) + 1I (centralize FFI declarations)
4.  **P1 architecture:** N5→6CC (implement Path 2 — compile once, link per-test)
5.  **P1 consolidation:** N9 (`include!` consolidation — stepping stone or done as part of Path 2)
6.  **P1 testing:** N7 (host-testability strategy) → 5Y (host unit tests) + 1E (Result) + 1B (de-duplicate m/smode)
7.  **P1 refactoring:** 1A (safe core template on one module, then rollout)
8.  **P1 validation:** N8 (cross-backend: run all 45 tests, diff C vs Rust output)
9.  **P1 registries:** 5Z (YAML test registry, post-Path-2 schema)
10. **P2 rollout:** 1C → 1D → 1F → 1G → N10 → N11 → 2M → N12 → 1H → 3P
11. **P3 polish:** 6BB → 4T → 4U → 4V → 3R → 3O → 5AA → 4W → 2N → 4X → 3S → N13
