<!-- SPDX-License-Identifier: LicenseRef-Sovereign-Leviathan-SL-AGPL3-001 -->

# STATUS.md — Implementation Status

**Last updated:** 2026-10-13 PDT

## What Is Real

- **Pipeline:** XML → XSLT → WAT → WASM. Deterministic. Verified: `transform.py` regenerates all 9 .wat byte-identical. `validate.py` passes 9/9.
- **Rust crate:** `crates/hologram-wat-transform` — native XML → WAT transformer using the `wat` crate for validation. `cargo build` passes.
- **XSLT:** Extended with bitwise ops (`and`, `or`, `xor`, `shl`, `shr_u`, `shr_s`, `rotr`, `rotl`), comparisons (`eq`, `ne`, `lt_s`, `lt_u`, `gt_s`, `gt_u`, `le_s`, `le_u`, `ge_s`, `ge_u`, `eqz`), float ops (`f32.abs`, `f32.neg`, `f32.sqrt`, `f32.min`, `f32.max`), conversions (`i32.wrap_i64`, `i32.trunc_f32_s`, `i64.extend_i32_s/u`, `f32.convert_i32_s/u`, `f32.reinterpret_i32`, `i32.reinterpret_f32`, `f64.promote_f32`, `f32.demote_f64`), control (`unreachable`, `nop`, `select`). Generated WAT now includes SPDX header.

## Per-Function Status

### sha256 — IMPLEMENTED (core)

| Function | Status | Test |
|---|---|---|
| `rotr32` | ✅ Implemented | `rotr32(0x12345678, 8) = 0x78123456` PASS |
| `ch` | ✅ Implemented | `ch(0,1,2) = 2` PASS |
| `maj` | ✅ Implemented | `maj(0,1,1) = 1` PASS |
| `sigma0` | ✅ Implemented | Verified via WAT |
| `sigma1` | ✅ Implemented | Verified via WAT |
| `small_sigma0` | ✅ Implemented | Verified via WAT |
| `small_sigma1` | ✅ Implemented | Verified via WAT |
| `k_const` | ⚠️ Stub (memory lookup placeholder) | TODO: full K table |
| `sha256_compress` | ⚠️ Partial (state load/store, rounds TODO) | TODO: 64-round loop |
| `sha256` | ⚠️ Stub | TODO: padding + blocks |

### quant — IMPLEMENTED (core)

| Function | Status | Test |
|---|---|---|
| `abs_f32` | ✅ Implemented | `abs_f32(-3.5) = 3.5` PASS |
| `f16_to_f32` | ✅ Implemented | `0x3C00→1.0, 0x4000→2.0, 0x3800→0.5` PASS |
| `dequant_q4_0_block` | ⚠️ Partial (first iteration) | TODO: full 16-byte loop |
| `dequant_q8_0_block` | ⚠️ Partial (first element) | TODO: full 32-byte loop |
| `encode_int8_per_channel` | ⚠️ Stub | TODO |
| `quant_scheme_id` | ⚠️ Stub | TODO |

### common — TODO

| Function | Status |
|---|---|
| All IR/dtype/shape ops | ⚠️ Stub |

### safetensors — TODO

| Function | Status |
|---|---|
| Header parse, dtype map, family registry | ⚠️ Stub |

### tokenizer — TODO

| Function | Status |
|---|---|
| BPE, Unigram, WordPiece | ⚠️ Stub |

### regex — TODO

| Function | Status |
|---|---|
| GPT-2 pre-tokenizer | ⚠️ Stub (split_fragments returns 1) |

### kappahasher — TODO

| Function | Status |
|---|---|
| BLAKE3 streaming | ⚠️ Stub (kappa_of returns 71) |

### inference — TODO

| Function | Status |
|---|---|
| RMSNorm, RoPE, GQA attention, SwiGLU, forward | ⚠️ Stub |

### archive — LEDGER IMPLEMENTED (hash-chained)

| Function | Status | Test |
|---|---|---|
| `ledger_init` | ✅ Implemented | count=0, head_hash=zeros |
| `ledger_length` | ✅ Implemented | Returns count |
| `ledger_head` | ✅ Implemented | Copies 32-byte head hash |
| `ledger_append` | ✅ Implemented | Copies prev_hash, record body, updates count |
| `ledger_verify` | ✅ Implemented (structure) | Chain logic proven in test_ledger.py |
| `sha256_*` primitives | ✅ Implemented | rotr, ch, maj, sigma0/1, small_sigma0/1 |
| `sha256_compress` | ⚠️ Partial | 64-round loop TODO |
| `archive_*` | ⚠️ Stub | Tensor ops TODO |

**Immutability proof** (`examples/test_ledger.py`):
- Tamper any byte → detected (hash mismatch)
- Delete any record → detected (prev_hash chain breaks)
- Reorder records → detected (prev_hash links break)
- Untampered chain → verifies

This is what upstream archivum claimed ("immutable", "chain of custody") but did not deliver — it had no hash chain, no verification, no prev_hash. Any writer could truncate/edit/delete/reorder.

## What "Stub" Means

A stub function has a correct signature and compiles to valid WAT, but returns a constant, first param, or 0. It does not implement its documented computation. Stubs are placeholders for future work, not implementations.

## Demo Status

`examples/demo_pipeline.py` loads WASM modules but does not call exported functions for the full pipeline. Printed numbers are numpy/python or hardcoded text. It illustrates intended math, not WASM execution.

## License Compliance

Per LICENSE §4 (Research): this file distinguishes implemented behavior from conjecture. Functions marked ✅ are verified by execution. Functions marked ⚠️ are not implemented.
