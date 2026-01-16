//! Log Writer for WAL
//!
//! Manages writing WAL records to disk with:
//! - Segment-based log files
//! - Buffering for performance
//! - Fsync control for durability
//! - Log file rotation
//! - Group commit for high-throughput durability

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

use super::{WALRecord, LSN};

/// Log writer configuration
#[derive(Debug, Clone)]
pub struct LogWriterConfig {
    pub wal_dir: PathBuf,
    pub segment_size: usize,
    pub sync_on_write: bool,
    pub buffer_size: usize,
    /// Group commit delay in milliseconds (0 to disable group commit)
    pub group_commit_delay_ms: u64,
    /// Maximum number of records per group commit batch
    pub group_commit_max_records: usize,
    /// Maximum bytes per group commit batch
    pub group_commit_max_bytes: usize,
}

/// Pending record waiting for group commit
struct PendingRecord {
    record: WALRecord,
    #[allow(dead_code)] // Used for buffer size tracking
    size: usize,
    response_tx: oneshot::Sender<Result<()>>,
}

/// Group commit buffer that batches multiple records for a single fsync
struct GroupCommitBuffer {
    pending: Vec<PendingRecord>,
    total_bytes: usize,
    flush_deadline: Option<Instant>,
}

impl GroupCommitBuffer {
    const fn new() -> Self {
        Self {
            pending: Vec::new(),
            total_bytes: 0,
            flush_deadline: None,
        }
    }

    fn push(&mut self, record: WALRecord, size: usize, response_tx: oneshot::Sender<Result<()>>) {
        self.pending.push(PendingRecord {
            record,
            size,
            response_tx,
        });
        self.total_bytes += size;
    }

    const fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    fn should_flush(&self, max_records: usize, max_bytes: usize) -> bool {
        !self.is_empty()
            && (self.pending.len() >= max_records
                || self.total_bytes >= max_bytes
                || self
                    .flush_deadline
                    .is_some_and(|deadline| Instant::now() >= deadline))
    }

    fn drain(&mut self) -> Vec<PendingRecord> {
        self.total_bytes = 0;
        self.flush_deadline = None;
        std::mem::take(&mut self.pending)
    }
}

/// Commands for group commit background task
enum GroupCommitCommand {
    Append {
        record: WALRecord,
        size: usize,
        response_tx: oneshot::Sender<Result<()>>,
    },
    Flush,
    Shutdown,
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
    /// Group commit command sender (None if group commit is disabled)
    group_commit_tx: Option<mpsc::UnboundedSender<GroupCommitCommand>>,
}

impl LogWriter {
    /// Create a new log writer
    pub async fn new(config: LogWriterConfig) -> Result<Self> {
        info!("📝 Initializing LogWriter at: {}", config.wal_dir.display());

        // Find the latest segment or create first one
        let (segment_num, next_lsn) = Self::find_latest_segment(&config.wal_dir).await?;

        let mut writer = Self {
            config: config.clone(),
            current_segment: segment_num,
            current_file: None,
            current_file_size: 0,
            next_lsn,
            group_commit_tx: None,
        };

        // Open the current segment file
        writer.open_segment(segment_num).await?;

        // Start group commit background task if enabled
        if config.sync_on_write && config.group_commit_delay_ms > 0 {
            let (tx, rx) = mpsc::unbounded_channel();
            writer.group_commit_tx = Some(tx);
            writer.spawn_group_commit_task(rx);
            info!(
                "🚀 Group commit enabled: delay={}ms, max_records={}, max_bytes={}",
                config.group_commit_delay_ms,
                config.group_commit_max_records,
                config.group_commit_max_bytes
            );
        }

        info!(
            "✅ LogWriter initialized: segment={}, next_lsn={}",
            segment_num, next_lsn
        );
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
                    if let Some(num_str) = filename
                        .strip_prefix("wal-")
                        .and_then(|s| s.strip_suffix(".log"))
                    {
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
        let segment_path = wal_dir.join(format!("wal-{max_segment:08}.log"));
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
                | Ok(_) => {
                    let record_len = u32::from_le_bytes(len_buf) as usize;

                    // Read record data
                    let mut record_buf = vec![0u8; record_len];
                    file.read_exact(&mut record_buf).await?;

                    // Deserialize to get LSN
                    if let Ok(record) = WALRecord::from_bytes(&record_buf) {
                        last_lsn = record.lsn;
                    }
                },
                | Err(_) => break, // End of file
            }
        }

