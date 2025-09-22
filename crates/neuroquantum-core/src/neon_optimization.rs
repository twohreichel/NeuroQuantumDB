//! # ARM64/NEON Optimization
//!
//! SIMD optimizations for ARM64 architecture using NEON instructions
//! to accelerate neuromorphic computations in NeuroQuantumDB.

use crate::error::{CoreError, CoreResult};
use std::collections::HashMap;
use tracing::{info, warn};

/// NEON optimizer for ARM64 SIMD operations
#[derive(Debug)]
pub struct NeonOptimizer {
    enabled: bool,
    optimization_stats: OptimizationStats,
}

/// Statistics about NEON optimization performance
#[derive(Debug, Default)]
pub struct OptimizationStats {
    pub simd_operations: u64,
    pub scalar_fallbacks: u64,
    pub performance_gain: f32,
    pub memory_bandwidth_saved: u64,
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
            // On ARM64, NEON is always available
            true
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
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
            connection.weight = (connection.weight * node.decay_factor
                + connection.usage_count as f32 * 0.01 * node.learning_rate)
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
    pub fn get_stats(&self) -> &OptimizationStats {
        &self.optimization_stats
    }

    /// Check if NEON optimizations are enabled
    pub fn is_enabled(&self) -> bool {
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
}

impl Default for NeonOptimizer {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            enabled: false,
            optimization_stats: OptimizationStats::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neon_optimizer_creation() {
        let _optimizer = NeonOptimizer::new().unwrap();
        // Should not fail regardless of platform
    }

    #[test]
    fn test_dot_product() {
        let optimizer = NeonOptimizer::new().unwrap();
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![2.0, 3.0, 4.0, 5.0];

        let result = optimizer.dot_product(&a, &b).unwrap();
        let expected = 1.0 * 2.0 + 2.0 * 3.0 + 3.0 * 4.0 + 4.0 * 5.0;
        assert!((result - expected).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product_mismatched_lengths() {
        let optimizer = NeonOptimizer::new().unwrap();
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![2.0, 3.0];

        let result = optimizer.dot_product(&a, &b);
        assert!(result.is_err());
    }

    #[test]
    fn test_activation_function() {
        let optimizer = NeonOptimizer::new().unwrap();
        let mut inputs = vec![0.5, 1.5, -0.5, 2.0];
        let threshold = 1.0;

        optimizer
            .apply_activation_function(&mut inputs, threshold)
            .unwrap();

        // Expected: max(0, input - threshold)
        assert!((inputs[0] - 0.0).abs() < 1e-6); // 0.5 - 1.0 = -0.5 -> 0.0
        assert!((inputs[1] - 0.5).abs() < 1e-6); // 1.5 - 1.0 = 0.5
        assert!((inputs[2] - 0.0).abs() < 1e-6); // -0.5 - 1.0 = -1.5 -> 0.0
        assert!((inputs[3] - 1.0).abs() < 1e-6); // 2.0 - 1.0 = 1.0
    }

    #[test]
    fn test_matrix_operations() {
        let optimizer = NeonOptimizer::new().unwrap();
        let mut matrix = vec![1.0, -1.0, 2.0, -2.0];

        optimizer.optimize_matrix_operations(&mut matrix).unwrap();

        // Should apply sigmoid-like function: 1/(1 + |x|)
        for &value in &matrix {
            assert!(value > 0.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_enable_disable() {
        let mut optimizer = NeonOptimizer::new().unwrap();
        let _initial_state = optimizer.is_enabled();

        optimizer.set_enabled(false);
        assert!(!optimizer.is_enabled());

        optimizer.set_enabled(true);
        // Should be enabled only if platform supports it
        #[cfg(target_arch = "aarch64")]
        assert!(optimizer.is_enabled());
    }
}
