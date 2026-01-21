//! ID generation strategies for `NeuroQuantumDB`
//!
//! This module provides different strategies for generating unique identifiers
//! for database rows, including auto-increment, UUID, and Snowflake patterns.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Strategy for automatic ID generation
///
/// This determines how unique identifiers are generated for new rows.
/// Choose based on your use case:
/// - `AutoIncrement`: Best for single-instance, high-performance scenarios
/// - `Uuid`: Best for distributed systems or when IDs should be unpredictable
/// - `Snowflake`: Best for distributed systems requiring sortable IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IdGenerationStrategy {
    /// Sequential auto-incrementing integer (1, 2, 3, ...)
    ///
    /// **Pros:**
    /// - Minimal storage (8 bytes)
    /// - Excellent B+ tree performance (sequential inserts)
    /// - Human-readable and debuggable
    /// - Perfect for synaptic/neural ID references
    ///
    /// **Cons:**
    /// - Predictable (potential security concern for public APIs)
    /// - Single point of generation (not ideal for distributed systems)
    #[default]
    AutoIncrement,

    /// UUID v4 (random 128-bit identifier)
    ///
    /// **Pros:**
    /// - Globally unique without coordination
    /// - Unpredictable (good for security)
    /// - Works in distributed systems
    ///
    /// **Cons:**
    /// - Larger storage (16 bytes)
    /// - Poor B+ tree performance (random distribution causes page splits)
    /// - Not human-readable
    Uuid,

    /// Snowflake-style ID (64-bit time-based with machine ID)
    ///
    /// **Pros:**
    /// - Time-sortable (roughly ordered by creation time)
    /// - Distributed generation with machine ID
    /// - Same storage as auto-increment (8 bytes)
    ///
    /// **Cons:**
    /// - Requires time synchronization
    /// - More complex implementation
    Snowflake {
        /// Machine/node identifier (0-1023)
        machine_id: u16,
    },
}

/// Auto-increment column configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoIncrementConfig {
    /// Column name that uses auto-increment
    pub column_name: String,
    /// Current next value to be assigned
    pub next_value: i64,
    /// Increment step (default: 1)
    pub increment_by: i64,
    /// Minimum value (default: 1)
    pub min_value: i64,
    /// Maximum value (default: `i64::MAX`)
    pub max_value: i64,
    /// Whether to cycle when max is reached
    pub cycle: bool,
}

impl Default for AutoIncrementConfig {
    fn default() -> Self {
        Self {
            column_name: "id".to_string(),
            next_value: 1,
            increment_by: 1,
            min_value: 1,
            max_value: i64::MAX,
            cycle: false,
        }
    }
}

impl AutoIncrementConfig {
    /// Create a new auto-increment config for a column
    pub fn new(column_name: impl Into<String>) -> Self {
        Self {
            column_name: column_name.into(),
            ..Default::default()
        }
    }

    /// Set the starting value
    #[must_use]
    pub const fn start_with(mut self, start: i64) -> Self {
        self.next_value = start;
        self
    }

    /// Set the increment step
    #[must_use]
    pub const fn increment_by(mut self, step: i64) -> Self {
        self.increment_by = step;
        self
    }

    /// Generate the next ID and advance the counter
    ///
    /// # Errors
    ///
    /// Returns an error if the counter would overflow and cycling is disabled.
    pub fn next_id(&mut self) -> Result<i64> {
        let current = self.next_value;

        // Check for overflow
        if self.increment_by > 0 && current > self.max_value - self.increment_by {
            if self.cycle {
                self.next_value = self.min_value;
            } else {
                return Err(anyhow!(
                    "Auto-increment column '{}' has reached maximum value {}",
                    self.column_name,
                    self.max_value
                ));
            }
        } else {
            self.next_value = current + self.increment_by;
        }

        Ok(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_increment_config_default() {
        let config = AutoIncrementConfig::default();
        assert_eq!(config.next_value, 1);
        assert_eq!(config.increment_by, 1);
        assert_eq!(config.min_value, 1);
        assert_eq!(config.max_value, i64::MAX);
        assert!(!config.cycle);
    }

    #[test]
    fn test_auto_increment_next_id() {
        let mut config = AutoIncrementConfig::new("id");

        // Generate sequential IDs
        assert_eq!(config.next_id().unwrap(), 1);
        assert_eq!(config.next_id().unwrap(), 2);
        assert_eq!(config.next_id().unwrap(), 3);
        assert_eq!(config.next_value, 4);
    }

    #[test]
    fn test_auto_increment_custom_start() {
        let mut config = AutoIncrementConfig::new("user_id").start_with(1000);

        assert_eq!(config.next_id().unwrap(), 1000);
        assert_eq!(config.next_id().unwrap(), 1001);
    }

    #[test]
    fn test_auto_increment_custom_increment() {
        let mut config = AutoIncrementConfig::new("id").increment_by(10);

        assert_eq!(config.next_id().unwrap(), 1);
        assert_eq!(config.next_id().unwrap(), 11);
        assert_eq!(config.next_id().unwrap(), 21);
    }

    #[test]
    fn test_id_generation_strategy_default() {
        let strategy = IdGenerationStrategy::default();
        assert_eq!(strategy, IdGenerationStrategy::AutoIncrement);
    }
}
