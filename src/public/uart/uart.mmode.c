/*
 * SPDX-FileCopyrightText: 2025 - 2026 Rivos Inc.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include "jumpstart.h"
#include <inttypes.h>

void setup_uart(void);
void m_mark_uart_as_enabled(void);

__attr_mtext void m_putch(char c) {
  *(volatile char *)UART_BASE_ADDRESS = c;
}

__attr_mtext void m_setup_uart(void) {
  m_mark_uart_as_enabled();
}
