//! Quantum Search Demo - Grover's Algorithm
//!
//! This example demonstrates the real quantum state vector simulator
//! implementing Grover's search algorithm with:
//! - Quantum superposition initialization
//! - Oracle phase flips
//! - Diffusion operator (amplitude amplification)
//! - Multiple search scenarios
//! - Performance comparisons with classical search

use neuroquantum_core::quantum_processor::{
    create_byte_search_processor, DatabaseOracle, QuantumProcessorConfig, QuantumStateProcessor,
};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🔬 NeuroQuantumDB - Quantum Search Demo (Grover's Algorithm)");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("This demo showcases a real quantum state vector simulator");
    println!("implementing Grover's algorithm for database search.");
    println!();

    // Demo 1: Simple database search
    demo_simple_database_search()?;
    println!();

    // Demo 2: Byte pattern search
    demo_byte_pattern_search()?;
    println!();

    // Demo 3: Multiple target search
    demo_multiple_targets()?;
    println!();

    // Demo 4: Quantum vs Classical comparison
    demo_quantum_vs_classical()?;
    println!();

    // Demo 5: Scaling analysis
    demo_scaling_analysis()?;
    println!();

    // Demo 6: DNA sequence search (bioinformatics)
    demo_dna_sequence_search()?;
    println!();

    println!("═══════════════════════════════════════════════════════════════");
    println!("📊 Quantum Search System Summary");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("✓ Quantum State Vector: |ψ⟩ = Σ αᵢ|i⟩");
    println!("✓ Superposition: αᵢ = 1/√N for all states");
    println!("✓ Oracle: Phase flip |x⟩ → -|x⟩ for target states");
    println!("✓ Diffusion: Amplitude amplification (2|ψ⟩⟨ψ| - I)");
    println!("✓ Iterations: π/4 * √N (optimal)");
    println!("✓ Speedup: √N over classical O(N) search");
    println!("✓ Quantum Limit: ~20-25 qubits on classical hardware");
    println!();
    println!("🧬 Biological Inspiration:");
    println!("  - Quantum tunneling in microtubules (Penrose-Hameroff)");
    println!("  - Quantum coherence in avian navigation");
    println!("  - Photosynthesis quantum efficiency");
    println!();
    println!("✅ Demo completed successfully!");

    Ok(())
}

/// Demo 1: Simple database search with integers
fn demo_simple_database_search() -> Result<(), Box<dyn std::error::Error>> {
    println!("📦 Demo 1: Simple Database Search");
    println!("───────────────────────────────────────────────────────────────");

    // Create a database of integers
    let database = vec![42, 17, 93, 8, 55, 23, 67, 12];
    let target = 55;
    let n = database.len();

    println!("Database: {:?}", database);
    println!("Target: {}", target);
    println!(
        "Size: N = {} (requires {} qubits)",
        n,
        (n as f64).log2().ceil() as usize
    );
    println!();

    // Create oracle and quantum processor
    let oracle = Arc::new(DatabaseOracle::new(database.clone(), target));
    let config = QuantumProcessorConfig::default();
    let qubits = (n as f64).log2().ceil() as usize;

    let mut processor = QuantumStateProcessor::new(qubits, oracle, config)?;

    // Calculate optimal iterations
    let iterations = ((std::f64::consts::PI / 4.0) * (n as f64).sqrt()) as usize;
    println!("Optimal Grover iterations: π/4 * √{} ≈ {}", n, iterations);
    println!();

    // Run Grover's search
    let start = Instant::now();
    let result = processor.grovers_search()?;
    let elapsed = start.elapsed();

    let probability = processor.get_probability(result);

    println!("✅ Quantum Search Result:");
    println!("  Found index: {} (value: {})", result, database[result]);
    println!(
        "  Probability: {:.4} ({:.2}%)",
        probability,
        probability * 100.0
    );
    println!("  Time: {:?}", elapsed);
    println!(
        "  Verification: {}",
        if database[result] == target {
            "✓ CORRECT"
        } else {
            "✗ INCORRECT"
        }
    );

    // Classical comparison
    let classical_start = Instant::now();
    let classical_result = database.iter().position(|&x| x == target).unwrap();
    let classical_elapsed = classical_start.elapsed();

    println!();
    println!("📊 Classical Linear Search:");
    println!("  Found index: {}", classical_result);
    println!("  Time: {:?}", classical_elapsed);
    println!("  Expected comparisons: {} (worst case)", n);

    Ok(())
}

