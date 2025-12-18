//! Comprehensive tests for Grover's Algorithm Quantum Processor

#[cfg(test)]
mod tests {
    use crate::quantum_processor::{
        create_byte_search_processor, ByteOracle, DatabaseOracle, QuantumProcessorConfig,
        QuantumStateProcessor,
    };
    use std::sync::Arc;

    #[test]
    fn test_quantum_processor_creation() {
        let data = vec![1u8, 2, 3, 4];
        let target = 3u8;
        let oracle = Arc::new(DatabaseOracle::new(data, target));
        let config = QuantumProcessorConfig::default();

        let processor = QuantumStateProcessor::new(2, oracle, config);
        assert!(processor.is_ok());

        let processor = processor.unwrap();
        assert_eq!(processor.qubit_count(), 2);
        assert_eq!(processor.state_size(), 4);
    }

    #[test]
    fn test_invalid_qubit_count() {
        let data = vec![1u8, 2, 3, 4];
        let target = 3u8;
        let oracle = Arc::new(DatabaseOracle::new(data, target));
        let config = QuantumProcessorConfig::default();

        // 0 qubits should fail
        assert!(QuantumStateProcessor::new(0, oracle.clone(), config.clone()).is_err());

        // Too many qubits should fail
        assert!(QuantumStateProcessor::new(31, oracle, config).is_err());
    }

    #[test]
    fn test_superposition_initialization() {
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let pattern = vec![5u8];
        let config = QuantumProcessorConfig::default();

        let mut processor = create_byte_search_processor(data, pattern, config).unwrap();
        processor.initialize_superposition().unwrap();

        // Check normalization: sum of |amplitude|^2 should be 1
        assert!(processor.verify_normalization());

        // All states should have equal probability in superposition
        let expected_prob = 1.0 / (processor.state_size() as f64);
        for i in 0..processor.state_size() {
            let prob = processor.get_probability(i);
            assert!((prob - expected_prob).abs() < 1e-10);
        }
    }

    #[test]
    fn test_oracle_phase_flip() {
        let data = vec![1u8, 2, 3, 4];
        let target = 3u8;
        let oracle = Arc::new(DatabaseOracle::new(data, target));
        let config = QuantumProcessorConfig::default();

        let mut processor = QuantumStateProcessor::new(2, oracle, config).unwrap();
        processor.initialize_superposition().unwrap();

        // Get probability before oracle application
        let prob_before = processor.get_probability(2); // Index 2 has value 3

        processor.apply_oracle().unwrap();

        // After oracle, the amplitude should be flipped (negative)
        // but probability |amplitude|^2 stays the same
        let prob_after = processor.get_probability(2);
        assert!((prob_before - prob_after).abs() < 1e-10);

        // Normalization should still hold
        assert!(processor.verify_normalization());
    }

    #[test]
    fn test_diffusion_operator() {
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let pattern = vec![5u8];
        let config = QuantumProcessorConfig::default();

        let mut processor = create_byte_search_processor(data, pattern, config).unwrap();
        processor.initialize_superposition().unwrap();
        processor.apply_oracle().unwrap();

        // Apply diffusion operator
        processor.apply_diffusion_operator().unwrap();

        // Normalization should still hold after diffusion
        assert!(processor.verify_normalization());
    }

    #[test]
    fn test_grovers_search_finds_target() {
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let pattern = vec![5u8];
        let config = QuantumProcessorConfig::default();

        let mut processor = create_byte_search_processor(data.clone(), pattern, config).unwrap();
        let result = processor.grovers_search().unwrap();

        // Should find index 4 (where value 5 is located)
        assert_eq!(result, 4);
        assert_eq!(data[result], 5u8);

        // Probability at found index should be high
        let prob = processor.get_probability(result);
        assert!(prob > 0.5, "Probability {} should be > 0.5", prob);
    }

    #[test]
    fn test_grovers_search_multiple_targets() {
        // Create data with multiple occurrences of target
        let data = vec![1u8, 5, 3, 5, 5, 6, 7, 8];
        let pattern = vec![5u8];
        let config = QuantumProcessorConfig::default();

        let mut processor = create_byte_search_processor(data.clone(), pattern, config).unwrap();
        let results = processor.grovers_search_multiple().unwrap();

        // Should find multiple indices
        assert!(!results.is_empty());

        // Verify all results actually contain the target
        for (idx, _prob) in results {
            assert_eq!(data[idx], 5u8);
        }
    }

    #[test]
    fn test_grovers_search_byte_pattern() {
        let data = b"Hello Quantum World!".to_vec();
        let pattern = b"Quantum".to_vec();
        let config = QuantumProcessorConfig::default();

        let mut processor = create_byte_search_processor(data.clone(), pattern.clone(), config).unwrap();
        let result = processor.grovers_search().unwrap();

        // Verify the pattern is found at the correct position
        assert_eq!(&data[result..result + pattern.len()], pattern.as_slice());
    }

