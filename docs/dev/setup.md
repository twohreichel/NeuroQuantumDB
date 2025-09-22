# Entwicklungsumgebung Setup

## Voraussetzungen

### System-Requirements
- **Rust**: 1.70+ (neueste stabile Version empfohlen)
- **Git**: Für Versionskontrolle
- **Docker**: Für Containerisierung und Tests
- **Make**: Für Build-Automatisierung

### Hardware-Empfehlungen
- **CPU**: Multi-Core (ARM64 oder x86_64)
- **RAM**: 8GB+ für vollständige Builds
- **Storage**: 10GB+ freier Speicherplatz

## Entwicklungsumgebung einrichten

### 1. Repository klonen

```bash
git clone https://github.com/neuroquantumdb/neuroquantumdb.git
cd neuroquantumdb
```

### 2. Rust-Toolchain installieren

```bash
# Rust installieren (falls noch nicht installiert)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Benötigte Komponenten installieren
rustup component add rustfmt clippy
rustup target add aarch64-unknown-linux-gnu

# Zusätzliche Tools installieren
cargo install cargo-audit cargo-deny cargo-machete cargo-tarpaulin
```

### 3. Entwicklungsabhängigkeiten installieren

```bash
# System-Pakete (Ubuntu/Debian)
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# macOS (mit Homebrew)
brew install openssl pkg-config

# Entwicklung-spezifische Tools
cargo install mdbook mdbook-linkcheck mdbook-mermaid
```

### 4. Git Hooks einrichten

```bash
# Pre-commit Hooks installieren
chmod +x hooks/*
cp hooks/* .git/hooks/

# Oder mit dem Setup-Script
./scripts/setup-dev.sh
```

## Development Workflow

### Build und Test

```bash
# Entwicklungsbuild
make dev

# Vollständige Tests ausführen
make test

# Linting und Code-Qualität
make lint

# Alle Checks (wie in CI)
make ci
```

### Code-Qualitätsstandards

#### Formatierung
```bash
# Code automatisch formatieren
make format

# Formatierung prüfen
make format-check
```

#### Linting
```bash
# Clippy-Analyse
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Security Audit
cargo audit

# Dependency-Check
cargo deny check
```

### Debugging

#### Lokale Entwicklung
```bash
# Debug-Build mit zusätzlichen Features
cargo build --features debug-synaptic,neuromorphic,quantum

# Mit Debug-Logging starten
RUST_LOG=debug cargo run --bin neuroquantum-api
```

#### Tests debuggen
```bash
# Einzelnen Test mit Output
cargo test test_name -- --nocapture

# Integration Tests
cargo test --test integration_tests

# Benchmark Tests
cargo bench
```

## Projektstruktur verstehen

### Workspace-Layout
```
neuroquantumdb/
├── crates/
│   ├── neuroquantum-core/     # Kern-Funktionalität
│   ├── neuroquantum-qsql/     # Query-Sprache
│   └── neuroquantum-api/      # REST API & Server
├── tests/                     # Integration Tests
├── docs/                      # Dokumentation
├── config/                    # Konfigurationsdateien
└── scripts/                   # Build & Setup Scripts
```

### Modul-Abhängigkeiten
```
neuroquantum-api
    ├── neuroquantum-core
    │   ├── quantum.rs         # Quantum Optimizations
    │   ├── synaptic.rs        # Neuromorphic Computing
    │   ├── plasticity.rs      # Learning Algorithms
    │   └── security.rs        # Security Layer
    └── neuroquantum-qsql
        ├── parser.rs          # Query Parser
        ├── optimizer.rs       # Query Optimizer
        └── natural_language.rs # NL Processing
```

## Code-Konventionen

### Rust-Stil
- **Naming**: `snake_case` für Funktionen/Variablen, `PascalCase` für Typen
- **Dokumentation**: Alle öffentlichen APIs müssen dokumentiert sein
- **Error Handling**: Verwende `Result<T, E>` und `anyhow` für Fehlerbehandlung
- **Async**: Verwende `tokio` für asynchrone Operationen

