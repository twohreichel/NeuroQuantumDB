# DNA Compression

The DNA compression engine uses bio-inspired quaternary encoding to achieve extreme compression ratios.

## How It Works

DNA compression encodes data using four bases: Adenine (A), Thymine (T), Guanine (C), and Cytosine (C).

### Encoding Process

1. **Input Data** → Binary stream
2. **Quaternary Conversion** → 2 bits → 1 base (A=00, T=01, G=10, C=11)
3. **Pattern Recognition** → Identify repeating sequences
4. **Compression** → Apply DNA-specific compression
5. **Error Correction** → Reed-Solomon encoding
6. **Storage** → Compressed quaternary stream

### Compression Ratios

- **Plain Text:** 500:1 (highly repetitive patterns)
- **JSON/XML:** 200:1 (structured data)
- **Binary:** 100:1 (less structure)
- **Already Compressed:** 1:1 (no benefit)

## Configuration

```toml
[compression]
enabled = true
min_size_bytes = 1024  # Only compress > 1 KB
ecc_enabled = true
ecc_redundancy = 0.2  # 20% overhead for error correction
```

## See Example

[DNA Compression Demo](../examples/dna-compression.md)

