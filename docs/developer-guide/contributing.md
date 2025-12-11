# Contributing

## Getting Started

1. Fork the repository
2. Clone your fork
3. Create a feature branch

```bash
git checkout -b feature/my-feature
```

## Development Workflow

```bash
# Make changes
vim src/my_file.rs

# Format
cargo fmt

# Lint
cargo clippy --all-targets

# Test
cargo test

# Commit
git commit -m "feat: add my feature"
```

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation |
| `refactor` | Code refactoring |
| `test` | Adding tests |
| `perf` | Performance improvement |

Examples:
```
feat: add DNA compression for text data
fix: resolve buffer overflow in B+Tree
docs: update API reference
```

## Pull Request Process

1. Update documentation
2. Add tests for new features
3. Ensure CI passes
4. Request review

## Code Style

- Use `rustfmt` defaults
- Document public APIs
- Add `# Safety` for unsafe blocks
- Prefer `Result` over `panic!`

## Testing Requirements

- Unit tests for new functions
- Integration tests for features
- Benchmark for performance-critical code

## Questions?

- Open a [Discussion](https://github.com/neuroquantumdb/neuroquantumdb/discussions)
- Check existing [Issues](https://github.com/neuroquantumdb/neuroquantumdb/issues)
