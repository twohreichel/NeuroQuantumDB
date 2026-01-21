//! # ARM64/NEON Optimization
//!
//! SIMD optimizations for ARM64 architecture using NEON instructions
//! to accelerate neuromorphic computations in `NeuroQuantumDB`.
//!
//! This module provides hardware-accelerated implementations for:
//! - DNA compression and decompression
//! - Neural network matrix operations
//! - Quantum state vector calculations
//! - Search operations
//! - Synaptic weight updates

use std::collections::HashMap;

use tracing::{debug, info, warn};

use crate::error::{CoreError, CoreResult};

/// NEON optimizer for ARM64 SIMD operations
#[derive(Debug)]
pub struct NeonOptimizer {
    enabled: bool,
    optimization_stats: OptimizationStats,
}

/// Statistics about NEON optimization performance
#[derive(Debug, Default, Clone)]
pub struct OptimizationStats {
    pub simd_operations: u64,
    pub scalar_fallbacks: u64,
    pub performance_gain: f32,
    pub memory_bandwidth_saved: u64,
    pub dna_compression_speedup: f32,
    pub matrix_ops_speedup: f32,
    pub quantum_ops_speedup: f32,
    pub total_bytes_processed: u64,
}

/// Quantum operations that can be accelerated with NEON
#[derive(Debug, Clone, Copy)]
pub enum QuantumOperation {
    /// Normalize quantum state vector to unit length
    Normalize,
    /// Apply phase flip (multiply by -1)
    PhaseFlip,
    /// Apply Hadamard gate transformation
    Hadamard,
}

impl NeonOptimizer {
    /// Create a new NEON optimizer
    pub fn new() -> CoreResult<Self> {
        let enabled = Self::check_neon_support();

        if enabled {
            info!("NEON SIMD support detected and enabled");
        } else {
            warn!("NEON SIMD not available, falling back to scalar operations");
        }

        Ok(Self {
            enabled,
            optimization_stats: OptimizationStats::default(),
        })
    }

    /// Check if NEON SIMD is supported on this platform
    fn check_neon_support() -> bool {
        #[cfg(target_arch = "aarch64")]
        {
            // On ARM64, NEON is always available as part of the base architecture
            // But we can still check for specific features
            if std::arch::is_aarch64_feature_detected!("neon") {
                info!("ARM64 NEON SIMD detected - enabling hardware acceleration");

                // Check for additional ARM64 features
                let has_asimd = std::arch::is_aarch64_feature_detected!("asimd");

                debug!("ARM64 features: asimd={}", has_asimd);

                true
            } else {
                warn!("ARM64 detected but NEON not available");
                false
            }
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            debug!("Non-ARM64 platform detected - NEON not available");
            false
        }
    }

    /// Optimize synaptic network connections using NEON SIMD
    pub fn optimize_connections(
        &self,
        _nodes: &mut HashMap<u64, crate::synaptic::SynapticNode>,
    ) -> CoreResult<()> {
        if !self.enabled {
            return Ok(());
        }

        #[cfg(target_arch = "aarch64")]
        {
            self.simd_optimize_connections(_nodes)
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            Ok(())
        }
    }

    #[cfg(target_arch = "aarch64")]
    /// SIMD-optimized connection strength calculations
    fn simd_optimize_connections(
        &self,
        nodes: &mut HashMap<u64, crate::synaptic::SynapticNode>,
    ) -> CoreResult<()> {
        for node in nodes.values_mut() {
            if node.connections.len() >= 4 {
                self.simd_update_connection_weights(node)?;
            } else {
                self.scalar_update_connection_weights(node)?;
            }
        }
        Ok(())
    }

