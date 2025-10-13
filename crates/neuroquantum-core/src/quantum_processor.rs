//! True Quantum Processor with Grover's Algorithm Implementation
//!
//! This module implements a real quantum state vector simulator with
//! Grover's search algorithm, oracle functions, and diffusion operators.

use crate::error::{CoreError, CoreResult};
use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Oracle trait for Grover's algorithm - marks target states
pub trait Oracle: Send + Sync {
    /// Check if the given index matches the search criterion
    fn is_target(&self, index: usize) -> bool;

    /// Apply phase flip to target states in the quantum state vector
    fn apply_phase_flip(&self, state_vector: &mut [Complex64]);
}

/// Generic oracle for searching in database
pub struct DatabaseOracle<T: PartialEq + Send + Sync> {
    database: Vec<T>,
    target: T,
}

impl<T: PartialEq + Send + Sync> DatabaseOracle<T> {
    pub fn new(database: Vec<T>, target: T) -> Self {
        Self { database, target }
    }
}

impl<T: PartialEq + Send + Sync> Oracle for DatabaseOracle<T> {
    fn is_target(&self, index: usize) -> bool {
        if index < self.database.len() {
            self.database[index] == self.target
        } else {
            false
        }
    }

    fn apply_phase_flip(&self, state_vector: &mut [Complex64]) {
        for (i, amplitude) in state_vector.iter_mut().enumerate() {
            if self.is_target(i) {
                *amplitude = -*amplitude; // Phase flip: |x⟩ → -|x⟩
            }
        }
    }
}

/// Byte pattern oracle for string/byte searching
pub struct ByteOracle {
    data: Vec<u8>,
    pattern: Vec<u8>,
}

impl ByteOracle {
    pub fn new(data: Vec<u8>, pattern: Vec<u8>) -> Self {
        Self { data, pattern }
    }
}

impl Oracle for ByteOracle {
    fn is_target(&self, index: usize) -> bool {
        if index + self.pattern.len() <= self.data.len() {
            &self.data[index..index + self.pattern.len()] == self.pattern.as_slice()
        } else {
            false
        }
    }

    fn apply_phase_flip(&self, state_vector: &mut [Complex64]) {
        for (i, amplitude) in state_vector.iter_mut().enumerate() {
            if self.is_target(i) {
                *amplitude = -*amplitude;
            }
        }
    }
}

/// Configuration for quantum processor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumProcessorConfig {
    /// Maximum iterations for Grover's algorithm
    pub max_grover_iterations: usize,
    /// Enable state normalization checks
    pub verify_normalization: bool,
    /// Probability threshold for measurement
    pub measurement_threshold: f64,
}

impl Default for QuantumProcessorConfig {
    fn default() -> Self {
        Self {
            max_grover_iterations: 1000,
            verify_normalization: true,
            measurement_threshold: 0.1,
        }
    }
}

/// Quantum Processor implementing true Grover's algorithm with state vectors
pub struct QuantumStateProcessor {
    /// Number of qubits in the quantum system
    qubits: usize,
    /// Quantum state vector: 2^n complex amplitudes
    state_vector: Vec<Complex64>,
    /// Oracle for marking target states
    oracle: Arc<dyn Oracle>,
    /// Configuration
    config: QuantumProcessorConfig,
}

impl QuantumStateProcessor {
    /// Create a new quantum processor with the given number of qubits
    pub fn new(
        qubits: usize,
        oracle: Arc<dyn Oracle>,
        config: QuantumProcessorConfig,
    ) -> CoreResult<Self> {
        if qubits == 0 || qubits > 30 {
            return Err(CoreError::invalid_operation(
                "Invalid qubit count: must be between 1 and 30",
            ));
        }

        let state_size = 1 << qubits; // 2^n states
        let state_vector = vec![Complex64::new(0.0, 0.0); state_size];

        info!(
            "Created quantum processor: {} qubits, {} states",
            qubits, state_size
        );

        Ok(Self {
            qubits,
            state_vector,
            oracle,
            config,
        })
    }

