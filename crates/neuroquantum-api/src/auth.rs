use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub expires_at: String,
    pub created_at: String,
}

pub struct AuthService {
    // In a real implementation, this would be stored in a database
    api_keys: HashMap<String, ApiKey>,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            api_keys: HashMap::new(),
        }
    }

    pub fn generate_api_key(&mut self, name: String, permissions: Vec<String>) -> ApiKey {
        let key = format!("nqdb_{}", Uuid::new_v4().to_string().replace('-', ""));
        
        let api_key = ApiKey {
            key: key.clone(),
            name,
            permissions,
            expires_at: "2025-09-13T10:00:00Z".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.api_keys.insert(key, api_key.clone());
        api_key
    }

    pub fn validate_api_key(&self, key: &str) -> Option<&ApiKey> {
        self.api_keys.get(key)
    }

    pub fn revoke_api_key(&mut self, key: &str) -> bool {
        self.api_keys.remove(key).is_some()
    }
}

impl Default for AuthService {
    fn default() -> Self {
        Self::new()
    }
}