### Dokumentation
```rust
/// Führt eine neuromorphe Abfrage aus.
///
/// # Arguments
/// * `query` - Die QSQL-Abfrage
/// * `plasticity_level` - Neuroplastizitätslevel (0.0-1.0)
///
/// # Examples
/// ```
/// let result = execute_neuromorphic_query("SELECT * FROM users", 0.8).await?;
/// ```
///
/// # Errors
/// Gibt einen `QueryError` zurück wenn die Abfrage syntaktisch ungültig ist.
pub async fn execute_neuromorphic_query(
    query: &str,
    plasticity_level: f64,
) -> Result<QueryResult, QueryError> {
    // Implementation...
}
```

### Testing-Standards
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_neuromorphic_query_execution() {
        // Arrange
        let query = "SELECT * FROM test_table";
        let expected_plasticity = 0.5;

        // Act
        let result = execute_neuromorphic_query(query, expected_plasticity).await;

        // Assert
        assert!(result.is_ok());
        let query_result = result.unwrap();
        assert!(query_result.plasticity_applied > 0.0);
    }
}
```

## Performance Profiling

### Memory Profiling
```bash
# Mit Valgrind (Linux)
valgrind --tool=massif target/debug/neuroquantum-api

# Memory-optimierte Build
RUSTFLAGS="-C opt-level=3 -C lto=fat" cargo build --release
```

### CPU Profiling
```bash
# Mit perf (Linux)
cargo build --release
perf record --call-graph=dwarf target/release/neuroquantum-api
perf report

# Flamegraph generieren
cargo install flamegraph
cargo flamegraph --bin neuroquantum-api
```

### Benchmarking
```bash
# Micro-benchmarks
cargo bench

# Load-testing
hey -n 10000 -c 100 http://localhost:8080/api/v1/health
```

## Debugging-Tools

### Logging
```bash
# Strukturierte Logs mit tracing
RUST_LOG="neuroquantum_core=debug,neuroquantum_api=info" cargo run

# JSON-Logs für Produktion
RUST_LOG_FORMAT=json cargo run
```

### Tracing
```rust
use tracing::{info, debug, error, instrument};

#[instrument]
async fn process_quantum_query(query: &str) -> Result<QueryResult, Error> {
    debug!("Processing quantum query: {}", query);
    
    let result = quantum_processor.execute(query).await;
    
    match result {
        Ok(r) => {
            info!("Query processed successfully, {} results", r.count());
            Ok(r)
        }
        Err(e) => {
            error!("Query processing failed: {}", e);
            Err(e)
        }
    }
}
```

## IDE-Konfiguration

### VS Code
Empfohlene Extensions:
- `rust-analyzer`: Rust Language Server
- `CodeLLDB`: Debugging
- `Better TOML`: Konfigurationsdateien
- `Error Lens`: Inline Fehleranzeige

`.vscode/settings.json`:
```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.features": "all",
    "editor.formatOnSave": true
}
```

### IntelliJ IDEA / CLion
- **Rust Plugin** installieren
- **Cargo Integration** aktivieren
- **Live Templates** für häufige Patterns einrichten

## Continuous Integration

### Lokale CI-Simulation
```bash
# Alle CI-Checks lokal ausführen
make ci

# Einzelne CI-Jobs simulieren
make lint          # Linting
make test          # Tests
make security      # Security Audit
```

### Branch-Protection
Vor dem Commit prüfen:
1. ✅ Alle Tests bestehen
2. ✅ Linting ohne Warnings
3. ✅ Security Audit erfolgreich
4. ✅ Dokumentation aktuell

## Nächste Schritte

Nach dem Setup:
1. [Architektur verstehen](./architecture.md)
2. [Core Module erkunden](./core.md)
3. [Testing-Strategien lernen](./testing.md)
4. [Ersten Beitrag leisten](./contributing.md)