    /// Initialize all qubits in equal superposition: |ψ⟩ = 1/√N Σ|x⟩
    pub fn initialize_superposition(&mut self) -> CoreResult<()> {
        let n = self.state_vector.len();
        let amplitude = Complex64::new(1.0 / (n as f64).sqrt(), 0.0);

        for state in &mut self.state_vector {
            *state = amplitude;
        }

        debug!(
            "Initialized superposition: {} qubits, {} states",
            self.qubits, n
        );

        Ok(())
    }

    /// Apply the oracle: phase flip for target states
    pub fn apply_oracle(&mut self) -> CoreResult<()> {
        self.oracle.apply_phase_flip(&mut self.state_vector);
        debug!("Applied oracle phase flip");
        Ok(())
    }

    /// Apply the diffusion operator (inversion about average)
    /// D = 2|ψ⟩⟨ψ| - I
    pub fn apply_diffusion_operator(&mut self) -> CoreResult<()> {
        let n = self.state_vector.len();

        // Calculate average amplitude
        let sum: Complex64 = self.state_vector.iter().sum();
        let average = sum / (n as f64);

        // Inversion about average: amplitude_i = 2 * average - amplitude_i
        for amplitude in &mut self.state_vector {
            *amplitude = Complex64::new(2.0, 0.0) * average - *amplitude;
        }

        debug!("Applied diffusion operator");
        Ok(())
    }

    /// Measure the quantum state and return the index with highest probability
    pub fn measure_highest_probability(&self) -> CoreResult<usize> {
        let mut max_prob = 0.0;
        let mut max_index = 0;

        for (i, &amplitude) in self.state_vector.iter().enumerate() {
            let probability = amplitude.norm_sqr(); // |amplitude|^2
            if probability > max_prob {
                max_prob = probability;
                max_index = i;
            }
        }

        debug!(
            "Measurement: index={}, probability={:.4}",
            max_index, max_prob
        );

        Ok(max_index)
    }

    /// Measure and return all indices with probability above threshold
    pub fn measure_all_above_threshold(&self, threshold: f64) -> Vec<(usize, f64)> {
        self.state_vector
            .iter()
            .enumerate()
            .map(|(i, &amplitude)| (i, amplitude.norm_sqr()))
            .filter(|(_, prob)| *prob > threshold)
            .collect()
    }

    /// Get the probability of measuring a specific index
    pub fn get_probability(&self, index: usize) -> f64 {
        if index < self.state_vector.len() {
            self.state_vector[index].norm_sqr()
        } else {
            0.0
        }
    }

    /// Verify the quantum state is properly normalized: Σ|amplitude|^2 = 1
    pub fn verify_normalization(&self) -> bool {
        let total_probability: f64 = self.state_vector.iter().map(|a| a.norm_sqr()).sum();

        (total_probability - 1.0).abs() < 1e-10
    }

    /// Complete Grover's search algorithm
    pub fn grovers_search(&mut self) -> CoreResult<usize> {
        let n = self.state_vector.len();

        // Calculate optimal number of iterations: π/4 * √N
        let iterations = ((PI / 4.0) * (n as f64).sqrt()) as usize;
        let clamped_iterations = iterations.min(self.config.max_grover_iterations);

        info!(
            "Starting Grover's search: N={}, iterations={}",
            n, clamped_iterations
        );

        // Step 1: Initialize superposition
        self.initialize_superposition()?;

        // Step 2: Grover iterations
        for iteration in 0..clamped_iterations {
            // Apply oracle (phase flip for target)
            self.apply_oracle()?;

            // Apply diffusion operator (amplitude amplification)
            self.apply_diffusion_operator()?;

            // Verify normalization periodically
            if self.config.verify_normalization
                && iteration % 10 == 0
                && !self.verify_normalization()
            {
                warn!(
                    "Quantum state normalization error at iteration {}",
                    iteration
                );
            }
        }

        // Step 3: Measure result
        let result_index = self.measure_highest_probability()?;
        let probability = self.get_probability(result_index);

        info!(
            "Grover's search completed: found index {} with probability {:.4}",
            result_index, probability
        );

        Ok(result_index)
    }

