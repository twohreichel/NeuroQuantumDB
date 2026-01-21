//! Query execution statistics and database metadata
//!
//! This module provides types for tracking query performance and database state.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::transaction_log::LSN;
use super::types::{RowId, TableSchema};

/// Query execution statistics for monitoring and optimization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryExecutionStats {
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of cache misses
    pub cache_misses: usize,
    /// Indexes used in the query
    pub indexes_used: Vec<String>,
    /// Whether index was actually used for query optimization
    pub index_scan: bool,
    /// Number of rows examined from storage
    pub rows_examined: usize,
}

impl QueryExecutionStats {
    /// Calculate the cache hit rate as a percentage
    ///
    /// Returns `None` if no cache accesses have been recorded.
    #[must_use]
    pub fn cache_hit_rate(&self) -> Option<f32> {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            Some(self.cache_hits as f32 / total as f32)
        } else {
            None
        }
    }
}

/// Database metadata persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetadata {
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_backup: Option<chrono::DateTime<chrono::Utc>>,
    pub tables: HashMap<String, TableSchema>,
    pub next_row_id: RowId,
    pub next_lsn: LSN,
}
