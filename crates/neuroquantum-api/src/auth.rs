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
    // In production, this would be stored in a secure database
    api_keys: HashMap<String, ApiKey>,
    // Store hashed keys for security
    key_hashes: HashMap<String, String>,
    // Rate limiting tracking
    usage_tracking: HashMap<String, Vec<DateTime<Utc>>>,
    // Track API key generation attempts per IP
    key_generation_tracking: HashMap<String, Vec<DateTime<Utc>>>,
}

impl AuthService {
    /// Create a new AuthService without any default keys
    /// Use `new_with_setup_mode()` for initial setup or restore from persistent storage
    pub fn new() -> Self {
        Self {
            api_keys: HashMap::new(),
            key_hashes: HashMap::new(),
            usage_tracking: HashMap::new(),
            key_generation_tracking: HashMap::new(),
        }
    }

    /// Create a new AuthService in setup mode (allows initial admin key creation)
    /// This should only be used during the initial setup phase
    pub fn new_with_setup_mode() -> Self {
        let service = Self::new();
        info!("🔧 AuthService initialized in setup mode - no default keys created");
        info!("💡 Run 'neuroquantum-api init' to create your first admin key");
        service
    }

    /// Check if any admin keys exist
    pub fn has_admin_keys(&self) -> bool {
        self.api_keys
            .values()
            .any(|key| key.permissions.contains(&"admin".to_string()))
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
            vec![
                "admin".to_string(),
                "neuromorphic".to_string(),
                "quantum".to_string(),
                "dna".to_string(),
                "read".to_string(),
                "write".to_string(),
            ],
            expiry_hours,
            Some(10000), // High rate limit for admin
        );

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
    ) -> ApiKey {
        let key = format!("nqdb_{}", Uuid::new_v4().to_string().replace('-', ""));

        // Use lower cost for tests to speed up execution
        #[cfg(test)]
        let cost = TEST_BCRYPT_COST;
        #[cfg(not(test))]
        let cost = DEFAULT_COST;

        let key_hash = match hash(&key, cost) {
            Ok(hash) => hash,
            Err(e) => {
                warn!("Failed to hash API key: {}", e);
                return ApiKey {
                    key: "error".to_string(),
                    name,
                    permissions,
                    expires_at: Utc::now(),
                    created_at: Utc::now(),
                    last_used: None,
                    usage_count: 0,
                    rate_limit_per_hour,
                };
            }
        };

        let expires_at = match expiry_hours {
            Some(hours) => Utc::now() + chrono::Duration::hours(hours as i64),
            None => Utc::now() + chrono::Duration::days(30), // Default 30 days
        };

        let api_key = ApiKey {
            key: key.clone(),
            name,
            permissions,
            expires_at,
            created_at: Utc::now(),
            last_used: None,
            usage_count: 0,
            rate_limit_per_hour,
        };

        self.key_hashes.insert(key.clone(), key_hash);
        self.api_keys.insert(key.clone(), api_key.clone());
        self.usage_tracking.insert(key.clone(), Vec::new());

        info!(
            "🔑 Generated new API key: {} for {}",
            &key[..12],
            api_key.name
        );
        api_key
    }

    pub async fn validate_api_key(&self, key: &str) -> Option<ApiKey> {
        // In production, this would query a database
        if let Some(api_key_data) = self.api_keys.get(key) {
            // Verify the key hash for additional security
            if let Some(stored_hash) = self.key_hashes.get(key) {
                if verify(key, stored_hash).unwrap_or(false) {
                    // Check rate limiting
                    if self.is_rate_limited(key) {
                        warn!("Rate limit exceeded for API key: {}", &key[..8]);
                        return None;
                    }

                    // Update last used timestamp (in production, this would update the database)
                    let mut updated_key = api_key_data.clone();
                    updated_key.last_used = Some(Utc::now());
                    updated_key.usage_count += 1;

                    return Some(updated_key);
                }
            }
        }
        None
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
        if let Some(api_key) = self.api_keys.get(key) {
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

    pub fn revoke_api_key(&mut self, key: &str) -> bool {
        info!("🗑️ Revoking API key: {}", &key[..8]);
        let removed_key = self.api_keys.remove(key).is_some();
        self.key_hashes.remove(key);
        self.usage_tracking.remove(key);
        removed_key
    }

    pub fn list_api_keys(&self) -> Vec<ApiKey> {
        self.api_keys.values().cloned().collect()
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
        if let Some(api_key) = self.api_keys.get(key) {
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
                is_expired: self.is_key_expired(api_key),
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
        Self::new()
    }
}