    /// Perform Grover search and return all high-probability results
    pub fn grovers_search_multiple(&mut self) -> CoreResult<Vec<(usize, f64)>> {
        let n = self.state_vector.len();
        let iterations = ((PI / 4.0) * (n as f64).sqrt()) as usize;
        let clamped_iterations = iterations.min(self.config.max_grover_iterations);

        info!(
            "Starting Grover's multiple search: N={}, iterations={}",
            n, clamped_iterations
        );

        self.initialize_superposition()?;

        for iteration in 0..clamped_iterations {
            self.apply_oracle()?;
            self.apply_diffusion_operator()?;

            if self.config.verify_normalization
                && iteration % 10 == 0
                && !self.verify_normalization()
            {
                warn!(
                    "Quantum state normalization error at iteration {}",
                    iteration
                );
            }
        }

        // Return all results above threshold
        let threshold = self.config.measurement_threshold / (n as f64);
        let results = self.measure_all_above_threshold(threshold);

        info!(
            "Grover's multiple search completed: {} results found",
            results.len()
        );

        Ok(results)
    }

    /// Get the number of qubits
    pub fn qubit_count(&self) -> usize {
        self.qubits
    }

    /// Get the state vector size
    pub fn state_size(&self) -> usize {
        self.state_vector.len()
    }

    /// Reset the quantum state to |0⟩
    pub fn reset(&mut self) {
        for state in &mut self.state_vector {
            *state = Complex64::new(0.0, 0.0);
        }
        self.state_vector[0] = Complex64::new(1.0, 0.0);
        debug!("Quantum state reset to |0⟩");
    }
}

/// Quantum search result with detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroverSearchResult {
    pub found_index: usize,
    pub probability: f64,
    pub iterations: usize,
    pub quantum_speedup: f64,
}

/// Helper function to create a quantum processor for byte search
pub fn create_byte_search_processor(
    data: Vec<u8>,
    pattern: Vec<u8>,
    config: QuantumProcessorConfig,
) -> CoreResult<QuantumStateProcessor> {
    // Calculate required qubits
    let n = data.len();
    let qubits = (n as f64).log2().ceil() as usize;

    let oracle = Arc::new(ByteOracle::new(data, pattern));
    QuantumStateProcessor::new(qubits, oracle, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_superposition_initialization() {
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let pattern = vec![5u8];
        let config = QuantumProcessorConfig::default();

        let mut processor = create_byte_search_processor(data, pattern, config).unwrap();
        processor.initialize_superposition().unwrap();

        assert!(processor.verify_normalization());
    }

    #[test]
    fn test_oracle_phase_flip() {
        let data = vec![1u8, 2, 3, 4];
        let target = 3u8;
        let oracle = Arc::new(DatabaseOracle::new(data, target));
        let config = QuantumProcessorConfig::default();

        let mut processor = QuantumStateProcessor::new(2, oracle, config).unwrap();
        processor.initialize_superposition().unwrap();

        let initial_amplitude = processor.state_vector[2]; // Index 2 contains value 3
        processor.apply_oracle().unwrap();
        let flipped_amplitude = processor.state_vector[2];

        assert_eq!(flipped_amplitude, -initial_amplitude);
    }

    #[test]
    fn test_grovers_search() {
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let pattern = vec![5u8];
        let config = QuantumProcessorConfig::default();

        let mut processor = create_byte_search_processor(data, pattern, config).unwrap();
        let result = processor.grovers_search().unwrap();

        // Should find index 4 (value 5 is at index 4)
        assert_eq!(result, 4);
        assert!(processor.get_probability(result) > 0.5);
    }
}