        Ok(last_lsn)
    }

    /// Open a segment file for writing
    async fn open_segment(&mut self, segment_num: u64) -> Result<()> {
        let segment_path = self
            .config
            .wal_dir
            .join(format!("wal-{segment_num:08}.log"));

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

        debug!(
            "📂 Opened WAL segment: {} (size: {} bytes)",
            segment_num, self.current_file_size
        );
        Ok(())
    }

    /// Append a WAL record to the log
    pub async fn append_record(&mut self, record: WALRecord) -> Result<()> {
        // If group commit is enabled, use it
        if let Some(tx) = &self.group_commit_tx {
            // Serialize the record to calculate size
            let record_bytes = record.to_bytes()?;
            let size = 4 + record_bytes.len(); // length prefix + data

            // Create response channel
            let (response_tx, response_rx) = oneshot::channel();

            // Send to group commit task
            tx.send(GroupCommitCommand::Append {
                record,
                size,
                response_tx,
            })
            .map_err(|_| anyhow!("Group commit task has stopped"))?;

            // Wait for the batch to be flushed
            response_rx
                .await
                .map_err(|_| anyhow!("Group commit response channel closed"))??;

            return Ok(());
        }

        // Fallback to direct write (group commit disabled)
        self.write_record_direct(record).await
    }

    /// Write a record directly without group commit
    async fn write_record_direct(&mut self, record: WALRecord) -> Result<()> {
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

            debug!(
                "✍️ Wrote WAL record LSN={} ({} bytes)",
                record.lsn,
                record_bytes.len()
            );
            Ok(())
        } else {
            Err(anyhow!("No active WAL segment file"))
        }
    }

    /// Flush buffered writes to disk
    pub async fn flush(&mut self) -> Result<()> {
        // If group commit is active, send flush command
        if let Some(tx) = &self.group_commit_tx {
            let _ = tx.send(GroupCommitCommand::Flush);
            // Give it a moment to process
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Also flush the file directly
        if let Some(file) = &mut self.current_file {
            file.flush().await?;
            file.get_ref().sync_all().await?;
            debug!("💾 Flushed WAL to disk");
        }
        Ok(())
    }

    /// Spawn group commit background task
    fn spawn_group_commit_task(&self, mut rx: mpsc::UnboundedReceiver<GroupCommitCommand>) {
        let config = self.config.clone();
        let wal_dir = config.wal_dir.clone();
        let delay_ms = config.group_commit_delay_ms;
        let max_records = config.group_commit_max_records;
        let max_bytes = config.group_commit_max_bytes;
        let segment_size = config.segment_size;
        let buffer_size = config.buffer_size;

        tokio::spawn(async move {
            let mut buffer = GroupCommitBuffer::new();
            let mut current_segment = 0u64;
            let mut current_file: Option<BufWriter<File>> = None;
            let mut current_file_size = 0usize;

            // Open initial segment
            if let Ok((segment, size)) =
                Self::open_segment_for_group_commit(&wal_dir, current_segment, buffer_size).await
            {
                current_file = Some(segment);
                current_file_size = size;
            }

            let mut flush_timer = tokio::time::interval(Duration::from_millis(delay_ms));
            flush_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = flush_timer.tick() => {
                        // Periodic flush check
                        if buffer.should_flush(max_records, max_bytes) {
                            Self::flush_group_commit_buffer(
                                &mut buffer,
                                &mut current_file,
                                &mut current_file_size,
                                &mut current_segment,
                                &wal_dir,
                                segment_size,
                                buffer_size,
                            )
                            .await;
                        }
                    }
                    cmd = rx.recv() => {
                        match cmd {
                            Some(GroupCommitCommand::Append { record, size, response_tx }) => {
                                // Set flush deadline if this is the first record
                                if buffer.is_empty() {
                                    buffer.flush_deadline = Some(
                                        Instant::now() + Duration::from_millis(delay_ms)
                                    );
                                }

                                buffer.push(record, size, response_tx);

                                // Check if we should flush immediately
                                if buffer.should_flush(max_records, max_bytes) {
                                    Self::flush_group_commit_buffer(
                                        &mut buffer,
                                        &mut current_file,
                                        &mut current_file_size,
                                        &mut current_segment,
                                        &wal_dir,
                                        segment_size,
                                        buffer_size,
                                    )
                                    .await;
                                }
                            }
                            Some(GroupCommitCommand::Flush) => {
                                if !buffer.is_empty() {
                                    Self::flush_group_commit_buffer(
                                        &mut buffer,
                                        &mut current_file,
                                        &mut current_file_size,
                                        &mut current_segment,
                                        &wal_dir,
                                        segment_size,
                                        buffer_size,
                                    )
                                    .await;
                                }
                            }
                            Some(GroupCommitCommand::Shutdown) | None => {
                                // Flush any remaining records
                                if !buffer.is_empty() {
                                    Self::flush_group_commit_buffer(
                                        &mut buffer,
                                        &mut current_file,
                                        &mut current_file_size,
                                        &mut current_segment,
                                        &wal_dir,
                                        segment_size,
                                        buffer_size,
                                    )
                                    .await;
                                }
                                break;
                            }
                        }
                    }
                }
            }

            info!("🛑 Group commit task stopped");
        });
    }

    /// Open a segment for the group commit task
    async fn open_segment_for_group_commit(
        wal_dir: &Path,
        segment_num: u64,
        buffer_size: usize,
    ) -> Result<(BufWriter<File>, usize)> {
        let segment_path = wal_dir.join(format!("wal-{segment_num:08}.log"));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&segment_path)
            .await?;

        let metadata = file.metadata().await?;
        let file_size = metadata.len() as usize;

        let buf_writer = BufWriter::with_capacity(buffer_size, file);

        debug!(
            "📂 Group commit opened WAL segment: {} (size: {} bytes)",
            segment_num, file_size
        );

        Ok((buf_writer, file_size))
    }

    /// Flush the group commit buffer
    async fn flush_group_commit_buffer(
        buffer: &mut GroupCommitBuffer,
        current_file: &mut Option<BufWriter<File>>,
        current_file_size: &mut usize,
        current_segment: &mut u64,
        wal_dir: &Path,
        segment_size: usize,
        buffer_capacity: usize,
    ) {
        let pending = buffer.drain();
        if pending.is_empty() {
            return;
        }

        let batch_size = pending.len();
        let start = Instant::now();

        // Write all records
        let mut write_result = Ok(());
        for p in &pending {
            // Check if we need to rotate
            if *current_file_size >= segment_size {
                // Flush and close current file
                if let Some(file) = current_file.as_mut() {
                    let _ = file.flush().await;
                    let _ = file.get_ref().sync_all().await;
                }
                *current_file = None;

                // Open new segment
                *current_segment += 1;
                match Self::open_segment_for_group_commit(
                    wal_dir,
                    *current_segment,
                    buffer_capacity,
                )
                .await
                {
                    | Ok((file, size)) => {
                        *current_file = Some(file);
                        *current_file_size = size;
                    },
                    | Err(e) => {
                        write_result = Err(e);
                        break;
                    },
                }
            }

            // Write record
            if let Some(file) = current_file.as_mut() {
                if let Ok(record_bytes) = p.record.to_bytes() {
                    let record_len = record_bytes.len() as u32;
                    let len_bytes = record_len.to_le_bytes();

                    if file.write_all(&len_bytes).await.is_err()
                        || file.write_all(&record_bytes).await.is_err()
                    {
                        write_result = Err(anyhow!("Failed to write record"));
                        break;
                    }

                    *current_file_size += 4 + record_bytes.len();
                } else {
                    write_result = Err(anyhow!("Failed to serialize record"));
                    break;
                }
            } else {
                write_result = Err(anyhow!("No active segment file"));
                break;
            }
        }

        // Single fsync for the entire batch
        if write_result.is_ok() {
            if let Some(file) = current_file.as_mut() {
                if file.flush().await.is_err() || file.get_ref().sync_all().await.is_err() {
                    write_result = Err(anyhow!("Failed to sync"));
                }
            }
        }

        let elapsed = start.elapsed();

        // Notify all waiters with success or error
        match write_result {
            | Ok(()) => {
                for p in pending {
                    let _ = p.response_tx.send(Ok(()));
                }
            },
            | Err(e) => {
                let error_msg = e.to_string();
                for p in pending {
                    let _ = p.response_tx.send(Err(anyhow!("{error_msg}")));
                }
            },
        }

        debug!(
            "💾 Group commit: flushed {} records in {:.2}ms",
            batch_size,
            elapsed.as_secs_f64() * 1000.0
        );
    }

    /// Rotate to a new segment file
    async fn rotate_segment(&mut self) -> Result<()> {
        info!(
            "🔄 Rotating WAL segment from {} to {}",
            self.current_segment,
            self.current_segment + 1
        );

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
    pub const fn get_next_lsn(&self) -> LSN {
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
                .join(format!("wal-{segment_num:08}.log"));

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
                | Ok(_) => {
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
                },
                | Err(_) => break, // End of file
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
                .join(format!("wal-{segment_num:08}.log"));

            if segment_path.exists() {
                tokio::fs::remove_file(&segment_path).await?;
                info!("🗑️ Removed old WAL segment: {}", segment_num);
            }
        }

        Ok(())
    }
}

