# Natural Language Interface

## Überblick

NeuroQuantumDB bietet eine leistungsstarke Natural Language Processing (NLP) Pipeline, die es ermöglicht, Datenbankabfragen in natürlicher Sprache zu formulieren. Die Queries werden automatisch in QSQL übersetzt und ausgeführt.

## Architektur

Die NLP-Pipeline besteht aus vier Hauptkomponenten:

### 1. Tokenizer
Zerlegt natürliche Sprache in strukturierte Tokens:
```rust
use neuroquantum_qsql::natural_language::{Tokenizer, RegexTokenizer};

let tokenizer = RegexTokenizer::new()?;
let tokens = tokenizer.tokenize("Show me all sensors where temperature > 25")?;
```

### 2. Intent Classifier
Klassifiziert die Absicht der Query:
```rust
use neuroquantum_qsql::natural_language::{IntentClassifier, PatternIntentClassifier};

let classifier = PatternIntentClassifier::new()?;
let intent = classifier.classify(&tokens, "show me all users")?;
// Intent::Select
```

### 3. Entity Extractor
Extrahiert Named Entities (Tabellen, Spalten, Werte):
```rust
use neuroquantum_qsql::natural_language::{EntityExtractor, RegexEntityExtractor};

let extractor = RegexEntityExtractor::new()?;
let entities = extractor.extract(&tokens, "show sensors where temperature > 25")?;
```

### 4. Query Generator
Generiert QSQL aus Intent und Entities:
```rust
use neuroquantum_qsql::natural_language::{QueryGenerator, QSQLGenerator};

let generator = QSQLGenerator::new()?;
let qsql = generator.generate(&intent, &entities)?;
```

## Verwendung

### Einfache Query
```rust
use neuroquantum_qsql::natural_language::NLQueryEngine;

let engine = NLQueryEngine::new()?;
let qsql = engine.understand_query("Show me all users")?;
// Generiert: SELECT * FROM users
```

### Query mit Filtern
```rust
let qsql = engine.understand_query("Show me all sensors where temperature above 25")?;
// Generiert: SELECT * FROM sensors WHERE temperature > 25
```

### Neuromorphe Queries
```rust
let qsql = engine.understand_query("Find similar patterns using neural matching")?;
// Generiert: NEUROMATCH memories
```

### Quanteninspirierte Queries
```rust
let qsql = engine.understand_query("Quantum search for data")?;
// Generiert: QUANTUM_SEARCH data
```

## Unterstützte Query-Typen

### SELECT-Queries
- "Show me all users"
- "Display all sensors"
- "Find all records"
- "Get all data from users"

### Filtered Queries
- "Show sensors where temperature above 25"
- "Find users with age greater than 30"
- "Get data where status equal to active"

### Neuromorphic Queries
- "Find similar patterns"
- "Match neural patterns"
- "Search with neuromorphic matching"

### Quantum Queries
- "Quantum search for data"
- "Search using quantum algorithms"

### Aggregate Queries
- "Count all users"
- "Sum of all values"
- "Average temperature"

## Unterstützte Operatoren

Die Natural Language Pipeline versteht verschiedene Operator-Formulierungen:

| Natural Language | SQL Operator |
|-----------------|--------------|
| above, greater than | > |
| below, less than | < |
| equal to | = |
| >, <, = | >, <, = |
| >=, <= | >=, <= |
| != | != |

## Erweiterte Features

### Custom Tokenizer
Sie können einen eigenen Tokenizer implementieren:
```rust
use neuroquantum_qsql::natural_language::{Tokenizer, Token, TokenType};

struct MyTokenizer;

impl Tokenizer for MyTokenizer {
    fn tokenize(&self, text: &str) -> QSQLResult<Vec<Token>> {
        // Custom implementation
    }
}
```

