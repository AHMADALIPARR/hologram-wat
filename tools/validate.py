#!/usr/bin/env python3
# Copyright (C) 2026 Ahmad Ali Parr
# SPDX-License-Identifier: LicenseRef-Sovereign-Leviathan-SL-AGPL3-001
"""Validate all WAT files compile to WASM."""
import glob
import pathlib
import sys

try:
    from wasmtime import Store, Module
except ImportError:
    print("wasmtime not installed: pip install wasmtime")
    sys.exit(1)

ROOT = pathlib.Path(__file__).resolve().parents[1]
failed = 0

for wat_file in sorted((ROOT / "wat").glob("*.wat")):
    store = Store()
    try:
        Module(store.engine, wat_file.read_text())
        print(f"✓ {wat_file.name}")
    except Exception as e:
        print(f"✗ {wat_file.name}: {e}")
        failed += 1

if failed:
    sys.exit(1)
print(f"\nAll {len(list((ROOT / 'wat').glob('*.wat')))} WAT files valid.")
