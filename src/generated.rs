/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

// Single inclusion point for all generated data structures and constants.
// Previously, three modules (uart.rs, trap.rs, thread_attr_fns.rs) each
// included this file independently, creating duplicate symbol definitions.
// This module consolidates them so the generated code is parsed once.
include!(concat!(env!("OUT_DIR"), "/jumpstart_data_structures.rs"));
