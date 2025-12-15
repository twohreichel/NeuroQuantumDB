# NeuroQuantumDB Fuzzing

This directory contains fuzz targets for testing critical components of NeuroQuantumDB using cargo-fuzz and libFuzzer.

## Prerequisites

Install cargo-fuzz (requires nightly Rust):

```bash
rustup install nightly
cargo +nightly install cargo-fuzz
```

## Fuzz Targets

### QSQL Parser (`fuzz_qsql_parser`)

Tests the QSQL parser with arbitrary input strings to find parsing bugs, crashes, and edge cases in SQL and neuromorphic/quantum query syntax.

```bash
cd fuzz
cargo +nightly fuzz run fuzz_qsql_parser
```

### QSQL Tokenizer (`fuzz_qsql_tokenizer`)

Specifically tests the tokenization phase of the QSQL parser with various byte patterns and SQL-like prefixes.

```bash
cd fuzz
cargo +nightly fuzz run fuzz_qsql_tokenizer
```

### DNA Encoder (`fuzz_dna_encoder`)

Tests the DNA quaternary encoder with arbitrary binary data to find encoding/compression bugs.

```bash
cd fuzz
cargo +nightly fuzz run fuzz_dna_encoder
```

### DNA SIMD (`fuzz_dna_simd`)

Tests the SIMD-optimized DNA encoding/decoding functions to ensure memory safety and correctness.

```bash
cd fuzz
cargo +nightly fuzz run fuzz_dna_simd
```

## Running All Fuzz Targets

To run all fuzz targets for a short smoke test:

```bash
cd fuzz
for target in fuzz_qsql_parser fuzz_qsql_tokenizer fuzz_dna_encoder fuzz_dna_simd; do
    echo "Running $target for 60 seconds..."
    timeout 60 cargo +nightly fuzz run $target -- -max_total_time=60 || true
done
```

## Corpus Management

Each fuzz target maintains a corpus of interesting inputs in the `corpus/` directory:

```
fuzz/
├── corpus/
│   ├── fuzz_qsql_parser/
│   ├── fuzz_qsql_tokenizer/
│   ├── fuzz_dna_encoder/
│   └── fuzz_dna_simd/
└── artifacts/          # Crash-inducing inputs
```

## Minimizing Crashes

When a crash is found, minimize it to find the smallest input that triggers the bug:

```bash
cargo +nightly fuzz tmin fuzz_qsql_parser artifacts/fuzz_qsql_parser/crash-<hash>
```

## Coverage

To generate coverage reports:

```bash
cargo +nightly fuzz coverage fuzz_qsql_parser
```

## CI Integration

Add to your CI pipeline:

```yaml
fuzzing:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: nightly
    - run: cargo +nightly install cargo-fuzz
    - run: cd fuzz && cargo +nightly fuzz run fuzz_qsql_parser -- -max_total_time=300
    - run: cd fuzz && cargo +nightly fuzz run fuzz_dna_simd -- -max_total_time=300
```

## Security

Fuzzing is critical for finding security vulnerabilities. Report any crashes that could lead to:
- Memory corruption
- Denial of service (infinite loops, excessive memory usage)
- Information disclosure
- Logic errors that bypass security checks

See [SECURITY.md](../SECURITY.md) for reporting procedures.
