//! Row types for the storage engine
//!
//! This module defines the `Row` type which represents a database row,
//! and the internal `CompressedRowEntry` for binary storage format.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::types::{RowId, Value};
use crate::dna::EncodedData;
use crate::storage::encryption::EncryptedData;

/// Database row containing field values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub id: RowId,
    pub fields: HashMap<String, Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Compressed row entry for binary storage format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CompressedRowEntry {
    pub row_id: RowId,
    pub compressed_data: EncodedData,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Encrypted wrapper for additional security (optional)
    pub encrypted_wrapper: Option<EncryptedData>,
    /// Format version for backward compatibility
    pub format_version: u32,
}