/// Demo 2: Byte pattern search in data
fn demo_byte_pattern_search() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Demo 2: Byte Pattern Search");
    println!("───────────────────────────────────────────────────────────────");

    // Create data and search pattern
    let data = b"The quick brown fox jumps over the lazy dog".to_vec();
    let pattern = b"fox".to_vec();

    println!("Data: \"{}\"", String::from_utf8_lossy(&data));
    println!("Pattern: \"{}\"", String::from_utf8_lossy(&pattern));
    println!("Data length: {} bytes", data.len());
    println!();

    // Create quantum processor for byte search
    let config = QuantumProcessorConfig::default();
    let mut processor = create_byte_search_processor(data.clone(), pattern.clone(), config)?;

    println!("Qubits: {}", processor.qubit_count());
    println!("State size: {} quantum states", processor.state_size());
    println!();

    // Run Grover's search
    let start = Instant::now();
    let result = processor.grovers_search()?;
    let elapsed = start.elapsed();

    println!("✅ Quantum Search Result:");
    println!("  Found at position: {}", result);
    println!(
        "  Context: \"{}\"",
        String::from_utf8_lossy(&data[result..result + 10.min(data.len() - result)])
    );
    println!("  Time: {:?}", elapsed);

    // Verify result
    let is_correct = result + pattern.len() <= data.len()
        && &data[result..result + pattern.len()] == pattern.as_slice();
    println!(
        "  Verification: {}",
        if is_correct {
            "✓ CORRECT"
        } else {
            "✗ INCORRECT"
        }
    );

    Ok(())
}

/// Demo 3: Search for multiple targets
fn demo_multiple_targets() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Demo 3: Multiple Target Search");
    println!("───────────────────────────────────────────────────────────────");

    // Create database with multiple occurrences of target
    let database = vec![10, 20, 30, 20, 40, 20, 50, 60];
    let target = 20;

    println!("Database: {:?}", database);
    println!(
        "Target: {} (appears {} times)",
        target,
        database.iter().filter(|&&x| x == target).count()
    );
    println!();

    // Create quantum processor
    let oracle = Arc::new(DatabaseOracle::new(database.clone(), target));
    let config = QuantumProcessorConfig {
        measurement_threshold: 0.05, // Lower threshold to find multiple results
        ..Default::default()
    };

    let qubits = (database.len() as f64).log2().ceil() as usize;
    let mut processor = QuantumStateProcessor::new(qubits, oracle, config)?;

    // Run Grover's search for multiple results
    let start = Instant::now();
    let results = processor.grovers_search_multiple()?;
    let elapsed = start.elapsed();

    println!("✅ Quantum Search Results (multiple targets):");
    println!("  Found {} results", results.len());
    println!("  Time: {:?}", elapsed);
    println!();

    for (index, probability) in &results {
        if *index < database.len() {
            println!(
                "  Index {}: value = {}, probability = {:.4} ({:.2}%)",
                index,
                database[*index],
                probability,
                probability * 100.0
            );
        }
    }

    // Verify all results
    let all_correct = results
        .iter()
        .filter(|(idx, _)| *idx < database.len())
        .all(|(idx, _)| database[*idx] == target);

    println!();
    println!(
        "  Verification: {}",
        if all_correct {
            "✓ ALL CORRECT"
        } else {
            "✗ SOME INCORRECT"
        }
    );

    Ok(())
}

