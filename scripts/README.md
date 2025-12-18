# Documentation Generator Scripts

This directory contains Rust-based tools for generating documentation.

## generate-docs.rs

Converts Markdown documentation to HTML without requiring external dependencies like Pandoc.

### Features

- ✅ Pure Rust implementation using `pulldown-cmark`
- ✅ No external dependencies (no Python, no Pandoc, no Node.js)
- ✅ Works in CI/CD environments
- ✅ Supports GitHub Flavored Markdown extensions:
  - Tables
  - Strikethrough
  - Task lists
  - Footnotes
  - Heading attributes

### Usage

```bash
# Run from project root
cd scripts
cargo run --bin generate-docs

# Or via Makefile
make docs-guides
```

### How it Works

1. Reads Markdown files from `docs/` directory
2. Parses Markdown using `pulldown-cmark`
3. Converts to HTML
4. Wraps in HTML template with navigation and styling
5. Outputs to `target/doc/guides/`

### Benefits over Pandoc

- **No system dependencies**: Works on any platform with Rust
- **Faster**: Native Rust performance
- **CI-friendly**: No apt-get install required
- **Consistent**: Same output across all platforms
- **Customizable**: Easy to modify HTML templates

### Files Generated

- `target/doc/guides/developer_guide.html`
- `target/doc/guides/user_guide.html`
- `target/doc/guides/README.html`
- `target/doc/guides/quick_reference.html`
- `target/doc/guides/NAVIGATION.html`
- `target/doc/guides/docs-style.css` (copied)

### Dependencies

Only one dependency:
- `pulldown-cmark` - CommonMark + GFM Markdown parser

### Fallback Chain

The Makefile uses a three-tier fallback system:

1. **Primary**: Rust-based converter (this tool)
2. **Secondary**: Pandoc (if installed)
3. **Tertiary**: Copy raw Markdown files

This ensures documentation is always available, regardless of the environment.

