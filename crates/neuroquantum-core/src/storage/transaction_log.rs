//! Transaction log types for ACID compliance
//!
//! This module defines the transaction-related types used for write-ahead logging
//! and transaction management within the storage engine.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::row::Row;
use super::types::TableSchema;

/// Transaction identifier
pub type TransactionId = Uuid;

/// Log Sequence Number for write-ahead logging
pub type LSN = u64;

/// Transaction log entry for ACID compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub operations: Vec<Operation>,
    pub status: TransactionStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub lsn: LSN,
}

/// Operations that can be recorded in the transaction log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Insert {
        table: String,
        row_id: super::types::RowId,
        data: Row,
    },
    Update {
        table: String,
        row_id: super::types::RowId,
        old_data: Row,
        new_data: Row,
    },
    Delete {
        table: String,
        row_id: super::types::RowId,
        data: Row,
    },
    CreateTable {
        schema: TableSchema,
    },
    DropTable {
        table: String,
    },
    AlterTable {
        table: String,
        old_schema: TableSchema,
        new_schema: TableSchema,
    },
}

/// Status of a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Active,
    Committed,
    Aborted,
}
