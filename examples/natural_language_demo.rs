//! Natural Language Query Demo
//!
//! Demonstrates the Natural Language Processing capabilities of NeuroQuantumDB.
//! Shows how to translate human language queries into executable QSQL.
//!
//! # Examples
//!
//! ```bash
//! cargo run --example natural_language_demo
//! ```

use anyhow::Result;
use neuroquantum_qsql::natural_language::{
    NLQueryEngine, Tokenizer, IntentClassifier, EntityExtractor, QueryGenerator,
    RegexTokenizer, PatternIntentClassifier, RegexEntityExtractor, QSQLGenerator,
    QueryIntent,
};

fn main() -> Result<()> {
    println!("🧠 NeuroQuantumDB - Natural Language Query Demo\n");
    println!("=" .repeat(60));

    // Create the main NLP engine
    let engine = NLQueryEngine::new()?;

    // Demo 1: Basic SELECT query
    println!("\n📝 Demo 1: Basic SELECT Query");
    println!("-".repeat(60));
    demo_basic_select(&engine)?;

    // Demo 2: Filtered queries
    println!("\n📝 Demo 2: Filtered Queries");
    println!("-".repeat(60));
    demo_filtered_queries(&engine)?;

    // Demo 3: Neuromorphic queries
    println!("\n📝 Demo 3: Neuromorphic Queries");
    println!("-".repeat(60));
    demo_neuromorphic_queries(&engine)?;

    // Demo 4: Quantum queries
    println!("\n📝 Demo 4: Quantum Queries");
    println!("-".repeat(60));
    demo_quantum_queries(&engine)?;

    // Demo 5: Aggregate queries
    println!("\n📝 Demo 5: Aggregate Queries");
    println!("-".repeat(60));
    demo_aggregate_queries(&engine)?;

    // Demo 6: Pipeline components
    println!("\n📝 Demo 6: Individual Pipeline Components");
    println!("-".repeat(60));
    demo_pipeline_components()?;

    // Demo 7: Real-world scenarios
    println!("\n📝 Demo 7: Real-World IoT Scenarios");
    println!("-".repeat(60));
    demo_iot_scenarios(&engine)?;

    println!("\n" .repeat(2));
    println!("✅ All demos completed successfully!");
    println!("=" .repeat(60));

    Ok(())
}

fn demo_basic_select(engine: &NLQueryEngine) -> Result<()> {
    let queries = vec![
        "Show me all users",
        "Display all sensors",
        "Find all records",
        "Get all data from users",
        "List all devices",
    ];

    for query in queries {
        let qsql = engine.understand_query(query)?;
        println!("Natural: {}", query);
        println!("   QSQL: {}", qsql);
        println!();
    }

    Ok(())
}

fn demo_filtered_queries(engine: &NLQueryEngine) -> Result<()> {
    let queries = vec![
        "Show me all sensors where temperature above 25",
        "Find users with age greater than 30",
        "Display sensors where temperature > 20",
        "Get devices where status equal to active",
        "Show sensors with humidity below 60",
    ];

    for query in queries {
        let qsql = engine.understand_query(query)?;
        println!("Natural: {}", query);
        println!("   QSQL: {}", qsql);
        println!();
    }

    Ok(())
}

fn demo_neuromorphic_queries(engine: &NLQueryEngine) -> Result<()> {
    let queries = vec![
        "Find similar patterns using neural matching",
        "Search for similar memories",
        "Match patterns with neuromorphic algorithms",
        "Find neural patterns in data",
    ];

    for query in queries {
        let qsql = engine.understand_query(query)?;
        println!("Natural: {}", query);
        println!("   QSQL: {}", qsql);
        println!();
    }

    Ok(())
}

fn demo_quantum_queries(engine: &NLQueryEngine) -> Result<()> {
    let queries = vec![
        "Quantum search for data",
        "Search using quantum algorithms",
        "Find data with quantum search",
        "Quantum superposition search",
    ];

    for query in queries {
        let qsql = engine.understand_query(query)?;
        println!("Natural: {}", query);
        println!("   QSQL: {}", qsql);
        println!();
    }

    Ok(())
}