### Custom Intent Classifier
Eigene Intent-Klassifikation:
```rust
use neuroquantum_qsql::natural_language::{IntentClassifier, QueryIntent};

struct MLIntentClassifier {
    model: MyMLModel,
}

impl IntentClassifier for MLIntentClassifier {
    fn classify(&self, tokens: &[Token], text: &str) -> QSQLResult<QueryIntent> {
        // ML-based classification
    }
    
    fn confidence(&self, intent: &QueryIntent, tokens: &[Token]) -> f32 {
        // Confidence score
    }
}
```

### Custom Entity Extractor
Eigene Entity-Extraktion:
```rust
use neuroquantum_qsql::natural_language::{EntityExtractor, Entity};

struct NEREntityExtractor {
    ner_model: MyNERModel,
}

impl EntityExtractor for NEREntityExtractor {
    fn extract(&self, tokens: &[Token], text: &str) -> QSQLResult<Vec<Entity>> {
        // NER-based extraction
    }
}
```

## Best Practices

### 1. Klare Formulierungen
✅ **Gut:** "Show me all sensors where temperature above 25"
❌ **Schlecht:** "temp stuff over 25"

### 2. Explizite Tabellennamen
✅ **Gut:** "Find all users"
❌ **Schlecht:** "Find everything"

### 3. Eindeutige Operatoren
✅ **Gut:** "temperature greater than 25"
✅ **Gut:** "temperature > 25"
❌ **Schlecht:** "temperature kinda more than 25"

### 4. Strukturierte Bedingungen
✅ **Gut:** "sensors where temperature above 25"
❌ **Schlecht:** "sensors temp 25 something"

## Beispiele aus der Praxis

### IoT Sensor Monitoring
```rust
// Query 1: Aktuelle Sensordaten
let qsql = engine.understand_query(
    "Show me all sensors with temperature above 25 degrees"
)?;

// Query 2: Fehlerhafte Sensoren
let qsql = engine.understand_query(
    "Find all sensors where status equal to error"
)?;

// Query 3: Sensor-Statistiken
let qsql = engine.understand_query(
    "Count all sensors"
)?;
```

### User Management
```rust
// Query 1: Aktive Benutzer
let qsql = engine.understand_query(
    "Show all users where status equal to active"
)?;

// Query 2: Alte Accounts
let qsql = engine.understand_query(
    "Find users where age greater than 30"
)?;
```

### Data Analysis
```rust
// Query 1: Pattern Recognition
let qsql = engine.understand_query(
    "Find similar patterns in data using neural matching"
)?;

// Query 2: Quantum Search
let qsql = engine.understand_query(
    "Quantum search for anomalies"
)?;
```

## Performance

Die Natural Language Pipeline ist optimiert für:
- **Schnelle Tokenisierung**: Regex-basiert, < 1ms für typische Queries
- **Effiziente Pattern Matching**: Vorkompilierte Regex-Patterns
- **Minimaler Overhead**: Direkte Trait-Implementierungen
- **Skalierbarkeit**: Thread-safe und parallelisierbar

## Fehlerbehandlung

Die NLP-Pipeline gibt klare Fehlermeldungen:

```rust
match engine.understand_query("invalid nonsense query") {
    Ok(qsql) => println!("Generated: {}", qsql),
    Err(e) => eprintln!("Error: {}", e),
    // Error: Could not classify intent for: invalid nonsense query
}
```

## Zukünftige Erweiterungen

Geplante Features für die Natural Language Pipeline:
- 🔄 Machine Learning-basierte Intent-Klassifikation
- 🔄 Named Entity Recognition mit Transformer-Modellen
- 🔄 Multi-Language Support (Deutsch, Englisch, etc.)
- 🔄 Context-Aware Query Expansion
- 🔄 Query Suggestion und Auto-Complete
- 🔄 Semantic Query Understanding

## API-Referenz

Siehe [API-Dokumentation](../api/rust.md) für vollständige Details zu:
- `NLQueryEngine`
- `Tokenizer` Trait
- `IntentClassifier` Trait
- `EntityExtractor` Trait
- `QueryGenerator` Trait

## Siehe auch

- [QSQL Language Reference](qsql.md)
- [REST API](../api/rest.md)
- [Architecture Overview](../dev/architecture.md)