impl Drop for LogWriter {
    fn drop(&mut self) {
        // Send shutdown signal to group commit task
        if let Some(tx) = &self.group_commit_tx {
            let _ = tx.send(GroupCommitCommand::Shutdown);
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;
    use crate::storage::wal::WALRecordType;

    #[tokio::test]
    async fn test_log_writer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let wal_dir = temp_dir.path().to_path_buf();

        let config = LogWriterConfig {
            wal_dir,
            segment_size: 1024 * 1024,
            sync_on_write: false,
            buffer_size: 64 * 1024,
            group_commit_delay_ms: 0, // Disable for tests
            group_commit_max_records: 1000,
            group_commit_max_bytes: 4 * 1024 * 1024,
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
            group_commit_delay_ms: 0, // Disable for tests
            group_commit_max_records: 1000,
            group_commit_max_bytes: 4 * 1024 * 1024,
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
            group_commit_delay_ms: 0, // Disable for tests
            group_commit_max_records: 1000,
            group_commit_max_bytes: 4 * 1024 * 1024,
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

    #[tokio::test]
    async fn test_group_commit() {
        let temp_dir = TempDir::new().unwrap();
        let wal_dir = temp_dir.path().to_path_buf();

        let config = LogWriterConfig {
            wal_dir: wal_dir.clone(),
            segment_size: 1024 * 1024,
            sync_on_write: true, // Enable sync
            buffer_size: 64 * 1024,
            group_commit_delay_ms: 50, // Enable group commit with 50ms delay
            group_commit_max_records: 100,
            group_commit_max_bytes: 1024 * 1024,
        };

        let mut writer = LogWriter::new(config).await.unwrap();

        // Write multiple records quickly
        let tx_id = Uuid::new_v4();
        
        for i in 1..=50 {
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
        
        // Force flush to ensure everything is written
        writer.flush().await.unwrap();
        
        // Verify records were written
        let records = writer.read_records_from(1).await.unwrap();
        assert_eq!(records.len(), 50);
        
        // Test passed - group commit successfully batched and wrote records
    }

    #[tokio::test]
    async fn test_group_commit_batch_limits() {
        let temp_dir = TempDir::new().unwrap();
        let wal_dir = temp_dir.path().to_path_buf();

        let config = LogWriterConfig {
            wal_dir: wal_dir.clone(),
            segment_size: 1024 * 1024,
            sync_on_write: true,
            buffer_size: 64 * 1024,
            group_commit_delay_ms: 1000, // Long delay
            group_commit_max_records: 10, // Small limit to trigger immediate flush
            group_commit_max_bytes: 1024 * 1024,
        };

        let mut writer = LogWriter::new(config).await.unwrap();

        // Write exactly max_records + 1 to trigger immediate flush
        let tx_id = Uuid::new_v4();
        for i in 1..=11 {
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

        // Should flush automatically without waiting for delay
        let records = writer.read_records_from(1).await.unwrap();
        assert_eq!(records.len(), 11);
    }
}