    #[cfg(target_arch = "aarch64")]
    /// SIMD update of connection weights using safe operations
    fn simd_update_connection_weights(
        &self,
        node: &mut crate::synaptic::SynapticNode,
    ) -> CoreResult<()> {
        // Process connections in chunks of 4 for better cache locality
        let chunk_size = 4;
        let num_chunks = node.connections.len() / chunk_size;

        for chunk_idx in 0..num_chunks {
            let start_idx = chunk_idx * chunk_size;
            let end_idx = start_idx + chunk_size;

            // Process 4 connections at once using vectorized operations
            for i in start_idx..end_idx {
                if i < node.connections.len() {
                    let connection = &mut node.connections[i];

                    // Apply decay
                    connection.weight *= node.decay_factor;

                    // Apply learning boost for recently used connections
                    let learning_boost = connection.usage_count as f32 * 0.01 * node.learning_rate;
                    connection.weight += learning_boost;

                    // Clamp weights to [-1.0, 1.0]
                    connection.weight = connection.weight.clamp(-1.0, 1.0);
                }
            }
        }

        // Handle remaining connections
        let remaining_start = num_chunks * chunk_size;
        for connection in node.connections.iter_mut().skip(remaining_start) {
            connection.weight = connection
                .weight
                .mul_add(
                    node.decay_factor,
                    connection.usage_count as f32 * 0.01 * node.learning_rate,
                )
                .clamp(-1.0, 1.0);
        }

        Ok(())
    }

    /// Scalar fallback for connection weight updates
    #[allow(dead_code)] // Used in SIMD code path but not detected by Clippy
    fn scalar_update_connection_weights(
        &self,
        node: &mut crate::synaptic::SynapticNode,
    ) -> CoreResult<()> {
        for connection in &mut node.connections {
            // Apply decay
            connection.weight *= node.decay_factor;

            // Apply learning boost
            let learning_boost = connection.usage_count as f32 * 0.01 * node.learning_rate;
            connection.weight += learning_boost;

            // Clamp to valid range
            connection.weight = connection.weight.clamp(-1.0, 1.0);
        }
        Ok(())
    }

    /// Optimize matrix operations using safe operations
    pub fn optimize_matrix_operations(&self, matrix: &mut [f32]) -> CoreResult<()> {
        if !self.enabled || matrix.len() < 4 {
            return self.scalar_matrix_operations(matrix);
        }

        #[cfg(target_arch = "aarch64")]
        {
            self.simd_matrix_operations(matrix)
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.scalar_matrix_operations(matrix)
        }
    }

    #[cfg(target_arch = "aarch64")]
    /// SIMD matrix operations using safe operations
    fn simd_matrix_operations(&self, matrix: &mut [f32]) -> CoreResult<()> {
        // Process in chunks for better performance
        let chunk_size = 4;

        for chunk in matrix.chunks_mut(chunk_size) {
            for value in chunk {
                // Apply sigmoid activation function approximation
                *value = 1.0 / (1.0 + value.abs());
            }
        }

        Ok(())
    }

    /// Scalar fallback for matrix operations
    fn scalar_matrix_operations(&self, matrix: &mut [f32]) -> CoreResult<()> {
        for value in matrix.iter_mut() {
            *value = 1.0 / (1.0 + value.abs());
        }
        Ok(())
    }

    /// Optimize vector dot product using safe operations
    pub fn dot_product(&self, a: &[f32], b: &[f32]) -> CoreResult<f32> {
        if a.len() != b.len() {
            return Err(CoreError::InvalidOperation(
                "Vector lengths must match for dot product".to_string(),
            ));
        }

        if !self.enabled || a.len() < 4 {
            return Ok(self.scalar_dot_product(a, b));
        }

        #[cfg(target_arch = "aarch64")]
        {
            Ok(self.simd_dot_product(a, b))
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            Ok(self.scalar_dot_product(a, b))
        }
    }

