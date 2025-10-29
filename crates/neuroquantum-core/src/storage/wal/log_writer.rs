//! Log Writer for WAL
//!
//! Manages writing WAL records to disk with:
//! - Segment-based log files
//! - Buffering for performance
//! - Fsync control for durability
//! - Log file rotation

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tracing::{debug, info, warn};

use super::{LSN, WALRecord};

/// Log writer configuration
#[derive(Debug, Clone)]
pub struct LogWriterConfig {
    pub wal_dir: PathBuf,
    pub segment_size: usize,
    pub sync_on_write: bool,
    pub buffer_size: usize,
}

/// Log Writer - handles writing WAL records to segment files
pub struct LogWriter {
    config: LogWriterConfig,
    /// Current segment number
    current_segment: u64,
    /// Current segment file
    current_file: Option<BufWriter<File>>,
    /// Current file size
    current_file_size: usize,
    /// Next LSN to assign
    next_lsn: LSN,
}

impl LogWriter {
    /// Create a new log writer
    pub async fn new(config: LogWriterConfig) -> Result<Self> {
        info!("📝 Initializing LogWriter at: {}", config.wal_dir.display());

        // Find the latest segment or create first one
        let (segment_num, next_lsn) = Self::find_latest_segment(&config.wal_dir).await?;

        let mut writer = Self {
            config,
            current_segment: segment_num,
            current_file: None,
            current_file_size: 0,
            next_lsn,
        };

        // Open the current segment file
        writer.open_segment(segment_num).await?;

        info!("✅ LogWriter initialized: segment={}, next_lsn={}", segment_num, next_lsn);
        Ok(writer)
    }

    /// Find the latest segment and determine next LSN
    async fn find_latest_segment(wal_dir: &PathBuf) -> Result<(u64, LSN)> {
        let mut entries = tokio::fs::read_dir(wal_dir).await?;
        let mut max_segment = 0u64;
        let mut found_any = false;

        while let Some(entry) = entries.next_entry().await? {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.starts_with("wal-") && filename.ends_with(".log") {
                    if let Some(num_str) = filename.strip_prefix("wal-").and_then(|s| s.strip_suffix(".log")) {
                        if let Ok(num) = num_str.parse::<u64>() {
                            max_segment = max_segment.max(num);
                            found_any = true;
                        }
                    }
                }
            }
        }

        if !found_any {
            // No segments found, start fresh
            return Ok((0, 1));
        }

        // Read the latest segment to find the last LSN
        let segment_path = wal_dir.join(format!("wal-{:08}.log", max_segment));
        let next_lsn = Self::scan_segment_for_last_lsn(&segment_path).await?;

