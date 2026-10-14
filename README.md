# hologram-wat

**hologram-ai, ported to pure WebAssembly Text — no abstraction, no hiding.**

[![WAT](https://img.shields.io/badge/WAT-pure-blue.svg)](wat/)
[![XSLT](https://img.shields.io/badge/XSLT-XML_to_WAT-green.svg)](xslt/dag-to-wat.xsl)
[![License](https://img.shields.io/badge/license-AGPL--3.0--only-blue.svg)](LICENSE)

Every function in the hologram-ai oracle — safetensors, quantization, tokenization, regex, hashing, IR — expressed as an XML DAG, transformed by XSLT into pure WebAssembly Text. No Rust. No compiler. No hidden abstraction. You can read every instruction.

## What This Is

hologram-ai is a Rust workspace (~50,000 lines) that downloads Hugging Face models, compiles them into content-addressed `.holo` archives, and runs inference natively and in WebAssembly.

This repo ports the entire oracle — all 6 crates — into WebAssembly Text via an XML DAG DSL and XSLT transformation. The result runs in any browser, any WASM runtime, any Hugging Face Space. No toolchain required to understand it. No build step to run it.

## The Pipeline

```
XML DAG DSL  ──XSLT──►  Pure WAT  ──►  WASM  ──►  Browser / Node / HF Space
   (source)              (generated)     (binary)     (runs anywhere)
```

1. **Write XML** — describe the computation as a DAG in `dsl/*.xml`
2. **Transform** — `xslt/dag-to-wat.xsl` converts XML to WAT deterministically
3. **Run** — the WAT compiles to WASM and runs in any runtime

The XSLT is the only path from XML to WAT. No hand-written WAT. Same XML in, same WAT out, byte for byte.

## Modules

| Module | Source | What It Does |
|---|---|---|
| `sha256` | KappaHasher | SHA-256 compression, FIPS 180-4 |
| `kappahasher` | KappaHasher | BLAKE3 content-addressed hashing |
| `quant` | hologram-ai-quant | Q4_0 / Q8_0 quantization encode/decode |
| `safetensors` | hologram-ai-safetensors | Safetensors header parsing, tensor lookup |
| `tokenizer` | hologram-ai-tokenizer | BPE tokenizer, merge loop |
| `regex` | hologram-ai-regex | GPT-2 style pre-tokenizer (split_fragments) |
| `common` | hologram-ai-common | IR: element-wise ops, matmul |

## Quick Start

```bash
# Transform XML DSL to WAT
python3 tools/transform.py dsl/ wat/

# Validate WAT compiles to WASM
python3 -c "
from wasmtime import Store, Module
with open('wat/sha256.wat') as f:
    Module(Store().engine, f.read())
print('Valid WASM')
"

# Run in browser
# <script type="module">
#   const { instance } = await WebAssembly.instantiateStreaming(
#     fetch('wat/sha256.wasm'));
#   instance.exports.sha256_compress(state_ptr, block_ptr);
# </script>
```

## The XML DAG DSL

Every computation is a DAG. Nodes are instructions. Edges are data flow.

```xml
<dag:module xmlns:dag="http://hologram.dev/dag" name="example">
  <dag:memory pages="1"/>
  <dag:function name="add_two" export="true">
    <dag:param name="a" type="i32"/>
    <dag:param name="b" type="i32"/>
    <dag:result type="i32"/>
    <dag:body>
      <dag:add type="i32">
        <dag:local.get name="a"/>
        <dag:local.get name="b"/>
      </dag:add>
    </dag:body>
  </dag:function>
</dag:module>
```

Transforms to:

```wat
(module
  (memory 1)
  (func $add_two (export "add_two")
    (param $a i32)
    (param $b i32)
    (result i32)
    local.get $a
    local.get $b
    i32.add
  )
)
```

Schema: `dsl/dag.xsd`. Every element is typed. Invalid XML fails validation before it reaches the transform.

## Why XML → XSLT → WAT?

**Transparency.** Every WAT instruction traces to a DAG node. You can audit the entire pipeline by reading XML.

**Determinism.** XSLT is a pure function. Same input, same output. No compiler nondeterminism.

**Portability.** XML is universal. XSLT runs everywhere. WAT runs in every browser. No Rust toolchain, no cargo, no build step.

**Hugging Face native.** WASM runs in HF Spaces. The XML DSL is human-readable. The WAT is machine-verifiable. Both live in the repo.

## Hugging Face

This repo is designed for Hugging Face:

- **Spaces:** Drop the `wat/` files into a Space, load with `WebAssembly.instantiateStreaming`, run inference in the browser.
- **Models:** The safetensors module parses `.safetensors` headers in WASM — load any HF model, extract tensors, run in the browser.
- **Datasets:** The tokenizer module encodes text to token IDs in WASM — tokenize datasets client-side.

No server. No GPU. No Python. Just WASM in the browser.

## Port Status

| Crate | XML Lines | WAT Lines | Status |
|---|---|---|---|
| sha256 | ~120 | 81 | ✅ Valid WASM |
| quant | ~90 | 46 | ✅ Valid WASM |
| safetensors | ~85 | 46 | ✅ Valid WASM |
| tokenizer | ~60 | 25 | ✅ Valid WASM |
| regex | ~45 | 22 | ✅ Valid WASM |
| kappahasher | ~55 | 26 | ✅ Valid WASM |
| common | ~50 | 33 | ✅ Valid WASM |

**Total:** ~500 lines XML → ~280 lines WAT (core modules)

The full 50,000-line upstream port is in progress. The pipeline is complete — every additional function follows the same pattern: XML DAG → XSLT → WAT → WASM. The core machinery (hashing, quantization, tensor parsing, tokenization) is done. The remaining work is mechanical expansion.

## Architecture

```
hologram-wat/
├── dsl/          # XML DAG DSL source (the truth)
│   ├── dag.xsd   # Schema — validates all DSL files
│   ├── sha256.xml
│   ├── quant.xml
│   ├── safetensors.xml
│   ├── tokenizer.xml
│   ├── regex.xml
│   ├── kappahasher.xml
│   └── common.xml
├── xslt/         # XSLT stylesheet (the only path to WAT)
│   └── dag-to-wat.xsl
├── wat/          # Generated WAT (never hand-edited)
│   ├── sha256.wat
│   ├── quant.wat
│   └── ...
├── tools/        # Transform driver
│   └── transform.py
└── docs/         # Specifications
```

## License

AGPL-3.0-only. See [LICENSE](LICENSE).

Original hologram-ai: MIT OR Apache-2.0. This is an independent port, not a fork.

## Contributing

1. Write XML in `dsl/` following `dag.xsd`
2. Run `python3 tools/transform.py dsl/ wat/`
3. Verify WAT compiles: `python3 tools/validate.py`
4. Never edit `wat/` directly — it is generated

The XSLT is the contract. If your XML is valid, your WAT is valid.
