use crate::permissions::Permission;
use crate::storage::{ApiKeyInfo, ApiKeyStorage, StorageStats};
#[cfg(not(test))]
use bcrypt::DEFAULT_COST;
use bcrypt::{hash, verify};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

// For testing, use a lower cost to speed up tests
#[cfg(test)]
const TEST_BCRYPT_COST: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub usage_count: u64,
    pub rate_limit_per_hour: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct AuthService {
    // Persistent storage for API keys
    storage: ApiKeyStorage,
    // Rate limiting tracking (in-memory for performance)
    usage_tracking: HashMap<String, Vec<DateTime<Utc>>>,
    // Track API key generation attempts per IP
    key_generation_tracking: HashMap<String, Vec<DateTime<Utc>>>,
}

impl AuthService {
    /// Create a new AuthService with persistent storage
    /// Storage path defaults to .neuroquantum/api_keys.db
    pub fn new() -> Result<Self, String> {
        Self::new_with_path(".neuroquantum/api_keys.db")
    }

    /// Create a new AuthService with custom storage path
    pub fn new_with_path(db_path: &str) -> Result<Self, String> {
        // Ensure directory exists
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create storage directory: {}", e))?;
        }

        let storage = ApiKeyStorage::new(db_path)
            .map_err(|e| format!("Failed to initialize API key storage: {}", e))?;

        // Cleanup expired keys on startup
        if let Err(e) = storage.cleanup_expired_keys() {
            warn!("Failed to cleanup expired keys: {}", e);
        }

        let service = Self {
            storage,
            usage_tracking: HashMap::new(),
            key_generation_tracking: HashMap::new(),
        };

        info!("🔧 AuthService initialized with persistent storage");

        // Check if we have admin keys
        if !service.has_admin_keys() {
            warn!("⚠️  No admin keys found!");
            warn!("💡 Run 'neuroquantum-api init' to create your first admin key");
        }

