# Contributing Guide

Vielen Dank für Ihr Interesse, zu NeuroQuantumDB beizutragen! Dieser Guide hilft Ihnen beim Einstieg.

## Code of Conduct

Wir erwarten von allen Mitwirkenden, dass sie unseren [Code of Conduct](https://github.com/neuroquantumdb/neuroquantumdb/blob/main/CODE_OF_CONDUCT.md) befolgen.

## Entwicklungsworkflow

### 1. Repository Setup
```bash
# Fork das Repository auf GitHub
# Dann klonen Sie Ihren Fork
git clone https://github.com/IHR_USERNAME/neuroquantumdb.git
cd neuroquantumdb

# Upstream Remote hinzufügen
git remote add upstream https://github.com/neuroquantumdb/neuroquantumdb.git

# Entwicklungsumgebung einrichten
make dev
./scripts/setup-dev.sh
```

### 2. Branch-Strategie
```bash
# Für neue Features
git checkout -b feature/beschreibung-des-features

# Für Bugfixes
git checkout -b fix/beschreibung-des-bugs

# Für Dokumentation
git checkout -b docs/beschreibung-der-docs
```

### 3. Entwicklung
```bash
# Code-Qualität prüfen vor Commit
make pre-commit

# Tests ausführen
make test

# Dokumentation aktualisieren
make docs
```

## Beitrag-Arten

### 🐛 Bug Reports
Verwenden Sie das [Bug Report Template](https://github.com/neuroquantumdb/neuroquantumdb/issues/new?template=bug_report.md):

**Erforderliche Informationen:**
- NeuroQuantumDB Version
- Betriebssystem und Architektur (ARM64/x86_64)
- Reproduktionsschritte
- Erwartetes vs. tatsächliches Verhalten
- Relevante Logs

### ✨ Feature Requests
Verwenden Sie das [Feature Request Template](https://github.com/neuroquantumdb/neuroquantumdb/issues/new?template=feature_request.md):

**Erforderliche Informationen:**
- Problem/Bedarf Beschreibung
- Vorgeschlagene Lösung
- Alternativen
- Zusätzlicher Kontext

### 🔧 Code Contributions

#### Pull Request Prozess
1. **Issue erstellen**: Beschreiben Sie Ihre geplante Änderung
2. **Fork & Branch**: Erstellen Sie einen Feature-Branch
3. **Entwickeln**: Implementieren Sie Ihre Änderung
4. **Testen**: Stellen Sie sicher, dass alle Tests bestehen
5. **Dokumentieren**: Aktualisieren Sie relevante Dokumentation
6. **PR erstellen**: Verwenden Sie das PR Template

#### Code-Standards

##### Rust Code Style
```rust
// ✅ Gut: Dokumentierte öffentliche Funktion
/// Führt eine neuromorphe Abfrage mit Plastizität aus.
///
/// # Arguments
/// * `query` - Die QSQL-Abfrage als String
/// * `plasticity_level` - Plastizitätslevel zwischen 0.0 und 1.0
///
/// # Returns
/// * `Result<QueryResult, QueryError>` - Abfrageergebnis oder Fehler
///
/// # Examples
/// ```
/// let result = execute_neuromorphic_query(
///     "SELECT * FROM users WHERE age > 25",
///     0.8
/// ).await?;
/// ```
pub async fn execute_neuromorphic_query(
    query: &str,
    plasticity_level: f64,
) -> Result<QueryResult, QueryError> {
    // Implementation...
}

// ❌ Schlecht: Undokumentierte Funktion
pub async fn do_stuff(s: &str, f: f64) -> Result<Thing, Error> {
    // ...
}
```

##### Error Handling
```rust
// ✅ Gut: Strukturierte Fehlerbehandlung
#[derive(Debug, thiserror::Error)]
pub enum NeuromorphicError {
    #[error("Invalid plasticity level: {level}. Must be between 0.0 and 1.0")]
    InvalidPlasticityLevel { level: f64 },
    
    #[error("Synaptic learning failed: {reason}")]
    LearningFailure { reason: String },
    
    #[error("Quantum optimization error: {0}")]
    QuantumError(#[from] QuantumError),
}

// ❌ Schlecht: Generische Fehler
pub fn some_function() -> Result<(), String> {
    Err("Something went wrong".to_string())
}
```

##### Testing Standards
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_neuromorphic_query_with_valid_plasticity() {
        // Arrange
        let query = "SELECT * FROM test_data WHERE value > 10";
        let plasticity = 0.5;
        
        // Act
        let result = execute_neuromorphic_query(query, plasticity).await;
        
        // Assert
        assert!(result.is_ok());
        let query_result = result.unwrap();
        assert!(query_result.plasticity_applied >= 0.0);
        assert!(query_result.results.len() > 0);
    }

    #[tokio::test]
    async fn test_neuromorphic_query_with_invalid_plasticity() {
        // Arrange
        let query = "SELECT * FROM test_data";
        let invalid_plasticity = 1.5; // > 1.0
        
        // Act
        let result = execute_neuromorphic_query(query, invalid_plasticity).await;
        
        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), 
            NeuromorphicError::InvalidPlasticityLevel { level } if level == 1.5));
    }
}
```

### 📖 Dokumentation

#### API Dokumentation
- Alle öffentlichen Funktionen müssen dokumentiert sein
- Beispiele für komplexe APIs bereitstellen
- Parameter und Rückgabewerte erklären

#### Benutzer-Dokumentation
- Schreiben Sie in klarem, verständlichem Deutsch
- Verwenden Sie praktische Beispiele
- Berücksichtigen Sie verschiedene Erfahrungslevel

#### Code-Kommentare
```rust
// ✅ Gut: Erklärt das "Warum"
// Wir verwenden Hebbian Learning hier, da es die synaptische
// Plastizität besser modelliert für zeitlich korrelierte Daten
if temporal_correlation > threshold {
    apply_hebbian_learning(&mut weights);
}

// ❌ Schlecht: Erklärt das "Was" (offensichtlich)
// Addiere 1 zu counter
counter += 1;
```

## Spezielle Bereiche

### Neuromorphic Computing
Wenn Sie an neuromorphen Features arbeiten:

```rust
// Plastizität-Tests erfordern spezielle Considerations
#[tokio::test]
async fn test_synaptic_plasticity_convergence() {
    let mut network = SynapticNetwork::new();
    let training_data = generate_test_patterns();
    
    // Trainiere das Netzwerk
    for pattern in training_data {
        network.learn_pattern(&pattern).await?;
    }
    
    // Teste Konvergenz
    let final_weights = network.get_synaptic_weights();
    assert!(weights_converged(&final_weights), 
           "Synaptic weights did not converge after training");
}
```

### Quantum Computing
Für Quantum-Optimierungen:

```rust
#[cfg(feature = "quantum-simulation")]
#[tokio::test]
async fn test_quantum_superposition_query() {
    // Simuliere Quantum-Zustand für Tests
    let quantum_state = QuantumState::superposition(vec![
        QuantumBit::new(0.707, 0.707), // |+⟩ state
        QuantumBit::new(1.0, 0.0),     // |0⟩ state
    ]);
    
    let result = execute_quantum_query(&quantum_state).await?;
    assert!(result.coherence_maintained);
}
```

### ARM64 Optimierungen
NEON-optimierte Funktionen testen:

```rust
#[cfg(target_arch = "aarch64")]
#[test]
fn test_neon_vector_operations() {
    let vector_a = vec![1.0, 2.0, 3.0, 4.0];
    let vector_b = vec![2.0, 3.0, 4.0, 5.0];
    
    let result = unsafe { neon_vector_multiply(&vector_a, &vector_b) };
    let expected = vec![2.0, 6.0, 12.0, 20.0];
    
    assert_eq!(result, expected);
}
```

## Review-Prozess

### Code Review Checkliste
- [ ] Code folgt Rust-Standards
- [ ] Alle Tests bestehen
- [ ] Dokumentation ist aktuell
- [ ] Performance-Impact bewertet
- [ ] Security-Aspekte berücksichtigt
- [ ] ARM64-Kompatibilität getestet

### Performance Benchmarks
```bash
# Vor und nach Änderungen
make benchmark

# Memory-Profiling
make memory-profile

# ARM64-spezifische Tests
cargo test --target aarch64-unknown-linux-gnu
```

## Release-Prozess

### Semantic Versioning
- **MAJOR**: Breaking Changes
- **MINOR**: Neue Features (rückwärtskompatibel)
- **PATCH**: Bugfixes

### Changelog-Format
```markdown
## [1.2.0] - 2024-01-15

### Added
- 🧠 Neue Hebbian Learning Algorithmen
- ⚛️ Quantum Entanglement für korrelierte Abfragen
- 📊 Erweiterte Monitoring-Metriken

### Changed
- 🚀 Performance-Optimierungen für ARM64
- 📝 Verbesserte Dokumentation

### Fixed
- 🐛 Memory-Leak in Synaptic Network
- 🔒 Security-Issue in Authentifizierung

### Deprecated
- ⚠️ Alte API wird in v2.0 entfernt
```

## Hilfe bekommen

### Community Channels
- **GitHub Discussions**: Allgemeine Fragen und Diskussionen
- **Issues**: Bug Reports und Feature Requests
- **Discord**: Real-time Community Chat (Link in README)

### Mentorship
Neue Contributor erhalten Unterstützung durch:
- Assigned Mentors für große Features
- Code Review Guidelines
- Pair Programming Sessions (auf Anfrage)

### Good First Issues
Suchen Sie nach Issues mit den Labels:
- `good first issue`: Für Newcomer geeignet
- `help wanted`: Community-Hilfe erwünscht
- `documentation`: Dokumentationsverbesserungen

## Anerkennungen

Alle Mitwirkenden werden in:
- `CONTRIBUTORS.md` gelistet
- GitHub Contributors Graph
- Release Notes erwähnt

Vielen Dank für Ihre Beiträge zu NeuroQuantumDB! 🎉
