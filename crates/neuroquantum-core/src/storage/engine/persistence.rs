//! Persistence operations for `StorageEngine`
//!
//! This module handles all disk I/O operations including:
//! - Row file operations (append, rewrite)
//! - Metadata persistence
//! - Index persistence
//! - Compressed block persistence
//! - Transaction log persistence

use std::collections::{BTreeMap, HashMap};

use anyhow::{anyhow, Result};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

use super::StorageEngine;
use crate::dna::{DNACompressor, EncodedData};
use crate::storage::row::{CompressedRowEntry, Row};
use crate::storage::types::RowId;

impl StorageEngine {
    /// Load all rows for a table with DNA decompression
    pub(crate) async fn load_table_rows(&self, table: &str) -> Result<Vec<Row>> {
        let table_path = self.data_dir.join("tables").join(format!("{table}.nqdb"));

        if !table_path.exists() {
            return Ok(Vec::new());
        }

        let file_content = fs::read(&table_path).await?;

        // If file is empty, return empty vector
        if file_content.is_empty() {
            return Ok(Vec::new());
        }

        let mut rows = Vec::new();
        let mut offset = 0;

        // Try to read as binary compressed format first
        while offset < file_content.len() {
            // Check if we have enough bytes for length prefix
            if offset + 4 > file_content.len() {
                break;
            }

            // Read length prefix
            let len_bytes: [u8; 4] = file_content[offset..offset + 4]
                .try_into()
                .map_err(|_| anyhow!("Failed to read length prefix"))?;
            let entry_len = u32::from_le_bytes(len_bytes) as usize;
            offset += 4;

            // Check if we have enough bytes for entry
            if offset + entry_len > file_content.len() {
                break;
            }

            // Read and deserialize compressed entry
            let entry_bytes = &file_content[offset..offset + entry_len];
            offset += entry_len;

            match bincode::deserialize::<CompressedRowEntry>(entry_bytes) {
                | Ok(entry) => {
                    // Get the compressed data (decrypt first if encrypted)
                    let compressed_data = if let Some(ref encrypted_wrapper) =
                        entry.encrypted_wrapper
                    {
                        // Data is encrypted, decrypt it first
                        if let Some(ref encryption_manager) = self.encryption_manager {
                            match encryption_manager.decrypt(encrypted_wrapper) {
                                | Ok(decrypted_bytes) => {
                                    // Deserialize the decrypted compressed data
                                    match bincode::deserialize::<EncodedData>(&decrypted_bytes) {
                                        | Ok(data) => data,
                                        | Err(e) => {
                                            debug!("Failed to deserialize decrypted data: {}", e);
                                            continue;
                                        },
                                    }
                                },
                                | Err(e) => {
                                    debug!("Failed to decrypt row: {}", e);
                                    continue;
                                },
                            }
                        } else {
                            debug!("Encrypted data found but no encryption manager available");
                            continue;
                        }
                    } else {
                        // Data is not encrypted, use directly
                        entry.compressed_data
                    };

                    // Decompress the row data using the async decompress_row method
                    match self.decompress_row(&compressed_data).await {
                        | Ok(row) => {
                            rows.push(row);
                        },
                        | Err(e) => {
                            debug!("Failed to decompress row: {}", e);
                        },
                    }
                },
                | Err(_) => {
                    // Might be legacy JSON format, try to read as text
                    break;
                },
            }
        }

        // If no rows were read, try legacy JSON format
        if rows.is_empty() && offset == 0 {
            let content = String::from_utf8_lossy(&file_content);
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(row) = serde_json::from_str::<Row>(line) {
                    rows.push(row);
                }
            }
        }

