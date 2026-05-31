<!--
SPDX-FileCopyrightText: 2023 - 2026 Rivos Inc.

SPDX-License-Identifier: Apache-2.0
-->

# JumpStart

Bare-metal kernel, APIs and build infrastructure for writing directed diags for RISC-V CPU/SoC validation.

## Backends

JumpStart currently has two backends:

| Backend | Status |
|---|---|
| **Rust** | Default. The actively developed backend. All new features land here first. |
| **C** | Legacy. Being phased out in favor of the Rust backend. Still functional and available via explicit opt-in. |

The C backend will eventually be removed once the Rust backend has full parity and has proven stable. New diags should target the Rust backend.

## Setup the Environment

JumpStart requires the following tools to be available in your path:
* [meson](https://mesonbuild.com)
* [riscv-gnu-toolchain](https://github.com/riscv-collab/riscv-gnu-toolchain)
* [Spike](https://github.com/riscv-software-src/riscv-isa-sim)
* [just](https://github.com/casey/just) (command runner)
* [Rust](https://www.rust-lang.org) with the `riscv64gc-unknown-none-elf` target (required for the default Rust backend)

Install the Rust RISC-V target:
```shell
rustup target add riscv64gc-unknown-none-elf
```

### Ubuntu

Install required packages:
```shell
# gcc toolchain
# Install riscv-gnu-toolchain from source or use a prebuilt version

# just tool
curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to /usr/local/bin

# meson
sudo apt install meson

```

#### Build Spike from source

See: https://github.com/riscv-software-src/riscv-isa-sim

### macOS

```
brew tap riscv-software-src/riscv
brew install riscv-gnu-toolchain just meson
```

#### Build Spike from source

See: https://github.com/riscv-software-src/riscv-isa-sim

## Test the Environment

This will build JumpStart with the **Rust** backend (default) and run the unit tests.

```shell
just test gcc release spike
```

To see all the possible test targets, run:

```shell
just --list
```

### Using the C Backend

The C backend is still available but must be explicitly selected. To build and test with the C backend, pass `c` as the final argument:

```shell
# Unit tests with the C backend
just test gcc release spike c

# Individual diag with the C backend
scripts/build_diag.py --diag_src_dir tests/common/test053 --diag_build_dir /tmp/diag --environment spike --jumpstart_backend c
```

## Building and Running Diags

The [`scripts/build_diag.py`](scripts/build_diag.py) script provides an easy way to build and run diags on different environments.

This will build the diag in the [`tests/common/test000`](tests/common/test000) using the `gcc` toolchain and the **Rust** backend (default) and run it on the `spike` environment:

```shell
❯ scripts/build_diag.py --diag_src_dir tests/common/test000/ --diag_build_dir /tmp/diag --environment spike
INFO: [ThreadPoolExecutor-0_0]: Compiling 'tests/common/test000/'
INFO: [ThreadPoolExecutor-1_0]: Running diag 'tests/common/test000/'
INFO: [MainThread]:
Summary
Build root: /tmp/diag
Build Repro Manifest: /tmp/diag/build_manifest.repro.yaml
┏━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ Diag                  ┃ Build        ┃ Run [spike]  ┃ Result                        ┃
┡━━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┩
│ tests/common/test000/ │ PASS (2.20s) │ PASS (0.20s) │ /tmp/diag/test000/test000.elf │
└───────────────────────┴──────────────┴──────────────┴───────────────────────────────┘

Diagnostics built: 1
Diagnostics run: 1

Run Manifest:
/tmp/diag/run_manifest.yaml

STATUS: PASSED
```

For more details, check the Reference Manual section on [Building and Running Diags](docs/reference_manual.md#building-and-running-diags).

## Documentation

* [Quick Start: Anatomy of a Diag](docs/quick_start_anatomy_of_a_diag.md)
* [Reference Manual](docs/reference_manual.md)
* [FAQs](docs/faqs.md)
* [JumpStart Internals](docs/jumpstart_internals.md)

## Support

For help, please send a message on the Slack channel #jumpstart-directed-diags-framework.