fn demo_aggregate_queries(engine: &NLQueryEngine) -> Result<()> {
    let queries = vec![
        "Count all users",
        "Sum of all temperatures",
        "Average temperature in sensors",
        "Total count of devices",
    ];

    for query in queries {
        let qsql = engine.understand_query(query)?;
        println!("Natural: {}", query);
        println!("   QSQL: {}", qsql);
        println!();
    }

    Ok(())
}

fn demo_pipeline_components() -> Result<()> {
    println!("Demonstrating individual pipeline components:\n");

    // 1. Tokenizer
    println!("1️⃣ Tokenizer");
    let tokenizer = RegexTokenizer::new()?;
    let tokens = tokenizer.tokenize("Show me sensors where temperature > 25")?;
    println!("   Input: 'Show me sensors where temperature > 25'");
    println!("   Tokens: {:?}", tokens.iter().map(|t| &t.text).collect::<Vec<_>>());
    println!();

    // 2. Intent Classifier
    println!("2️⃣ Intent Classifier");
    let classifier = PatternIntentClassifier::new()?;
    let intent = classifier.classify(&tokens, "show me all users")?;
    println!("   Input: 'show me all users'");
    println!("   Intent: {:?}", intent);
    println!("   Confidence: {:.2}", classifier.confidence(&intent, &tokens));
    println!();

    // 3. Entity Extractor
    println!("3️⃣ Entity Extractor");
    let extractor = RegexEntityExtractor::new()?;
    let entities = extractor.extract(&tokens, "show sensors where temperature > 25")?;
    println!("   Input: 'show sensors where temperature > 25'");
    println!("   Entities:");
    for entity in &entities {
        println!("     - {:?}: '{}' (confidence: {:.2})",
                 entity.entity_type, entity.value, entity.confidence);
    }
    println!();

    // 4. Query Generator
    println!("4️⃣ Query Generator");
    let generator = QSQLGenerator::new()?;
    let qsql = generator.generate(&QueryIntent::Select, &entities)?;
    println!("   Intent: {:?}", QueryIntent::Select);
    println!("   Entities: {} found", entities.len());
    println!("   Generated QSQL: {}", qsql);
    println!();

    Ok(())
}

fn demo_iot_scenarios(engine: &NLQueryEngine) -> Result<()> {
    println!("Real-world IoT monitoring scenarios:\n");

    // Scenario 1: Temperature Alert
    println!("🌡️  Scenario 1: Temperature Alert");
    let qsql = engine.understand_query(
        "Show me all sensors where temperature above 30"
    )?;
    println!("   Use case: Find overheating sensors");
    println!("   Natural:  'Show me all sensors where temperature above 30'");
    println!("   QSQL:     {}", qsql);
    println!();

    // Scenario 2: Device Status Check
    println!("🔌 Scenario 2: Device Status Check");
    let qsql = engine.understand_query(
        "Find all devices where status equal to error"
    )?;
    println!("   Use case: Identify failing devices");
    println!("   Natural:  'Find all devices where status equal to error'");
    println!("   QSQL:     {}", qsql);
    println!();

    // Scenario 3: Pattern Recognition
    println!("🧠 Scenario 3: Pattern Recognition");
    let qsql = engine.understand_query(
        "Find similar patterns in sensor data using neural matching"
    )?;
    println!("   Use case: Anomaly detection with neuromorphic computing");
    println!("   Natural:  'Find similar patterns in sensor data using neural matching'");
    println!("   QSQL:     {}", qsql);
    println!();

    // Scenario 4: Quantum Search
    println!("⚛️  Scenario 4: Quantum-Accelerated Search");
    let qsql = engine.understand_query(
        "Quantum search for anomalies in data"
    )?;
    println!("   Use case: Fast search in large sensor datasets");
    println!("   Natural:  'Quantum search for anomalies in data'");
    println!("   QSQL:     {}", qsql);
    println!();

    // Scenario 5: Statistics
    println!("📊 Scenario 5: Sensor Statistics");
    let qsql = engine.understand_query(
        "Count all sensors"
    )?;
    println!("   Use case: Get total number of deployed sensors");
    println!("   Natural:  'Count all sensors'");
    println!("   QSQL:     {}", qsql);
    println!();

    Ok(())
}

