/*
 * SPDX-FileCopyrightText: 2025 - 2026 Rivos Inc.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include "jumpstart.h"
#include <inttypes.h>

void setup_uart(void);
void mark_uart_as_enabled(void);

__attr_stext void putch(char c) {
  *(volatile char *)UART_BASE_ADDRESS = c;
}

__attr_stext void setup_uart(void) {
  mark_uart_as_enabled();
}
