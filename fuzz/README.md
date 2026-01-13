# NeuroQuantumDB Fuzzing

This directory contains fuzz targets for testing critical components of NeuroQuantumDB using cargo-fuzz and libFuzzer.

## Prerequisites

Install cargo-fuzz (requires nightly Rust):

```bash
rustup install nightly
cargo +nightly install cargo-fuzz
```

## Fuzz Targets

### Core Targets

#### QSQL Parser (`fuzz_qsql_parser`)

Tests the QSQL parser with arbitrary input strings to find parsing bugs, crashes, and edge cases in SQL and neuromorphic/quantum query syntax.

```bash
cd fuzz
cargo +nightly fuzz run fuzz_qsql_parser
```

#### QSQL Tokenizer (`fuzz_qsql_tokenizer`)

Specifically tests the tokenization phase of the QSQL parser with various byte patterns and SQL-like prefixes.

```bash
cd fuzz
cargo +nightly fuzz run fuzz_qsql_tokenizer
```

#### DNA Encoder (`fuzz_dna_encoder`)

Tests the DNA quaternary encoder with arbitrary binary data to find encoding/compression bugs.

```bash
cd fuzz
cargo +nightly fuzz run fuzz_dna_encoder
```

#### DNA SIMD (`fuzz_dna_simd`)

Tests the SIMD-optimized DNA encoding/decoding functions to ensure memory safety and correctness.

```bash
cd fuzz
cargo +nightly fuzz run fuzz_dna_simd
```

### Extended Targets (Issue #262)

#### QSQL Functions (`fuzz_qsql_functions`)

Tests QSQL-specific neuromorphic and quantum extensions including:
- NEUROMATCH pattern matching
- QUANTUM_SEARCH with Grover's algorithm
- HEBBIAN_LEARNING weight adaptation
- SYNAPTIC_OPTIMIZE operations
- QUANTUM_JOIN entanglement operations

```bash
cd fuzz
cargo +nightly fuzz run fuzz_qsql_functions
```

#### REST API JSON (`fuzz_api_json`)

Tests API JSON payload parsing and validation for all endpoints:
- SQL query requests
- Table creation/modification
- Quantum search requests
- DNA compression requests
- Neural network training
- Biometric authentication

```bash
cd fuzz
cargo +nightly fuzz run fuzz_api_json
```

#### DNA Roundtrip (`fuzz_dna_roundtrip`)

Tests DNA compression/decompression roundtrip integrity:
- Verifies data preservation through encode/decode cycles
- Tests various compression configurations
- Validates error correction capabilities
- Edge case patterns (all zeros, all ones, repeated patterns)

```bash
cd fuzz
cargo +nightly fuzz run fuzz_dna_roundtrip
```

#### WebSocket Messages (`fuzz_websocket`)

Tests WebSocket message parsing and handling:
- Subscribe/Unsubscribe messages
- Query streaming messages
- Ping/Pong heartbeats
- Error responses
- Channel message broadcasting

```bash
cd fuzz
cargo +nightly fuzz run fuzz_websocket
```

## Running All Fuzz Targets

To run all fuzz targets for a short smoke test:

```bash
cd fuzz
for target in fuzz_qsql_parser fuzz_qsql_tokenizer fuzz_dna_encoder fuzz_dna_simd \
              fuzz_qsql_functions fuzz_api_json fuzz_dna_roundtrip fuzz_websocket; do
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
│   ├── fuzz_dna_simd/
│   ├── fuzz_qsql_functions/
│   ├── fuzz_api_json/
│   ├── fuzz_dna_roundtrip/
│   └── fuzz_websocket/
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

The fuzzing targets are integrated into the CI pipeline via GitHub Actions.
See `.github/workflows/fuzzing.yml` for the configuration.

Example CI configuration:

```yaml
fuzzing:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@nightly
    - run: cargo +nightly install cargo-fuzz
    - name: Run fuzz targets
      run: |
        cd fuzz
        for target in fuzz_qsql_parser fuzz_qsql_functions fuzz_api_json \
                      fuzz_dna_roundtrip fuzz_websocket; do
          cargo +nightly fuzz run $target -- -max_total_time=300
        done
```

## Local Development

### Quick Smoke Test

Run a quick 10-second test on each target:

```bash
make fuzz-smoke
```

### Extended Fuzzing Session

Run fuzzing for 10 minutes per target:

```bash
make fuzz-extended
```

### Single Target

Run a specific fuzz target:

```bash
cd fuzz
cargo +nightly fuzz run fuzz_qsql_functions -- -max_total_time=600
```

## Security

Fuzzing is critical for finding security vulnerabilities. Report any crashes that could lead to:
- Memory corruption
- Denial of service (infinite loops, excessive memory usage)
- Information disclosure
- Logic errors that bypass security checks

See [SECURITY.md](../SECURITY.md) for reporting procedures.