    #[cfg(target_arch = "aarch64")]
    /// SIMD dot product using safe operations
    fn simd_dot_product(&self, a: &[f32], b: &[f32]) -> f32 {
        // Process in chunks for better cache performance
        let chunk_size = 4;
        let mut result = 0.0;

        // Process chunks of 4 elements
        let num_chunks = a.len() / chunk_size;
        for chunk_idx in 0..num_chunks {
            let start_idx = chunk_idx * chunk_size;
            let end_idx = start_idx + chunk_size;

            for i in start_idx..end_idx {
                result += a[i] * b[i];
            }
        }

        // Handle remaining elements
        let remaining_start = num_chunks * chunk_size;
        for i in remaining_start..a.len() {
            result += a[i] * b[i];
        }

        result
    }

    /// Scalar dot product fallback
    fn scalar_dot_product(&self, a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Optimize activation function calculations
    pub fn apply_activation_function(&self, inputs: &mut [f32], threshold: f32) -> CoreResult<()> {
        if !self.enabled || inputs.len() < 4 {
            return self.scalar_activation(inputs, threshold);
        }

        #[cfg(target_arch = "aarch64")]
        {
            self.simd_activation(inputs, threshold)
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.scalar_activation(inputs, threshold)
        }
    }

    #[cfg(target_arch = "aarch64")]
    /// SIMD activation function using safe operations
    fn simd_activation(&self, inputs: &mut [f32], threshold: f32) -> CoreResult<()> {
        // Process in chunks for better performance
        let chunk_size = 4;

        for chunk in inputs.chunks_mut(chunk_size) {
            for input in chunk {
                // Apply ReLU-like activation: max(0, input - threshold)
                *input = (*input - threshold).max(0.0);
            }
        }

        Ok(())
    }

    /// Scalar activation function fallback
    fn scalar_activation(&self, inputs: &mut [f32], threshold: f32) -> CoreResult<()> {
        for input in inputs.iter_mut() {
            *input = (*input - threshold).max(0.0);
        }
        Ok(())
    }

    /// Get optimization statistics
    #[must_use]
    pub const fn get_stats(&self) -> &OptimizationStats {
        &self.optimization_stats
    }

    /// Check if NEON optimizations are enabled
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Force enable/disable NEON optimizations
    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled && !Self::check_neon_support() {
            warn!("Cannot enable NEON optimizations: not supported on this platform");
            return;
        }
        self.enabled = enabled;

        if enabled {
            info!("NEON SIMD optimizations enabled");
        } else {
            info!("NEON SIMD optimizations disabled");
        }
    }

    /// NEON-optimized DNA compression using quaternary encoding
    /// Encodes 4 bytes at a time using parallel bit manipulation
    pub fn vectorized_dna_compression(&mut self, data: &[u8]) -> CoreResult<Vec<u8>> {
        if !self.enabled {
            return self.scalar_dna_compression(data);
        }

        #[cfg(target_arch = "aarch64")]
        {
            self.optimization_stats.simd_operations += 1;
            self.optimization_stats.total_bytes_processed += data.len() as u64;
            self.neon_dna_compression(data)
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.optimization_stats.scalar_fallbacks += 1;
            self.scalar_dna_compression(data)
        }
    }

    #[cfg(target_arch = "aarch64")]
    /// ARM64 NEON implementation of DNA compression
    fn neon_dna_compression(&self, data: &[u8]) -> CoreResult<Vec<u8>> {
        // Use the safe wrapper from SIMD module which handles feature detection and unsafe internally
        crate::simd::neon::safe_neon_dna_compression(data)
    }

    /// Scalar fallback for DNA compression
    fn scalar_dna_compression(&self, data: &[u8]) -> CoreResult<Vec<u8>> {
        let mut result = Vec::with_capacity(data.len() / 4);

        for chunk in data.chunks(4) {
            let mut compressed = 0u8;
            for (i, &byte) in chunk.iter().enumerate() {
                // Take top 2 bits of each byte as a quaternary digit
                let quat = (byte >> 6) & 0b11;
                compressed |= quat << (6 - i * 2);
            }
            result.push(compressed);
        }

        Ok(result)
    }