        Ok(rows)
    }

    /// Append row to table file with DNA compression and encryption
    pub(crate) async fn append_row_to_file(&mut self, table: &str, row: &Row) -> Result<()> {
        let table_path = self.data_dir.join("tables").join(format!("{table}.nqdb"));

        // Get or create compressed data for this row
        let compressed_data = if let Some(data) = self.compressed_blocks.get(&row.id) {
            data.clone()
        } else {
            // Compress the row if not already compressed
            // Use bincode for consistency with compress_row() and decompress_row()
            let serialized =
                bincode::serialize(row).map_err(|e| anyhow!("Failed to serialize row: {e}"))?;
            let compressed = self.dna_compressor.compress(&serialized).await?;
            self.compressed_blocks.insert(row.id, compressed.clone());
            compressed
        };

        // Optionally encrypt the compressed data
        let encrypted_wrapper = if let Some(ref encryption_manager) = self.encryption_manager {
            // Serialize the compressed data
            let compressed_bytes = bincode::serialize(&compressed_data)
                .map_err(|e| anyhow!("Failed to serialize compressed data: {e}"))?;

            // Encrypt the serialized compressed data
            let encrypted = encryption_manager.encrypt(&compressed_bytes)?;
            Some(encrypted)
        } else {
            None
        };

        // Create a storage entry with metadata, compressed data, and optional encryption
        let storage_entry = CompressedRowEntry {
            row_id: row.id,
            compressed_data,
            created_at: row.created_at,
            updated_at: row.updated_at,
            encrypted_wrapper,
            format_version: 1,
        };

        // Serialize and write the entry
        let entry_bytes = bincode::serialize(&storage_entry)
            .map_err(|e| anyhow!("Failed to serialize storage entry: {e}"))?;

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&table_path)
            .await?;

        // Write length prefix (4 bytes) followed by entry data
        let len_bytes = (entry_bytes.len() as u32).to_le_bytes();
        file.write_all(&len_bytes).await?;
        file.write_all(&entry_bytes).await?;
        file.flush().await?;

        // Immediately persist compressed blocks to quantum directory
        self.save_compressed_blocks().await?;

        debug!(
            "💾 Row {} written (DNA compressed{}, {} bytes)",
            row.id,
            if self.encryption_manager.is_some() {
                " + encrypted"
            } else {
                ""
            },
            entry_bytes.len()
        );

        Ok(())
    }

    /// Rewrite table file with updated rows
    pub(crate) async fn rewrite_table_file_with_updates(
        &mut self,
        table: &str,
        updated_rows: &[Row],
    ) -> Result<()> {
        let table_path = self.data_dir.join("tables").join(format!("{table}.nqdb"));

        // Create a HashMap of updated rows for quick lookup
        let updated_map: HashMap<RowId, &Row> = updated_rows.iter().map(|r| (r.id, r)).collect();

        // Load all existing rows
        let existing_rows = self.load_table_rows(table).await?;

        // Create a temporary file
        let temp_path = table_path.with_extension("tmp");
        let mut temp_file = fs::File::create(&temp_path).await?;

        // Write all rows to temporary file (updated ones with new data, others as-is)
        for existing_row in existing_rows {
            let row_to_write = if let Some(updated_row) = updated_map.get(&existing_row.id) {
                updated_row
            } else {
                &existing_row
            };

            self.write_row_entry(&mut temp_file, row_to_write).await?;
        }

        temp_file.flush().await?;

        // Replace the original file with the temporary file
        fs::rename(&temp_path, &table_path).await?;

        // Immediately persist compressed blocks to quantum directory
        self.save_compressed_blocks().await?;

        Ok(())
    }

    /// Rewrite table file with deleted rows
    pub(crate) async fn rewrite_table_file_with_deletions(
        &mut self,
        table: &str,
        deleted_row_ids: &[RowId],
    ) -> Result<()> {
        let table_path = self.data_dir.join("tables").join(format!("{table}.nqdb"));

        // Create a temporary file
        let temp_path = table_path.with_extension("tmp");
        let mut temp_file = fs::File::create(&temp_path).await?;

        // Load existing rows
        let existing_rows = self.load_table_rows(table).await?;

        // Write rows that are not deleted to temporary file
        for row in existing_rows {
            if !deleted_row_ids.contains(&row.id) {
                self.write_row_entry(&mut temp_file, &row).await?;
            }
        }

        temp_file.flush().await?;

        // Replace the original file with the temporary file
        fs::rename(&temp_path, &table_path).await?;

        // Immediately persist compressed blocks to quantum directory
        self.save_compressed_blocks().await?;

        Ok(())
    }

    /// Helper method to write a single row entry to a file
    async fn write_row_entry(&mut self, file: &mut fs::File, row: &Row) -> Result<()> {
        // Get or create compressed data for this row
        let compressed_data = if let Some(data) = self.compressed_blocks.get(&row.id) {
            data.clone()
        } else {
            // Compress the row if not already compressed
            let serialized =
                bincode::serialize(row).map_err(|e| anyhow!("Failed to serialize row: {e}"))?;
            let compressed = self.dna_compressor.compress(&serialized).await?;
            self.compressed_blocks.insert(row.id, compressed.clone());
            compressed
        };

        // Optionally encrypt the compressed data
        let encrypted_wrapper = if let Some(ref encryption_manager) = self.encryption_manager {
            let compressed_bytes = bincode::serialize(&compressed_data)
                .map_err(|e| anyhow!("Failed to serialize compressed data: {e}"))?;
            let encrypted = encryption_manager.encrypt(&compressed_bytes)?;
            Some(encrypted)
        } else {
            None
        };

        // Create a storage entry with metadata, compressed data, and optional encryption
        let storage_entry = CompressedRowEntry {
            row_id: row.id,
            compressed_data,
            created_at: row.created_at,
            updated_at: row.updated_at,
            encrypted_wrapper,
            format_version: 1,
        };

        // Serialize and write the entry
        let entry_bytes = bincode::serialize(&storage_entry)
            .map_err(|e| anyhow!("Failed to serialize storage entry: {e}"))?;

        // Write length prefix (4 bytes) followed by entry data
        let len_bytes = (entry_bytes.len() as u32).to_le_bytes();
        file.write_all(&len_bytes).await?;
        file.write_all(&entry_bytes).await?;

        Ok(())
    }

    /// Save metadata to disk
    ///
    /// Persists the current database metadata including table schemas,
    /// auto-increment counters, and row/LSN counters.
    pub async fn save_metadata(&mut self) -> Result<()> {
        self.metadata.next_row_id = self.next_row_id;
        self.metadata.next_lsn = self.next_lsn;

        let metadata_path = self.data_dir.join("metadata.json");
        let content = serde_json::to_string_pretty(&self.metadata)?;
        fs::write(&metadata_path, content).await?;

        Ok(())
    }

    /// Save transaction log to disk
    pub(crate) async fn save_transaction_log(&self) -> Result<()> {
        let log_path = self.data_dir.join("logs").join("transaction.log");
        let content = serde_json::to_string_pretty(&self.transaction_log)?;
        fs::write(&log_path, content).await?;

        Ok(())
    }

    /// Load transaction log from disk
    pub(crate) async fn load_transaction_log(&mut self) -> Result<()> {
        let log_path = self.data_dir.join("logs").join("transaction.log");

        if log_path.exists() {
            let content = fs::read_to_string(&log_path).await?;
            if !content.trim().is_empty() {
                self.transaction_log = serde_json::from_str(&content)?;
            }
        }

        Ok(())
    }

    /// Save indexes to disk
    pub(crate) async fn save_indexes(&self) -> Result<()> {
        for (index_name, index_data) in &self.indexes {
            let index_path = self
                .data_dir
                .join("indexes")
                .join(format!("{index_name}.idx"));
            let content = serde_json::to_string_pretty(index_data)?;
            fs::write(&index_path, content).await?;
        }

        Ok(())
    }

    /// Load indexes from disk
    pub(crate) async fn load_indexes(&mut self) -> Result<()> {
        let indexes_dir = self.data_dir.join("indexes");

        if !indexes_dir.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&indexes_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("idx") {
                let content = fs::read_to_string(&path).await?;
                if !content.trim().is_empty() {
                    let index_data: BTreeMap<String, RowId> = serde_json::from_str(&content)?;

                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        self.indexes.insert(stem.to_string(), index_data);
                    }
                }
            }
        }

        Ok(())
    }

    /// Save compressed blocks to disk
    pub(crate) async fn save_compressed_blocks(&self) -> Result<()> {
        let blocks_path = self
            .data_dir
            .join("quantum")
            .join("compressed_blocks.qdata");
        // Use bincode instead of JSON because CompressedDNA contains HashMap with Vec<u8> keys
        // which cannot be serialized to JSON (JSON only supports string keys)
        let content = bincode::serialize(&self.compressed_blocks)
            .map_err(|e| anyhow!("Failed to serialize compressed blocks: {e}"))?;
        fs::write(&blocks_path, content).await?;

        Ok(())
    }

    /// Load compressed blocks from disk
    pub(crate) async fn load_compressed_blocks(&mut self) -> Result<()> {
        let blocks_path = self
            .data_dir
            .join("quantum")
            .join("compressed_blocks.qdata");

        if blocks_path.exists() {
            let content = fs::read(&blocks_path).await?;
            if !content.is_empty() {
                // Use bincode to deserialize (consistent with save_compressed_blocks)
                self.compressed_blocks = bincode::deserialize(&content)
                    .map_err(|e| anyhow!("Failed to deserialize compressed blocks: {e}"))?;
            }
        }

        Ok(())
    }

    /// Load all persistent data from disk
    ///
    /// Called during initialization to restore database state.
    pub(crate) async fn load_from_disk(&mut self) -> Result<()> {
        info!("📥 Loading existing data from disk...");

        // Load row ID and LSN counters from metadata
        self.next_row_id = self.metadata.next_row_id;
        self.next_lsn = self.metadata.next_lsn;

        // Load indexes
        self.load_indexes().await?;

        // Load transaction log
        self.load_transaction_log().await?;

        // Load compressed blocks
        self.load_compressed_blocks().await?;

        info!(
            "✅ Loaded {} tables, next_row_id: {}, next_lsn: {}",
            self.metadata.tables.len(),
            self.next_row_id,
            self.next_lsn
        );

        Ok(())
    }

    /// Flush all pending changes to disk
    ///
    /// Ensures all in-memory data is persisted:
    /// - Metadata (table schemas, counters)
    /// - Transaction log
    /// - Indexes
    /// - Compressed blocks
    pub async fn flush_to_disk(&mut self) -> Result<()> {
        info!("💾 Flushing all data to disk...");

        // Save metadata
        self.save_metadata().await?;

        // Save transaction log
        self.save_transaction_log().await?;

        // Save indexes
        self.save_indexes().await?;

        // Save compressed blocks
        self.save_compressed_blocks().await?;

        info!("✅ All data flushed to disk successfully");
        Ok(())
    }
}