        Ok(service)
    }

    /// Check if any admin keys exist
    pub fn has_admin_keys(&self) -> bool {
        self.storage.has_admin_keys().unwrap_or(false)
    }

    /// Create an admin key - only allowed if no admin keys exist yet (setup mode)
    pub fn create_initial_admin_key(
        &mut self,
        name: String,
        expiry_hours: Option<u32>,
    ) -> Result<ApiKey, String> {
        if self.has_admin_keys() {
            return Err(
                "Admin key already exists. Use API endpoints to create additional keys."
                    .to_string(),
            );
        }

        let admin_key = self.generate_api_key(
            name,
            Permission::admin_permissions(),
            expiry_hours,
            Some(10000), // High rate limit for admin
        )?;

        info!("🔐 Initial admin API key created: {}", &admin_key.key[..12]);
        warn!("⚠️ SECURITY: Store this key securely - it will not be shown again!");
        Ok(admin_key)
    }

    pub fn generate_api_key(
        &mut self,
        name: String,
        permissions: Vec<String>,
        expiry_hours: Option<u32>,
        rate_limit_per_hour: Option<u32>,
    ) -> Result<ApiKey, String> {
        let key = format!("nqdb_{}", Uuid::new_v4().to_string().replace('-', ""));

        // Use lower cost for tests to speed up execution
        #[cfg(test)]
        let cost = TEST_BCRYPT_COST;
        #[cfg(not(test))]
        let cost = DEFAULT_COST;

        let key_hash = hash(&key, cost).map_err(|e| format!("Failed to hash API key: {}", e))?;

        let expires_at = match expiry_hours {
            Some(hours) => Utc::now() + chrono::Duration::hours(hours as i64),
            None => Utc::now() + chrono::Duration::days(30), // Default 30 days
        };

        let api_key = ApiKey {
            key: key.clone(),
            name: name.clone(),
            permissions,
            expires_at,
            created_at: Utc::now(),
            last_used: None,
            usage_count: 0,
            rate_limit_per_hour,
        };

        // Store in persistent database
        self.storage
            .store_key(&api_key, &key_hash)
            .map_err(|e| format!("Failed to store API key: {}", e))?;

        self.usage_tracking.insert(key.clone(), Vec::new());

        info!("🔑 Generated new API key: {} for {}", &key[..12], name);
        Ok(api_key)
    }

    pub async fn validate_api_key(&self, key: &str) -> Option<ApiKey> {
        // Retrieve from persistent storage
        let (api_key_data, stored_hash) = match self.storage.get_key(key) {
            Ok(Some(data)) => data,
            Ok(None) => {
                warn!("API key not found: {}", &key[..8.min(key.len())]);
                return None;
            }
            Err(e) => {
                warn!("Failed to retrieve API key: {}", e);
                return None;
            }
        };

        // Verify the key hash for additional security
        // NOTE: bcrypt::verify is designed to be constant-time and resistant to timing attacks
        // The comparison takes the same amount of time regardless of where differences occur
        if !verify(key, &stored_hash).unwrap_or(false) {
            warn!("API key hash verification failed: {}", &key[..8]);
            return None;
        }

        // Check if key is expired
        if self.is_key_expired(&api_key_data) {
            warn!("API key expired: {}", &key[..8]);
            return None;
        }

        // Check rate limiting
        if self.is_rate_limited(key) {
            warn!("Rate limit exceeded for API key: {}", &key[..8]);
            return None;
        }

        // Update last used timestamp in database
        if let Err(e) = self.storage.update_usage(key) {
            warn!("Failed to update API key usage: {}", e);
        }

        // Return updated key with current timestamp
        let mut updated_key = api_key_data;
        updated_key.last_used = Some(Utc::now());
        updated_key.usage_count += 1;

        Some(updated_key)
    }

    pub fn is_key_expired(&self, api_key: &ApiKey) -> bool {
        Utc::now() > api_key.expires_at
    }

    pub fn check_endpoint_permission(&self, api_key: &ApiKey, path: &str) -> bool {
        // Define permission mappings for different endpoints
        let required_permission = match path {
            p if p.starts_with("/api/v1/neuromorphic") => "neuromorphic",
            p if p.starts_with("/api/v1/quantum") => "quantum",
            p if p.starts_with("/api/v1/dna") => "dna",
            p if p.starts_with("/api/v1/admin") => "admin",
            p if p.starts_with("/metrics") => "admin",
            p if p.contains("/query") || p.contains("/search") => "read",
            p if p.contains("/train") || p.contains("/optimize") || p.contains("/compress") => {
                "write"
            }
            _ => "read", // Default to read permission
        };

        api_key
            .permissions
            .contains(&required_permission.to_string())
            || api_key.permissions.contains(&"admin".to_string())
    }

    fn is_rate_limited(&self, key: &str) -> bool {
        // Get API key from storage to check rate limit
        if let Ok(Some((api_key, _))) = self.storage.get_key(key) {
            if let Some(rate_limit) = api_key.rate_limit_per_hour {
                if let Some(usage_times) = self.usage_tracking.get(key) {
                    let one_hour_ago = Utc::now() - chrono::Duration::hours(1);
                    let recent_usage = usage_times
                        .iter()
                        .filter(|&&time| time > one_hour_ago)
                        .count();

                    return recent_usage >= rate_limit as usize;
                }
            }
        }
        false
    }

    pub fn revoke_api_key(&mut self, key: &str, revoked_by: Option<&str>) -> bool {
        info!("🗑️ Revoking API key: {}", &key[..8.min(key.len())]);

        match self.storage.revoke_key(key, revoked_by) {
            Ok(revoked) => {
                if revoked {
                    self.usage_tracking.remove(key);
                }
                revoked
            }
            Err(e) => {
                warn!("Failed to revoke API key: {}", e);
                false
            }
        }
    }

    pub fn list_api_keys(&self) -> Vec<ApiKeyInfo> {
        self.storage.list_keys().unwrap_or_default()
    }

    pub fn get_storage_stats(&self) -> StorageStats {
        self.storage.get_stats().unwrap_or(StorageStats {
            total_active_keys: 0,
            total_revoked_keys: 0,
            admin_keys: 0,
        })
    }

    /// Check if API key generation is rate limited for a given IP address
    /// Default: Max 5 key generations per hour per IP
    pub fn check_key_generation_rate_limit(&self, ip_address: &str) -> Result<(), String> {
        const MAX_GENERATIONS_PER_HOUR: usize = 5;

        if let Some(generation_times) = self.key_generation_tracking.get(ip_address) {
            let one_hour_ago = Utc::now() - chrono::Duration::hours(1);
            let recent_generations = generation_times
                .iter()
                .filter(|&&time| time > one_hour_ago)
                .count();

            if recent_generations >= MAX_GENERATIONS_PER_HOUR {
                warn!(
                    "⚠️ API key generation rate limit exceeded for IP: {} ({}/{} in last hour)",
                    ip_address, recent_generations, MAX_GENERATIONS_PER_HOUR
                );
                return Err(format!(
                    "Rate limit exceeded: Maximum {} key generations per hour. Try again later.",
                    MAX_GENERATIONS_PER_HOUR
                ));
            }
        }
        Ok(())
    }

    /// Track API key generation attempt from an IP address
    pub fn track_key_generation(&mut self, ip_address: &str) {
        let entry = self
            .key_generation_tracking
            .entry(ip_address.to_string())
            .or_default();
        entry.push(Utc::now());

        // Clean up old entries (older than 24 hours) to prevent memory growth
        let cutoff = Utc::now() - chrono::Duration::hours(24);
        entry.retain(|&time| time > cutoff);
    }

    pub fn get_api_key_stats(&self, key: &str) -> Option<ApiKeyStats> {
        if let Ok(Some((api_key, _hash))) = self.storage.get_key(key) {
            let usage_times = self.usage_tracking.get(key)?;
            let one_hour_ago = Utc::now() - chrono::Duration::hours(1);
            let recent_usage = usage_times
                .iter()
                .filter(|&&time| time > one_hour_ago)
                .count();

            Some(ApiKeyStats {
                total_usage: api_key.usage_count,
                recent_usage: recent_usage as u64,
                last_used: api_key.last_used,
                expires_at: api_key.expires_at,
                is_expired: self.is_key_expired(&api_key),
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyStats {
    pub total_usage: u64,
    pub recent_usage: u64,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub is_expired: bool,
}

impl Default for AuthService {
    fn default() -> Self {
        Self::new().expect("Failed to initialize AuthService with default storage")
    }
}