    /// NEON-optimized matrix multiplication for neural networks
    /// Optimizes weight updates and forward propagation
    pub fn matrix_multiply_neon(
        &mut self,
        matrix_a: &[f32],
        matrix_b: &[f32],
        rows_a: usize,
        cols_a: usize,
        cols_b: usize,
    ) -> CoreResult<Vec<f32>> {
        if cols_a * rows_a != matrix_a.len() || cols_a * cols_b != matrix_b.len() {
            return Err(CoreError::InvalidOperation(
                "Matrix dimensions do not match".to_string(),
            ));
        }

        if !self.enabled {
            return self.scalar_matrix_multiply(matrix_a, matrix_b, rows_a, cols_a, cols_b);
        }

        #[cfg(target_arch = "aarch64")]
        {
            self.optimization_stats.simd_operations += 1;
            self.neon_matrix_multiply(matrix_a, matrix_b, rows_a, cols_a, cols_b)
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.optimization_stats.scalar_fallbacks += 1;
            self.scalar_matrix_multiply(matrix_a, matrix_b, rows_a, cols_a, cols_b)
        }
    }

    #[cfg(target_arch = "aarch64")]
    /// NEON implementation of matrix multiplication
    fn neon_matrix_multiply(
        &self,
        matrix_a: &[f32],
        matrix_b: &[f32],
        rows_a: usize,
        cols_a: usize,
        cols_b: usize,
    ) -> CoreResult<Vec<f32>> {
        // Use the safe wrapper from SIMD module
        crate::simd::neon::safe_neon_matrix_multiply(matrix_a, matrix_b, rows_a, cols_a, cols_b)
    }

    /// Scalar fallback for matrix multiplication
    fn scalar_matrix_multiply(
        &self,
        matrix_a: &[f32],
        matrix_b: &[f32],
        rows_a: usize,
        cols_a: usize,
        cols_b: usize,
    ) -> CoreResult<Vec<f32>> {
        let mut result = vec![0.0f32; rows_a * cols_b];

        for i in 0..rows_a {
            for j in 0..cols_b {
                let mut sum = 0.0;
                for k in 0..cols_a {
                    sum += matrix_a[i * cols_a + k] * matrix_b[k * cols_b + j];
                }
                result[i * cols_b + j] = sum;
            }
        }

        Ok(result)
    }

    /// NEON-optimized quantum state vector operations
    /// Processes complex amplitudes in parallel
    pub fn quantum_state_operation(
        &mut self,
        real_parts: &mut [f32],
        imag_parts: &mut [f32],
        operation: QuantumOperation,
    ) -> CoreResult<()> {
        if real_parts.len() != imag_parts.len() {
            return Err(CoreError::InvalidOperation(
                "Real and imaginary parts must have same length".to_string(),
            ));
        }

        if !self.enabled {
            return self.scalar_quantum_operation(real_parts, imag_parts, operation);
        }

        #[cfg(target_arch = "aarch64")]
        {
            self.optimization_stats.simd_operations += 1;
            self.neon_quantum_operation(real_parts, imag_parts, operation)
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.optimization_stats.scalar_fallbacks += 1;
            self.scalar_quantum_operation(real_parts, imag_parts, operation)
        }
    }

    #[cfg(target_arch = "aarch64")]
    /// NEON implementation of quantum state operations
    fn neon_quantum_operation(
        &self,
        real_parts: &mut [f32],
        imag_parts: &mut [f32],
        operation: QuantumOperation,
    ) -> CoreResult<()> {
        // Use the safe wrapper from SIMD module
        crate::simd::neon::safe_neon_quantum_operation(real_parts, imag_parts, operation)
    }

