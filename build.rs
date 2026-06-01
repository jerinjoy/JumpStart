/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Tell Cargo to re-run this script if any of these files change
    println!("cargo:rerun-if-changed=src/common/jumpstart.mmode.S");
    println!("cargo:rerun-if-changed=src/public/jumpstart_public_source_attributes.yaml");
    println!("cargo:rerun-if-changed=scripts/generate_diag_sources.py");

    // Tell Cargo to rebuild if the environment variable changes
    println!("cargo:rerun-if-env-changed=DIAG_YAML");

    // OUT_DIR is a temporary folder Cargo gives us to put generated files
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let defines_file = out_dir.join("jumpstart_defines.h");
    let data_structures_file = out_dir.join("jumpstart_data_structures.h");
    let rust_data_structures_file = out_dir.join("jumpstart_data_structures.rs");

    // Read the DIAG_YAML environment variable.
    // We expect this to be set by the caller (like the validation script).
    let diag_yaml = env::var("DIAG_YAML").expect(
        "DIAG_YAML environment variable not set. Please provide the path to a diagnostic attributes YAML file."
    );
    println!("cargo:rerun-if-changed={}", diag_yaml);

    // 1. Run your existing Python script to generate the missing headers!
    let status = Command::new("python3")
        .arg("scripts/generate_diag_sources.py")
        .arg("--diag_attributes_yaml")
        .arg(&diag_yaml)
        .arg("--jumpstart_source_attributes_yaml")
        .arg("src/public/jumpstart_public_source_attributes.yaml")
        .arg("--priv_modes_enabled")
        .arg("mmode")
        .arg("smode")
        .arg("umode")
        .arg("--output_defines_file")
        .arg(&defines_file)
        .arg("--output_data_structures_file")
        .arg(&data_structures_file)
        .arg("--output_rust_data_structures_file")
        .arg(&rust_data_structures_file)
        .status()
        .expect("Failed to execute python script");

    assert!(status.success(), "Python script failed!");

    // 2. Compile the assembly file
    cc::Build::new()
        .compiler("riscv64-unknown-elf-gcc")
        // We need the Vector (v) extension for the vsetivli instruction!
        .flag("-march=rv64gcvh_zba_zbb_zbs_zihintpause")
        .flag("-mabi=lp64d")
        .flag("-mcmodel=medany")
        // Tell gcc to include the headers we just dynamically generated
        .flag("-include")
        .flag(defines_file.to_str().unwrap())
        .flag("-include")
        .flag(data_structures_file.to_str().unwrap())
        .file("src/common/jumpstart.mmode.S")
        .file("src/common/jumpstart.smode.S")
        .file("src/common/jumpstart.vsmode.S")
        .file("src/common/jumpstart.umode.S")
        .file("src/common/jumpstart.vumode.S")
        .file("src/common/data.privileged.S")
        .file("src/common/heap.smode.S")
        .file("src/public/init.mmode.S")
        .file("src/public/exit.mmode.S")
        .file("src/public/jump_to_main.mmode.S")
        .include("include/common")
        .compile("jumpstart_asm");
}
