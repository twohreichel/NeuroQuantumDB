//! NEON Optimization Demo
//!
//! Demonstrates the ARM64 NEON SIMD acceleration features of NeuroQuantumDB
//! including DNA compression, matrix operations, and quantum state processing.

use neuroquantum_core::neon_optimization::{NeonOptimizer, QuantumOperation};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🧬 NeuroQuantumDB - ARM64 NEON Optimization Demo");
    println!("================================================\n");

    // Create NEON optimizer
    let mut optimizer = NeonOptimizer::new()?;

    if optimizer.is_enabled() {
        println!("✅ ARM64 NEON SIMD detected and enabled");
    } else {
        println!("⚠️  NEON not available - using scalar fallback");
    }

    println!();

    // Demo 1: DNA Compression
    demo_dna_compression(&mut optimizer)?;

    // Demo 2: Matrix Operations
    demo_matrix_operations(&mut optimizer)?;

    // Demo 3: Quantum State Operations
    demo_quantum_operations(&mut optimizer)?;

    // Demo 4: Parallel Search
    demo_parallel_search(&mut optimizer)?;

    // Demo 5: Neural Network Operations
    demo_neural_operations(&optimizer)?;

    // Print final statistics
    print_statistics(&optimizer);

    Ok(())
}

/// Demonstrate NEON-accelerated DNA compression
fn demo_dna_compression(optimizer: &mut NeonOptimizer) -> Result<(), Box<dyn std::error::Error>> {
    println!("1️⃣  DNA Compression Demo");
    println!("   ---------------------");

    // Generate sample genomic data
    let data_sizes = [1024, 4096, 16384, 65536];

    for size in data_sizes {
        let data: Vec<u8> = (0..size).map(|i| ((i * 73) % 256) as u8).collect();

        let start = Instant::now();
        let compressed = optimizer.vectorized_dna_compression(&data)?;
        let duration = start.elapsed();

        let compression_ratio = data.len() as f32 / compressed.len() as f32;

        println!(
            "   {} bytes → {} bytes ({}:1 ratio) in {:?}",
            data.len(),
            compressed.len(),
            compression_ratio,
            duration
        );

        optimizer.update_performance_stats("dna_compression", duration.as_nanos() as u64);
    }

    println!();
    Ok(())
}

/// Demonstrate NEON-accelerated matrix operations
fn demo_matrix_operations(optimizer: &mut NeonOptimizer) -> Result<(), Box<dyn std::error::Error>> {
    println!("2️⃣  Matrix Multiplication Demo");
    println!("   ---------------------------");

    let matrix_sizes = [(4, 4), (8, 8), (16, 16), (32, 32)];

    for (rows, cols) in matrix_sizes {
        let matrix_a: Vec<f32> = (0..rows * cols).map(|i| i as f32 * 0.1).collect();
        let matrix_b: Vec<f32> = (0..rows * cols).map(|i| i as f32 * 0.2).collect();

        let start = Instant::now();
        let result = optimizer.matrix_multiply_neon(&matrix_a, &matrix_b, rows, cols, cols)?;
        let duration = start.elapsed();

        println!(
            "   {}x{} matrix multiplication: {:?} ({} FLOPS)",
            rows,
            cols,
            duration,
            (2 * rows * cols * cols) as f32 / duration.as_secs_f32()
        );

        // Verify result is valid
        assert_eq!(result.len(), rows * cols);

        optimizer.update_performance_stats("matrix_ops", duration.as_nanos() as u64);
    }

    println!();
    Ok(())
}

/// Demonstrate NEON-accelerated quantum state operations
fn demo_quantum_operations(optimizer: &mut NeonOptimizer) -> Result<(), Box<dyn std::error::Error>> {
    println!("3️⃣  Quantum State Operations Demo");
    println!("   ------------------------------");

    // Test different quantum state sizes (qubits)
    for qubits in [4, 6, 8, 10] {
        let size = 1 << qubits; // 2^qubits
        let mut real_parts: Vec<f32> = (0..size).map(|i| (i as f32) / (size as f32)).collect();
        let mut imag_parts: Vec<f32> = vec![0.0; size];

        // Normalize
        let start = Instant::now();
        optimizer.quantum_state_operation(&mut real_parts, &mut imag_parts, QuantumOperation::Normalize)?;
        let norm_duration = start.elapsed();

        // Verify normalization
        let norm_sq: f32 = real_parts
            .iter()
            .zip(imag_parts.iter())
            .map(|(r, i)| r * r + i * i)
            .sum();

        println!(
            "   {} qubits ({} amplitudes): Normalize {:?}, norm={:.6}",
            qubits, size, norm_duration, norm_sq.sqrt()
        );

        // Phase flip
        let start = Instant::now();
        optimizer.quantum_state_operation(&mut real_parts, &mut imag_parts, QuantumOperation::PhaseFlip)?;
        let flip_duration = start.elapsed();

        println!("      Phase flip: {:?}", flip_duration);

        // Hadamard transformation
        let start = Instant::now();
        optimizer.quantum_state_operation(&mut real_parts, &mut imag_parts, QuantumOperation::Hadamard)?;
        let hadamard_duration = start.elapsed();

        println!("      Hadamard: {:?}", hadamard_duration);

        optimizer.update_performance_stats("quantum_ops", norm_duration.as_nanos() as u64);
    }

    println!();
    Ok(())
}