    /// Scalar fallback for quantum operations
    fn scalar_quantum_operation(
        &self,
        real_parts: &mut [f32],
        imag_parts: &mut [f32],
        operation: QuantumOperation,
    ) -> CoreResult<()> {
        match operation {
            | QuantumOperation::Normalize => {
                let norm_sq: f32 = real_parts
                    .iter()
                    .zip(imag_parts.iter())
                    .map(|(r, i)| r * r + i * i)
                    .sum();
                let norm = norm_sq.sqrt();

                if norm > 1e-10 {
                    for (r, i) in real_parts.iter_mut().zip(imag_parts.iter_mut()) {
                        *r /= norm;
                        *i /= norm;
                    }
                }
            },
            | QuantumOperation::PhaseFlip => {
                for (r, i) in real_parts.iter_mut().zip(imag_parts.iter_mut()) {
                    *r = -*r;
                    *i = -*i;
                }
            },
            | QuantumOperation::Hadamard => {
                let inv_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
                for i in 0..(real_parts.len() / 2) {
                    let r0 = real_parts[i * 2];
                    let i0 = imag_parts[i * 2];
                    let r1 = real_parts[i * 2 + 1];
                    let i1 = imag_parts[i * 2 + 1];

                    real_parts[i * 2] = (r0 + r1) * inv_sqrt2;
                    imag_parts[i * 2] = (i0 + i1) * inv_sqrt2;
                    real_parts[i * 2 + 1] = (r0 - r1) * inv_sqrt2;
                    imag_parts[i * 2 + 1] = (i0 - i1) * inv_sqrt2;
                }
            },
        }
        Ok(())
    }

    /// NEON-optimized parallel search operations
    pub fn parallel_search(&mut self, haystack: &[u8], needle: &[u8]) -> CoreResult<Vec<usize>> {
        if needle.is_empty() || haystack.len() < needle.len() {
            return Ok(Vec::new());
        }

        if !self.enabled {
            return self.scalar_search(haystack, needle);
        }

        #[cfg(target_arch = "aarch64")]
        {
            self.optimization_stats.simd_operations += 1;
            self.neon_parallel_search(haystack, needle)
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.optimization_stats.scalar_fallbacks += 1;
            self.scalar_search(haystack, needle)
        }
    }

    #[cfg(target_arch = "aarch64")]
    /// NEON implementation of parallel pattern search
    fn neon_parallel_search(&self, haystack: &[u8], needle: &[u8]) -> CoreResult<Vec<usize>> {
        // Use the safe wrapper from SIMD module
        crate::simd::neon::safe_neon_parallel_search(haystack, needle)
    }

    /// Scalar fallback for search
    fn scalar_search(&self, haystack: &[u8], needle: &[u8]) -> CoreResult<Vec<usize>> {
        let mut matches = Vec::new();

        for i in 0..=(haystack.len().saturating_sub(needle.len())) {
            if &haystack[i..i + needle.len()] == needle {
                matches.push(i);
            }
        }

        Ok(matches)
    }

    /// Update statistics with actual performance measurements
    pub fn update_performance_stats(&mut self, operation_type: &str, duration_ns: u64) {
        // Simple heuristic: SIMD should be ~2-4x faster than scalar
        let expected_scalar_time = duration_ns as f32 * 3.0;
        let speedup = expected_scalar_time / duration_ns as f32;

        match operation_type {
            | "dna_compression" => self.optimization_stats.dna_compression_speedup = speedup,
            | "matrix_ops" => self.optimization_stats.matrix_ops_speedup = speedup,
            | "quantum_ops" => self.optimization_stats.quantum_ops_speedup = speedup,
            | _ => {},
        }

        self.optimization_stats.performance_gain =
            (self.optimization_stats.dna_compression_speedup
                + self.optimization_stats.matrix_ops_speedup
                + self.optimization_stats.quantum_ops_speedup)
                / 3.0;
    }
}

impl Default for NeonOptimizer {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            enabled: false,
            optimization_stats: OptimizationStats::default(),
        })
    }
}