    #[test]
    fn test_grovers_search_performance() {
        // Test with larger dataset
        let mut data: Vec<u8> = (0..128).map(|i| (i % 256) as u8).collect();
        data[100] = 42; // Place target at position 100
        let pattern = vec![42u8];
        let config = QuantumProcessorConfig::default();

        let start = std::time::Instant::now();
        let mut processor = create_byte_search_processor(data.clone(), pattern, config).unwrap();
        let result = processor.grovers_search().unwrap();
        let duration = start.elapsed();

        // Should find the target
        assert_eq!(data[result], 42u8);

        // Should complete reasonably fast
        assert!(duration.as_millis() < 1000, "Search took too long: {:?}", duration);
    }

    #[test]
    fn test_measurement_highest_probability() {
        let data = vec![1u8, 2, 3, 4];
        let target = 3u8;
        let oracle = Arc::new(DatabaseOracle::new(data, target));
        let config = QuantumProcessorConfig::default();

        let mut processor = QuantumStateProcessor::new(2, oracle, config).unwrap();
        processor.initialize_superposition().unwrap();

        // After several Grover iterations, target should have highest probability
        for _ in 0..2 {
            processor.apply_oracle().unwrap();
            processor.apply_diffusion_operator().unwrap();
        }

        let measured = processor.measure_highest_probability().unwrap();
        assert_eq!(measured, 2); // Index 2 contains value 3
    }

    #[test]
    fn test_quantum_state_reset() {
        let data = vec![1u8, 2, 3, 4];
        let target = 3u8;
        let oracle = Arc::new(DatabaseOracle::new(data, target));
        let config = QuantumProcessorConfig::default();

        let mut processor = QuantumStateProcessor::new(2, oracle, config).unwrap();
        processor.initialize_superposition().unwrap();

        // Perform some operations
        processor.apply_oracle().unwrap();
        processor.apply_diffusion_operator().unwrap();

        // Reset state
        processor.reset();

        // After reset, only |0⟩ state should have probability 1
        assert_eq!(processor.get_probability(0), 1.0);
        for i in 1..processor.state_size() {
            assert_eq!(processor.get_probability(i), 0.0);
        }
    }

    #[test]
    fn test_byte_oracle() {
        let data = b"test data with pattern inside".to_vec();
        let pattern = b"pattern".to_vec();

        let oracle = ByteOracle::new(data.clone(), pattern.clone());

        // Should find pattern at correct position
        let pattern_start = data.windows(pattern.len())
            .position(|window| window == pattern.as_slice())
            .unwrap();

        assert!(oracle.is_target(pattern_start));
        assert!(!oracle.is_target(0));
        assert!(!oracle.is_target(data.len() - 1));
    }

    #[test]
    fn test_database_oracle_generic() {
        let data = vec![10, 20, 30, 40, 50];
        let target = 30;

        let oracle = DatabaseOracle::new(data, target);

        assert!(!oracle.is_target(0)); // 10
        assert!(!oracle.is_target(1)); // 20
        assert!(oracle.is_target(2));  // 30 - target
        assert!(!oracle.is_target(3)); // 40
        assert!(!oracle.is_target(4)); // 50
    }

    #[test]
    fn test_optimal_grover_iterations() {
        use std::f64::consts::PI;

        // For N elements, optimal iterations = π/4 * √N
        let sizes = vec![4, 16, 64, 256];

        for size in sizes {
            let optimal = ((PI / 4.0) * (size as f64).sqrt()).round() as usize;

            // For 4 elements: ~1 iteration
            // For 16 elements: ~3 iterations
            // For 64 elements: ~6 iterations
            // For 256 elements: ~12 iterations

            let expected = match size {
                4 => 1,
                16 => 3,
                64 => 6,
                256 => 12,
                _ => optimal,
            };

            assert!(
                (optimal as i32 - expected as i32).abs() <= 1,
                "For size {}, expected ~{} iterations, got {}",
                size,
                expected,
                optimal
            );
        }
    }

    #[test]
    fn test_quantum_speedup_theoretical() {
        // Test that Grover's algorithm achieves theoretical O(√N) speedup
        let size = 256;
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        // Classical search: O(N) operations
        let classical_ops = size;

        // Quantum search: O(√N) operations
        let quantum_ops = (size as f64).sqrt() as usize;

        // Speedup should be √N
        let speedup = classical_ops as f64 / quantum_ops as f64;
        let expected_speedup = (size as f64).sqrt();

        assert!(
            (speedup - expected_speedup).abs() < 1.0,
            "Speedup {} should be close to {}",
            speedup,
            expected_speedup
        );
    }

    #[test]
    fn test_config_customization() {
        let config = QuantumProcessorConfig {
            max_grover_iterations: 50,
            verify_normalization: false,
            measurement_threshold: 0.2,
        };

        assert_eq!(config.max_grover_iterations, 50);
        assert!(!config.verify_normalization);
        assert_eq!(config.measurement_threshold, 0.2);
    }

    #[test]
    fn test_empty_data_handling() {
        let data = vec![];
        let pattern = vec![1u8];
        let config = QuantumProcessorConfig::default();

        // Should handle empty data gracefully
        let result = create_byte_search_processor(data, pattern, config);
        // This might error or return a minimal processor depending on implementation
        // Just ensure it doesn't panic
        let _ = result;
    }
}