/// Demonstrate NEON-accelerated parallel search
fn demo_parallel_search(optimizer: &mut NeonOptimizer) -> Result<(), Box<dyn std::error::Error>> {
    println!("4️⃣  Parallel Search Demo");
    println!("   --------------------");

    // Create test data with embedded patterns
    let pattern = b"ACGT";
    let mut haystack = Vec::new();

    // Insert pattern at specific positions
    let pattern_positions = vec![100, 500, 1000, 5000, 10000];
    let mut random_data: Vec<u8> = (0..20000).map(|i| ((i * 137) % 256) as u8).collect();

    for &pos in &pattern_positions {
        if pos + pattern.len() <= random_data.len() {
            random_data[pos..pos + pattern.len()].copy_from_slice(pattern);
        }
    }

    haystack.extend_from_slice(&random_data);

    let start = Instant::now();
    let matches = optimizer.parallel_search(&haystack, pattern)?;
    let duration = start.elapsed();

    println!(
        "   Searched {} bytes for pattern {:?}",
        haystack.len(),
        std::str::from_utf8(pattern).unwrap_or("(binary)")
    );
    println!(
        "   Found {} matches in {:?} ({:.2} GB/s)",
        matches.len(),
        duration,
        (haystack.len() as f64 / duration.as_secs_f64()) / 1_000_000_000.0
    );
    println!("   Match positions: {:?}", &matches[..matches.len().min(10)]);

    println!();
    Ok(())
}

/// Demonstrate neural network operations
fn demo_neural_operations(optimizer: &NeonOptimizer) -> Result<(), Box<dyn std::error::Error>> {
    println!("5️⃣  Neural Network Operations Demo");
    println!("   -------------------------------");

    // Dot product (neuron activation)
    let inputs = vec![0.5, 0.3, 0.8, 0.1, 0.9, 0.4, 0.6, 0.2];
    let weights = vec![0.7, 0.2, 0.5, 0.9, 0.3, 0.6, 0.4, 0.8];

    let start = Instant::now();
    let activation = optimizer.dot_product(&inputs, &weights)?;
    let dot_duration = start.elapsed();

    println!(
        "   Dot product ({} elements): {} in {:?}",
        inputs.len(),
        activation,
        dot_duration
    );

    // Activation function
    let mut layer_outputs = vec![1.5, -0.5, 2.3, 0.8, -1.2, 1.8, 0.3, -0.8];
    let threshold = 1.0;

    let start = Instant::now();
    optimizer.apply_activation_function(&mut layer_outputs, threshold)?;
    let act_duration = start.elapsed();

    println!(
        "   Activation function (ReLU-like) on {} neurons: {:?}",
        layer_outputs.len(),
        act_duration
    );
    println!("   Output: {:?}", layer_outputs);

    // Matrix operations (layer transformation)
    let mut layer_matrix = vec![
        0.5, 0.3, 0.8, 0.1,
        0.9, 0.4, 0.6, 0.2,
        0.7, 0.2, 0.5, 0.9,
        0.3, 0.6, 0.4, 0.8,
    ];

    let start = Instant::now();
    optimizer.optimize_matrix_operations(&mut layer_matrix)?;
    let matrix_duration = start.elapsed();

    println!(
        "   Sigmoid activation on 4x4 matrix: {:?}",
        matrix_duration
    );

    println!();
    Ok(())
}

/// Print performance statistics
fn print_statistics(optimizer: &NeonOptimizer) {
    println!("📊 Performance Statistics");
    println!("   =====================");

    let stats = optimizer.get_stats();

    println!("   SIMD operations:      {}", stats.simd_operations);
    println!("   Scalar fallbacks:     {}", stats.scalar_fallbacks);
    println!("   Total bytes processed: {}", stats.total_bytes_processed);
    println!();
    println!("   Speedup factors:");
    println!("   - DNA compression:    {:.2}x", stats.dna_compression_speedup);
    println!("   - Matrix operations:  {:.2}x", stats.matrix_ops_speedup);
    println!("   - Quantum operations: {:.2}x", stats.quantum_ops_speedup);
    println!("   - Overall gain:       {:.2}x", stats.performance_gain);
    println!();

    if optimizer.is_enabled() {
        println!("✨ ARM64 NEON SIMD acceleration is working!");
    } else {
        println!("⚠️  Running on scalar fallback (no NEON available)");
    }
}

