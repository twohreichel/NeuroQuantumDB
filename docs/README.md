# NeuroQuantumDB Documentation

This directory contains the source files for the NeuroQuantumDB user documentation, built with [mdBook](https://rust-lang.github.io/mdBook/).

## Structure

```
docs/
├── SUMMARY.md              # Table of contents
├── introduction.md         # Introduction page
├── getting-started/        # Installation and quick start guides
├── architecture/           # System architecture documentation
├── api-reference/          # REST API, QSQL, WebSocket reference
├── deployment/             # Docker, Raspberry Pi, monitoring setup
├── examples/               # Code examples and tutorials
├── operations/             # Performance tuning, security, troubleshooting
├── development/            # Contributing, testing, benchmarking
└── reference/              # Configuration, error codes, glossary, FAQ
```

## Building the Documentation

### Prerequisites

- Install mdBook:
  ```bash
  cargo install mdbook
  ```

### Build

```bash
# Build user documentation
make docs-user

# Build API documentation (Rust docs)
make docs-api

# Build both
make docs
```

### Serve Locally

```bash
# Start local server with live-reload
make docs-serve

# Or manually
mdbook serve --open
```

The documentation will be available at `http://localhost:3000`.

### Output

Built documentation is generated in:
- **User Docs:** `target/book/`
- **API Docs:** `target/doc/`

## GitHub Pages Deployment

Documentation is automatically deployed to GitHub Pages on push to `main` branch via `.github/workflows/docs.yml`.

- **User Documentation:** https://yourusername.github.io/NeuroQuantumDB/
- **API Documentation:** https://yourusername.github.io/NeuroQuantumDB/api/

## Contributing to Documentation

### Adding a New Page

1. Create a new `.md` file in the appropriate directory
2. Add it to `SUMMARY.md` in the correct section
3. Build and preview locally: `make docs-serve`
4. Commit and push

### Documentation Standards

- Use clear, concise language
- Include code examples with syntax highlighting
- Add cross-references to related sections
- Include troubleshooting tips where relevant
- Keep technical accuracy high - all examples should be runnable

### Example Page Structure

```markdown
# Page Title

Brief introduction explaining what this page covers.

## Section 1

Content with code examples:

\`\`\`bash
# Example command
cargo build --release
\`\`\`

## Section 2

More content...

## Next Steps

- Link to [Related Topic](./related.md)
- Link to [Another Topic](../other/topic.md)
```

## Maintenance

### Checking for Broken Links

```bash
make docs-check
```

### Cleaning Build Artifacts

```bash
make docs-clean
```

## License

Documentation is licensed under MIT License - see [LICENSE](../LICENSE) for details.

