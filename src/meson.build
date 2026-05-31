# SPDX-FileCopyrightText: 2023 - 2026 Rivos Inc.
#
# SPDX-License-Identifier: Apache-2.0

jumpstart_sources = []

if get_option('rivos_internal_build') == true
  jumpstart_source_attributes_yaml = files('rivos_internal/jumpstart_rivos_internal_source_attributes.yaml')
else
  jumpstart_source_attributes_yaml = files('public/jumpstart_public_source_attributes.yaml')
endif

mmode_sources = []
smode_sources = []
umode_sources = []

subdir('common')

if get_option('rivos_internal_build') == true
  subdir('rivos_internal')
else
  subdir('public')
endif

if get_option('jumpstart_backend') == 'c'
  if 'mmode' in riscv_priv_modes_enabled
    jumpstart_sources += mmode_sources
  endif
  if 'smode' in riscv_priv_modes_enabled
    jumpstart_sources += smode_sources
  endif
  if 'umode' in riscv_priv_modes_enabled
    jumpstart_sources += umode_sources
  endif
endif