/// Demo 4: Quantum vs Classical performance comparison
fn demo_quantum_vs_classical() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Demo 4: Quantum vs Classical Performance");
    println!("───────────────────────────────────────────────────────────────");

    let sizes = vec![16, 64, 256];

    println!(
        "{:<10} {:<15} {:<15} {:<15}",
        "Size (N)", "Quantum (μs)", "Classical (μs)", "Speedup"
    );
    println!("{:-<10} {:-<15} {:-<15} {:-<15}", "", "", "", "");

    for &size in &sizes {
        // Generate database
        let database: Vec<u32> = (0..size).collect();
        let target = size - 1; // Worst case for classical

        // Quantum search
        let oracle = Arc::new(DatabaseOracle::new(database.clone(), target));
        let config = QuantumProcessorConfig::default();
        let qubits = (size as f64).log2().ceil() as usize;
        let mut processor = QuantumStateProcessor::new(qubits, oracle, config)?;

        let quantum_start = Instant::now();
        let _ = processor.grovers_search()?;
        let quantum_time = quantum_start.elapsed();

        // Classical search
        let classical_start = Instant::now();
        let _ = database.iter().position(|&x| x == target);
        let classical_time = classical_start.elapsed();

        let speedup = classical_time.as_micros() as f64 / quantum_time.as_micros().max(1) as f64;

        println!(
            "{:<10} {:<15.2} {:<15.2} {:<15.2}x",
            size,
            quantum_time.as_micros(),
            classical_time.as_micros(),
            speedup
        );
    }

    println!();
    println!("Note: Theoretical quantum speedup is √N");
    println!("      Actual speedup depends on implementation overhead");

    Ok(())
}

/// Demo 5: Scaling analysis - qubits vs database size
fn demo_scaling_analysis() -> Result<(), Box<dyn std::error::Error>> {
    println!("📈 Demo 5: Quantum Search Scaling Analysis");
    println!("───────────────────────────────────────────────────────────────");

    println!(
        "{:<10} {:<15} {:<20} {:<20}",
        "Qubits", "States (2^n)", "Grover Iterations", "Classical Ops"
    );
    println!("{:-<10} {:-<15} {:-<20} {:-<20}", "", "", "", "");

    for qubits in 4..=12 {
        let states = 1usize << qubits; // 2^n
        let grover_iters = ((std::f64::consts::PI / 4.0) * (states as f64).sqrt()) as usize;
        let classical_ops = states; // Linear search

        println!(
            "{:<10} {:<15} {:<20} {:<20}",
            qubits, states, grover_iters, classical_ops
        );
    }

    println!();
    println!("📊 Key Observations:");
    println!("  • Quantum speedup: √N advantage over classical");
    println!("  • Memory requirement: 2^n complex numbers (state vector)");
    println!("  • Practical limit: ~20-25 qubits on classical hardware");
    println!("  • Real quantum computers: Can handle more qubits");

    Ok(())
}

/// Demo 6: DNA sequence search (bioinformatics application)
fn demo_dna_sequence_search() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧬 Demo 6: DNA Sequence Search (Bioinformatics)");
    println!("───────────────────────────────────────────────────────────────");

    // Create a DNA sequence
    let dna_sequence = b"ATCGATCGATCGTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGC".to_vec();
    let motif = b"TAGC".to_vec();

    println!("DNA Sequence: {}", String::from_utf8_lossy(&dna_sequence));
    println!("Motif: {}", String::from_utf8_lossy(&motif));
    println!("Sequence length: {} bases", dna_sequence.len());
    println!();

    // Create quantum processor
    let config = QuantumProcessorConfig::default();
    let mut processor = create_byte_search_processor(dna_sequence.clone(), motif.clone(), config)?;

    println!("Qubits: {}", processor.qubit_count());
    println!();

    // Run Grover's search
    let start = Instant::now();
    let result = processor.grovers_search()?;
    let elapsed = start.elapsed();

    println!("✅ Quantum DNA Motif Search Result:");
    println!("  Motif found at position: {}", result);
    println!("  Time: {:?}", elapsed);

    // Extract context
    if result + 10 <= dna_sequence.len() {
        let context = &dna_sequence[result..result + 10];
        println!("  Context: {}", String::from_utf8_lossy(context));
    }

    // Verify result
    let is_correct = result + motif.len() <= dna_sequence.len()
        && &dna_sequence[result..result + motif.len()] == motif.as_slice();
    println!(
        "  Verification: {}",
        if is_correct {
            "✓ CORRECT"
        } else {
            "✗ INCORRECT"
        }
    );

    println!();
    println!("🔬 Bioinformatics Applications:");
    println!("  • Gene sequence alignment");
    println!("  • Protein folding prediction");
    println!("  • Drug discovery (molecular search)");
    println!("  • Genome assembly optimization");

    Ok(())
}
