//! Demonstration of Grover's Algorithm Implementation
//!
//! This example shows how to use the quantum processor with Grover's algorithm
//! for efficient database searching with quadratic speedup.

use neuroquantum_core::quantum_processor::{
    create_byte_search_processor, ByteOracle, DatabaseOracle, QuantumProcessorConfig,
    QuantumStateProcessor,
};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("🔬 NeuroQuantumDB - Grover's Algorithm Demo\n");
    println!("=".repeat(60));

    // Demo 1: Simple integer search
    demo_integer_search();

    // Demo 2: Byte pattern search
    demo_byte_pattern_search();

    // Demo 3: Performance comparison
    demo_performance_comparison();

    // Demo 4: Multiple target search
    demo_multiple_targets();

    println!("\n✅ All demos completed successfully!");
}

fn demo_integer_search() {
    println!("\n📊 Demo 1: Integer Database Search");
    println!("-".repeat(60));

    let database = vec![10, 20, 30, 40, 50, 60, 70, 80];
    let target = 50;

    println!("Database: {:?}", database);
    println!("Searching for: {}", target);

    let oracle = Arc::new(DatabaseOracle::new(database.clone(), target));
    let config = QuantumProcessorConfig::default();
    let qubits = (database.len() as f64).log2().ceil() as usize;

    let mut processor = QuantumStateProcessor::new(qubits, oracle, config)
        .expect("Failed to create quantum processor");

    let start = Instant::now();
    let result = processor.grovers_search()
        .expect("Grover's search failed");
    let duration = start.elapsed();

    println!("\n✅ Found at index: {}", result);
    println!("   Value: {}", database[result]);
    println!("   Probability: {:.4}", processor.get_probability(result));
    println!("   Search time: {:?}", duration);

    // Calculate theoretical speedup
    let classical_ops = database.len();
    let quantum_ops = (database.len() as f64).sqrt() as usize;
    let speedup = classical_ops as f64 / quantum_ops as f64;

    println!("\n📈 Theoretical Analysis:");
    println!("   Classical search: O({}) operations", classical_ops);
    println!("   Quantum search: O({}) operations", quantum_ops);
    println!("   Speedup factor: {:.2}x", speedup);
}

fn demo_byte_pattern_search() {
    println!("\n📝 Demo 2: Byte Pattern Search");
    println!("-".repeat(60));

    let text = b"The quick brown fox jumps over the lazy quantum dog!";
    let pattern = b"quantum";

    println!("Text: {}", String::from_utf8_lossy(text));
    println!("Pattern: {}", String::from_utf8_lossy(pattern));

    let config = QuantumProcessorConfig::default();

    let start = Instant::now();
    let mut processor = create_byte_search_processor(
        text.to_vec(),
        pattern.to_vec(),
        config,
    ).expect("Failed to create processor");

    let result = processor.grovers_search()
        .expect("Search failed");
    let duration = start.elapsed();

    println!("\n✅ Pattern found at position: {}", result);
    println!("   Context: \"{}\"",
        String::from_utf8_lossy(&text[result.saturating_sub(5)..
            (result + pattern.len() + 5).min(text.len())])
    );
    println!("   Probability: {:.4}", processor.get_probability(result));
    println!("   Search time: {:?}", duration);
}

fn demo_performance_comparison() {
    println!("\n⚡ Demo 3: Performance Comparison (Quantum vs Classical)");
    println!("-".repeat(60));

    let sizes = vec![16, 64, 256, 1024];

    println!("\n{:<12} {:<15} {:<15} {:<12}",
        "Size", "Classical (μs)", "Quantum (μs)", "Speedup");
    println!("{}", "-".repeat(60));

    for size in sizes {
        let mut data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        data[size - 1] = 42; // Target at end (worst case for classical)
        let pattern = vec![42u8];

        // Classical search
        let start = Instant::now();
        let _classical_result = classical_search(&data, &pattern);
        let classical_time = start.elapsed();

        // Quantum search
        let config = QuantumProcessorConfig::default();
        let start = Instant::now();
        let mut processor = create_byte_search_processor(
            data.clone(),
            pattern.clone(),
            config,
        ).unwrap();
        let _quantum_result = processor.grovers_search().unwrap();
        let quantum_time = start.elapsed();

        let speedup = classical_time.as_micros() as f64 / quantum_time.as_micros() as f64;

        println!("{:<12} {:<15} {:<15} {:<12.2}x",
            size,
            classical_time.as_micros(),
            quantum_time.as_micros(),
            speedup
        );
    }
}

fn demo_multiple_targets() {
    println!("\n🎯 Demo 4: Multiple Target Search");
    println!("-".repeat(60));

    let data = b"Quantum computing uses quantum mechanics for quantum speedup!";
    let pattern = b"quantum";

    println!("Text: {}", String::from_utf8_lossy(data));
    println!("Pattern: {}", String::from_utf8_lossy(pattern));

    let config = QuantumProcessorConfig {
        max_grover_iterations: 100,
        verify_normalization: true,
        measurement_threshold: 0.05,
    };

    let mut processor = create_byte_search_processor(
        data.to_vec(),
        pattern.to_vec(),
        config,
    ).expect("Failed to create processor");

    let start = Instant::now();
    let results = processor.grovers_search_multiple()
        .expect("Multiple search failed");
    let duration = start.elapsed();

    println!("\n✅ Found {} occurrences:", results.len());
    for (idx, prob) in &results {
        // Verify it's actually the pattern
        if idx + pattern.len() <= data.len() {
            let matched = &data[*idx..*idx + pattern.len()] == pattern;
            if matched {
                println!("   Position {}: probability={:.4} ✓", idx, prob);
            }
        }
    }
    println!("   Search time: {:?}", duration);
}

/// Classical linear search for comparison
fn classical_search(data: &[u8], pattern: &[u8]) -> Option<usize> {
    for i in 0..=data.len().saturating_sub(pattern.len()) {
        if &data[i..i + pattern.len()] == pattern {
            return Some(i);
        }
    }
    None
}

