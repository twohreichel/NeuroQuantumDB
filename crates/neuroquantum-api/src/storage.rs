use crate::auth::ApiKey;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

/// Persistent storage for API keys using SQLite
#[derive(Debug, Clone)]
pub struct ApiKeyStorage {
    conn: Arc<Mutex<Connection>>,
}

impl ApiKeyStorage {
    /// Create a new API key storage with SQLite backend
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path.as_ref())
            .context("Failed to open SQLite database for API key storage")?;

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        storage.init_schema()?;
        info!("✅ API key storage initialized at: {:?}", db_path.as_ref());
        Ok(storage)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS api_keys (
                key_id TEXT PRIMARY KEY,
                key_hash TEXT NOT NULL,
                name TEXT NOT NULL,
                permissions TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_used TEXT,
                usage_count INTEGER NOT NULL DEFAULT 0,
                rate_limit_per_hour INTEGER,
                is_revoked INTEGER NOT NULL DEFAULT 0,
                revoked_at TEXT,
                revoked_by TEXT
            )",
            [],
        )?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_api_keys_name ON api_keys(name)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_api_keys_is_revoked ON api_keys(is_revoked)",
            [],
        )?;

        debug!("✅ Database schema initialized");
        Ok(())
    }

    /// Store a new API key
    pub fn store_key(&self, api_key: &ApiKey, key_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let permissions_json = serde_json::to_string(&api_key.permissions)?;

        conn.execute(
            "INSERT INTO api_keys (
                key_id, key_hash, name, permissions, expires_at, created_at,
                last_used, usage_count, rate_limit_per_hour
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                &api_key.key,
                key_hash,
                &api_key.name,
                permissions_json,
                api_key.expires_at.to_rfc3339(),
                api_key.created_at.to_rfc3339(),
                api_key.last_used.map(|dt| dt.to_rfc3339()),
                api_key.usage_count,
                api_key.rate_limit_per_hour,
            ],
        )?;

        info!(
            "💾 Stored API key: {} for {}",
            &api_key.key[..12],
            api_key.name
        );
        Ok(())
    }

    /// Retrieve API key by key_id
    pub fn get_key(&self, key_id: &str) -> Result<Option<(ApiKey, String)>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT key_id, key_hash, name, permissions, expires_at, created_at,
                    last_used, usage_count, rate_limit_per_hour, is_revoked
             FROM api_keys
             WHERE key_id = ? AND is_revoked = 0",
        )?;

        let result = stmt.query_row(params![key_id], |row| {
            let permissions_json: String = row.get(3)?;
            let permissions: Vec<String> = serde_json::from_str(&permissions_json).unwrap();

            let expires_at_str: String = row.get(4)?;
            let created_at_str: String = row.get(5)?;
            let last_used_str: Option<String> = row.get(6)?;

            let api_key = ApiKey {
                key: row.get(0)?,
                name: row.get(2)?,
                permissions,
                expires_at: DateTime::parse_from_rfc3339(&expires_at_str)
                    .unwrap()
                    .with_timezone(&Utc),
                created_at: DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&Utc),
                last_used: last_used_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                usage_count: row.get(7)?,
                rate_limit_per_hour: row.get(8)?,
            };

            let key_hash: String = row.get(1)?;
            Ok((api_key, key_hash))
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update last used timestamp and increment usage count
    pub fn update_usage(&self, key_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE api_keys
             SET last_used = ?, usage_count = usage_count + 1
             WHERE key_id = ?",
            params![Utc::now().to_rfc3339(), key_id],
        )?;

        Ok(())
    }

    /// Revoke an API key
    pub fn revoke_key(&self, key_id: &str, revoked_by: Option<&str>) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let rows_affected = conn.execute(
            "UPDATE api_keys
             SET is_revoked = 1, revoked_at = ?, revoked_by = ?
             WHERE key_id = ? AND is_revoked = 0",
            params![Utc::now().to_rfc3339(), revoked_by, key_id],
        )?;

        if rows_affected > 0 {
            info!("🗑️ Revoked API key: {}", &key_id[..12]);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all active API keys (without exposing the actual key)
    pub fn list_keys(&self) -> Result<Vec<ApiKeyInfo>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT key_id, name, permissions, expires_at, created_at,
                    last_used, usage_count, rate_limit_per_hour
             FROM api_keys
             WHERE is_revoked = 0
             ORDER BY created_at DESC",
        )?;

        let keys = stmt
            .query_map([], |row| {
                let permissions_json: String = row.get(2)?;
                let permissions: Vec<String> = serde_json::from_str(&permissions_json).unwrap();

                let expires_at_str: String = row.get(3)?;
                let created_at_str: String = row.get(4)?;
                let last_used_str: Option<String> = row.get(5)?;

                let key_id: String = row.get(0)?;
                let masked_key = format!("{}...{}", &key_id[..8], &key_id[key_id.len() - 8..]);

                Ok(ApiKeyInfo {
                    key_id: masked_key,
                    name: row.get(1)?,
                    permissions,
                    expires_at: DateTime::parse_from_rfc3339(&expires_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    last_used: last_used_str
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    usage_count: row.get(6)?,
                    rate_limit_per_hour: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(keys)
    }

    /// Check if any admin keys exist
    pub fn has_admin_keys(&self) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM api_keys
             WHERE permissions LIKE '%admin%' AND is_revoked = 0",
            [],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Clean up expired keys (mark as revoked)
    pub fn cleanup_expired_keys(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();

        let now = Utc::now().to_rfc3339();
        let rows_affected = conn.execute(
            "UPDATE api_keys
             SET is_revoked = 1, revoked_at = ?, revoked_by = 'system:expired'
             WHERE expires_at < ? AND is_revoked = 0",
            params![&now, &now],
        )?;

        if rows_affected > 0 {
            info!("🧹 Cleaned up {} expired API keys", rows_affected);
        }

        Ok(rows_affected)
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> Result<StorageStats> {
        let conn = self.conn.lock().unwrap();

        let total_keys: i64 = conn.query_row(
            "SELECT COUNT(*) FROM api_keys WHERE is_revoked = 0",
            [],
            |row| row.get(0),
        )?;

        let total_revoked: i64 = conn.query_row(
            "SELECT COUNT(*) FROM api_keys WHERE is_revoked = 1",
            [],
            |row| row.get(0),
        )?;

        let admin_keys: i64 = conn.query_row(
            "SELECT COUNT(*) FROM api_keys
             WHERE permissions LIKE '%admin%' AND is_revoked = 0",
            [],
            |row| row.get(0),
        )?;

        Ok(StorageStats {
            total_active_keys: total_keys as usize,
            total_revoked_keys: total_revoked as usize,
            admin_keys: admin_keys as usize,
        })
    }
}

/// Information about an API key (without exposing the actual key)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiKeyInfo {
    pub key_id: String, // Masked
    pub name: String,
    pub permissions: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub usage_count: u64,
    pub rate_limit_per_hour: Option<u32>,
}

/// Storage statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageStats {
    pub total_active_keys: usize,
    pub total_revoked_keys: usize,
    pub admin_keys: usize,
}