        Ok((max_segment, next_lsn + 1))
    }

    /// Scan a segment file to find the last LSN
    async fn scan_segment_for_last_lsn(path: &PathBuf) -> Result<LSN> {
        let mut file = File::open(path).await?;
        let mut last_lsn = 0;

        loop {
            // Read record length (4 bytes)
            let mut len_buf = [0u8; 4];
            match file.read_exact(&mut len_buf).await {
                Ok(_) => {
                    let record_len = u32::from_le_bytes(len_buf) as usize;

                    // Read record data
                    let mut record_buf = vec![0u8; record_len];
                    file.read_exact(&mut record_buf).await?;

                    // Deserialize to get LSN
                    if let Ok(record) = WALRecord::from_bytes(&record_buf) {
                        last_lsn = record.lsn;
                    }
                }
                Err(_) => break, // End of file
            }
        }

        Ok(last_lsn)
    }

    /// Open a segment file for writing
    async fn open_segment(&mut self, segment_num: u64) -> Result<()> {
        let segment_path = self
            .config
            .wal_dir
            .join(format!("wal-{:08}.log", segment_num));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&segment_path)
            .await?;

        // Get current file size
        let metadata = file.metadata().await?;
        self.current_file_size = metadata.len() as usize;

        let buf_writer = BufWriter::with_capacity(self.config.buffer_size, file);
        self.current_file = Some(buf_writer);
        self.current_segment = segment_num;

        debug!("📂 Opened WAL segment: {} (size: {} bytes)", segment_num, self.current_file_size);
        Ok(())
    }

    /// Append a WAL record to the log
    pub async fn append_record(&mut self, record: WALRecord) -> Result<()> {
        // Check if we need to rotate to a new segment
        if self.current_file_size >= self.config.segment_size {
            self.rotate_segment().await?;
        }

        // Serialize the record
        let record_bytes = record.to_bytes()?;
        let record_len = record_bytes.len() as u32;

        // Write length prefix (4 bytes)
        let len_bytes = record_len.to_le_bytes();

        // Write to file
        if let Some(file) = &mut self.current_file {
            file.write_all(&len_bytes).await?;
            file.write_all(&record_bytes).await?;

            // Update file size
            self.current_file_size += 4 + record_bytes.len();

            // Sync if configured
            if self.config.sync_on_write {
                file.flush().await?;
                file.get_ref().sync_all().await?;
            }

            debug!("✍️ Wrote WAL record LSN={} ({} bytes)", record.lsn, record_bytes.len());
            Ok(())
        } else {
            Err(anyhow!("No active WAL segment file"))
        }
    }

    /// Flush buffered writes to disk
    pub async fn flush(&mut self) -> Result<()> {
        if let Some(file) = &mut self.current_file {
            file.flush().await?;
            file.get_ref().sync_all().await?;
            debug!("💾 Flushed WAL to disk");
        }
        Ok(())
    }

    /// Rotate to a new segment file
    async fn rotate_segment(&mut self) -> Result<()> {
        info!("🔄 Rotating WAL segment from {} to {}", self.current_segment, self.current_segment + 1);

        // Flush current segment
        self.flush().await?;

        // Close current file (drop will handle it)
        self.current_file = None;

        // Open new segment
        let new_segment = self.current_segment + 1;
        self.open_segment(new_segment).await?;

        Ok(())
    }

    /// Get the next LSN that will be assigned
    pub fn get_next_lsn(&self) -> LSN {
        self.next_lsn
    }

    /// Read records starting from a given LSN
    pub async fn read_records_from(&self, start_lsn: LSN) -> Result<Vec<WALRecord>> {
        let mut records = Vec::new();

        // Scan all segment files
        for segment_num in 0..=self.current_segment {
            let segment_path = self
                .config
                .wal_dir
                .join(format!("wal-{:08}.log", segment_num));

            if !segment_path.exists() {
                continue;
            }

            let segment_records = Self::read_segment(&segment_path, start_lsn).await?;
            records.extend(segment_records);
        }

        records.sort_by_key(|r| r.lsn);
        Ok(records)
    }

    /// Read all records from a segment file
    async fn read_segment(path: &PathBuf, start_lsn: LSN) -> Result<Vec<WALRecord>> {
        let mut file = File::open(path).await?;
        let mut records = Vec::new();

        loop {
            // Read record length
            let mut len_buf = [0u8; 4];
            match file.read_exact(&mut len_buf).await {
                Ok(_) => {
                    let record_len = u32::from_le_bytes(len_buf) as usize;

                    // Read record data
                    let mut record_buf = vec![0u8; record_len];
                    file.read_exact(&mut record_buf).await?;

                    // Deserialize
                    if let Ok(record) = WALRecord::from_bytes(&record_buf) {
                        // Verify checksum
                        if !record.verify_checksum() {
                            warn!("⚠️ Checksum mismatch for LSN {}", record.lsn);
                            continue;
                        }

                        // Only include records >= start_lsn
                        if record.lsn >= start_lsn {
                            records.push(record);
                        }
                    }
                }
                Err(_) => break, // End of file
            }
        }

        Ok(records)
    }

    /// Clean up old segments (called after checkpoint)
    pub async fn cleanup_old_segments(&self, keep_segments: usize) -> Result<()> {
        if self.current_segment <= keep_segments as u64 {
            return Ok(()); // Not enough segments to clean
        }

        let delete_before = self.current_segment - keep_segments as u64;

        for segment_num in 0..delete_before {
            let segment_path = self
                .config
                .wal_dir
                .join(format!("wal-{:08}.log", segment_num));

            if segment_path.exists() {
                tokio::fs::remove_file(&segment_path).await?;
                info!("🗑️ Removed old WAL segment: {}", segment_num);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::wal::WALRecordType;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_log_writer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let wal_dir = temp_dir.path().to_path_buf();

        let config = LogWriterConfig {
            wal_dir,
            segment_size: 1024 * 1024,
            sync_on_write: false,
            buffer_size: 64 * 1024,
        };

        let writer = LogWriter::new(config).await.unwrap();
        assert_eq!(writer.current_segment, 0);
        assert_eq!(writer.next_lsn, 1);
    }

    #[tokio::test]
    async fn test_append_and_read_records() {
        let temp_dir = TempDir::new().unwrap();
        let wal_dir = temp_dir.path().to_path_buf();

        let config = LogWriterConfig {
            wal_dir: wal_dir.clone(),
            segment_size: 1024 * 1024,
            sync_on_write: false,
            buffer_size: 64 * 1024,
        };

        let mut writer = LogWriter::new(config.clone()).await.unwrap();

        // Write some records
        let tx_id = Uuid::new_v4();
        for i in 1..=5 {
            let record = WALRecord::new(
                i,
                if i > 1 { Some(i - 1) } else { None },
                Some(tx_id),
                WALRecordType::Begin {
                    tx_id,
                    timestamp: chrono::Utc::now(),
                },
            );
            writer.append_record(record).await.unwrap();
        }

        writer.flush().await.unwrap();

        // Read back records
        let records = writer.read_records_from(1).await.unwrap();
        assert_eq!(records.len(), 5);
        assert_eq!(records[0].lsn, 1);
        assert_eq!(records[4].lsn, 5);
    }

    #[tokio::test]
    async fn test_segment_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let wal_dir = temp_dir.path().to_path_buf();

        let config = LogWriterConfig {
            wal_dir: wal_dir.clone(),
            segment_size: 512, // Small size to force rotation
            sync_on_write: false,
            buffer_size: 256,
        };

        let mut writer = LogWriter::new(config).await.unwrap();
        let initial_segment = writer.current_segment;

        // Write enough records to trigger rotation
        let tx_id = Uuid::new_v4();
        for i in 1..=20 {
            let record = WALRecord::new(
                i,
                None,
                Some(tx_id),
                WALRecordType::Begin {
                    tx_id,
                    timestamp: chrono::Utc::now(),
                },
            );
            writer.append_record(record).await.unwrap();
        }

        // Should have rotated to a new segment
        assert!(writer.current_segment > initial_segment);
    }
}

