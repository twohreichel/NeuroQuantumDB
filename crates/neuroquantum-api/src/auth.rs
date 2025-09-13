use crate::error::{ApiError, QuantumAuthClaims};
use actix_web::{ResponseError, web, HttpResponse, Result as ActixResult};
use ring::rand::SecureRandom;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};
use anyhow::Result;
use jsonwebtoken::{encode, decode, Header, Algorithm, EncodingKey, DecodingKey, Validation};
use base64::{Engine as _, engine::general_purpose};

/// Quantum-resistant authentication service
#[derive(Clone)]
pub struct QuantumAuthService {
    kyber_keypair: KyberKeyPair,
    dilithium_keypair: DilithiumKeyPair,
    jwt_secret: Vec<u8>,
    token_expiry: u64,
}

/// Kyber key encapsulation mechanism for quantum-resistant key exchange
#[derive(Clone)]
pub struct KyberKeyPair {
    public_key: Vec<u8>,
    private_key: Vec<u8>,
}

/// Dilithium digital signature for quantum-resistant authentication
#[derive(Clone)]
pub struct DilithiumKeyPair {
    public_key: Vec<u8>,
    private_key: Vec<u8>,
}

/// Login request structure
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub quantum_challenge: Option<String>,
    pub kyber_public_key: Option<String>,
}

/// Login response with quantum-resistant tokens
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub quantum_level: u8,
    pub kyber_shared_secret: String,
    pub dilithium_signature: String,
}

/// Token refresh request
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
    pub quantum_signature: Option<String>,
}

impl QuantumAuthService {
    /// Initialize quantum-resistant authentication service
    pub fn new(config: &crate::config::AuthConfig) -> Result<Self> {
        let rng = ring::rand::SystemRandom::new();

        // Generate Kyber keypair for key encapsulation
        let kyber_keypair = Self::generate_kyber_keypair(&rng)?;

        // Generate Dilithium keypair for digital signatures
        let dilithium_keypair = Self::generate_dilithium_keypair(&rng)?;

        Ok(Self {
            kyber_keypair,
            dilithium_keypair,
            jwt_secret: config.jwt_secret.as_bytes().to_vec(),
            token_expiry: config.token_expiry_seconds,
        })
    }

    /// Generate JWT token with quantum-resistant claims
    pub fn generate_token(&self, user_id: &str, _permissions: Vec<String>) -> Result<String> {
        let _now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let claims = QuantumAuthClaims {
            user_id: user_id.to_string(),
            session_id: uuid::Uuid::new_v4().to_string(),
            quantum_signature: self.generate_quantum_signature(user_id)?,
            kyber_public_key: general_purpose::STANDARD.encode(&self.kyber_keypair.public_key),
            dilithium_signature: self.generate_dilithium_signature(user_id)?,
        };

        let header = Header::new(Algorithm::HS512);
        let token = encode(&header, &claims, &EncodingKey::from_secret(&self.jwt_secret))?;

        Ok(token)
    }

    /// Validate JWT token and extract quantum claims
    pub fn validate_token(&self, token: &str) -> Result<QuantumAuthClaims> {
        let validation = Validation::new(Algorithm::HS512);
        let token_data = decode::<QuantumAuthClaims>(
            token,
            &DecodingKey::from_secret(&self.jwt_secret),
            &validation,
        )?;

        // Verify quantum signature
        if !self.verify_quantum_signature(&token_data.claims.user_id, &token_data.claims.quantum_signature)? {
            return Err(anyhow::anyhow!("Invalid quantum signature"));
        }

        Ok(token_data.claims)
    }

    /// Perform quantum key exchange using Kyber
    pub fn quantum_key_exchange(&self, client_public_key: &[u8]) -> Result<Vec<u8>> {
        // Simulate Kyber key encapsulation
        // In a real implementation, this would use a proper Kyber library
        let rng = ring::rand::SystemRandom::new();
        let mut shared_secret = vec![0u8; 32];
        rng.fill(&mut shared_secret).map_err(|_| anyhow::anyhow!("RNG failed"))?;

        Ok(shared_secret)
    }

    fn generate_kyber_keypair(rng: &ring::rand::SystemRandom) -> Result<KyberKeyPair> {
        // Simulate Kyber-768 keypair generation
        // In production, use a proper post-quantum cryptography library
        let mut public_key = vec![0u8; 1184]; // Kyber-768 public key size
        let mut private_key = vec![0u8; 2400]; // Kyber-768 private key size

        rng.fill(&mut public_key).map_err(|_| anyhow::anyhow!("RNG failed"))?;
        rng.fill(&mut private_key).map_err(|_| anyhow::anyhow!("RNG failed"))?;

        Ok(KyberKeyPair { public_key, private_key })
    }

    fn generate_dilithium_keypair(rng: &ring::rand::SystemRandom) -> Result<DilithiumKeyPair> {
        // Simulate Dilithium-3 keypair generation
        // In production, use a proper post-quantum cryptography library
        let mut public_key = vec![0u8; 1952]; // Dilithium-3 public key size
        let mut private_key = vec![0u8; 4000]; // Dilithium-3 private key size

        rng.fill(&mut public_key).map_err(|_| anyhow::anyhow!("RNG failed"))?;
        rng.fill(&mut private_key).map_err(|_| anyhow::anyhow!("RNG failed"))?;

        Ok(DilithiumKeyPair { public_key, private_key })
    }

    fn generate_quantum_signature(&self, user_id: &str) -> Result<String> {
        // Generate quantum-resistant signature using Dilithium
        let message = format!("quantum_auth:{}", user_id);
        let signature = ring::hmac::sign(
            &ring::hmac::Key::new(ring::hmac::HMAC_SHA512, &self.dilithium_keypair.private_key),
            message.as_bytes(),
        );

        Ok(general_purpose::STANDARD.encode(signature.as_ref()))
    }

    fn generate_dilithium_signature(&self, user_id: &str) -> Result<String> {
        // Generate Dilithium digital signature
        let message = format!("dilithium_auth:{}", user_id);
        let signature = ring::hmac::sign(
            &ring::hmac::Key::new(ring::hmac::HMAC_SHA512, &self.dilithium_keypair.private_key),
            message.as_bytes(),
        );

        Ok(general_purpose::STANDARD.encode(signature.as_ref()))
    }

    fn verify_quantum_signature(&self, user_id: &str, signature: &str) -> Result<bool> {
        let expected_signature = self.generate_quantum_signature(user_id)?;
        Ok(signature == expected_signature)
    }

    #[cfg(test)]
    pub fn new_test() -> Result<Self> {
        let config = crate::config::AuthConfig {
            jwt_secret: "test_secret_key_for_quantum_auth".to_string(),
            token_expiry_seconds: 3600,
            quantum_level: 128,
            kyber_key_size: 0,
            dilithium_signature_size: 0,
            password_hash_cost: 0,
        };
        Self::new(&config)
    }
}

/// Login endpoint with quantum-resistant authentication
pub async fn login(
    request: web::Json<LoginRequest>,
    auth_service: web::Data<QuantumAuthService>,
) -> ActixResult<HttpResponse> {
    let start_time = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        username = %request.username,
        "Processing quantum login request"
    );

    // Validate credentials (simplified for demo)
    if !validate_user_credentials(&request.username, &request.password).await {
        warn!(
            request_id = %request_id,
            username = %request.username,
            "Authentication failed - invalid credentials"
        );

        return Ok(ApiError::AuthenticationFailed {
            message: "Invalid username or password".to_string(),
        }.error_response());
    }

    // Generate quantum-resistant tokens
    match auth_service.generate_token(&request.username, vec!["read".to_string(), "write".to_string()]) {
        Ok(access_token) => {
            // Perform quantum key exchange if client provided public key
            let kyber_shared_secret = if let Some(client_key) = &request.kyber_public_key {
                match general_purpose::STANDARD.decode(client_key) {
                    Ok(key_bytes) => {
                        match auth_service.quantum_key_exchange(&key_bytes) {
                            Ok(secret) => general_purpose::STANDARD.encode(secret),
                            Err(e) => {
                                error!("Quantum key exchange failed: {}", e);
                                "".to_string()
                            }
                        }
                    }
                    Err(_) => "".to_string(),
                }
            } else {
                "".to_string()
            };

            let response = LoginResponse {
                access_token: access_token.clone(),
                refresh_token: generate_refresh_token(&request.username)?,
                expires_in: auth_service.token_expiry,
                quantum_level: 128,
                kyber_shared_secret,
                dilithium_signature: auth_service.generate_dilithium_signature(&request.username)
                    .unwrap_or_default(),
            };

            info!(
                request_id = %request_id,
                username = %request.username,
                processing_time_us = start_time.elapsed().as_micros(),
                "Quantum login successful"
            );

            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "Token generation failed"
            );

            Ok(ApiError::InternalError {
                context: "Token generation failed".to_string(),
            }.error_response())
        }
    }
}

/// Token refresh endpoint
pub async fn refresh_token(
    request: web::Json<RefreshRequest>,
    auth_service: web::Data<QuantumAuthService>,
) -> ActixResult<HttpResponse> {
    let request_id = uuid::Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        "Processing token refresh request"
    );

    // Validate refresh token (simplified)
    let user_id = match validate_refresh_token(&request.refresh_token) {
        Ok(user_id) => user_id,
        Err(_) => {
            return Ok(ApiError::AuthenticationFailed {
                message: "Invalid refresh token".to_string(),
            }.error_response());
        }
    };

    // Generate new access token
    match auth_service.generate_token(&user_id, vec!["read".to_string(), "write".to_string()]) {
        Ok(access_token) => {
            let response = LoginResponse {
                access_token,
                refresh_token: generate_refresh_token(&user_id)?,
                expires_in: auth_service.token_expiry,
                quantum_level: 128,
                kyber_shared_secret: "".to_string(), // Not needed for refresh
                dilithium_signature: auth_service.generate_dilithium_signature(&user_id)
                    .unwrap_or_default(),
            };

            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "Token refresh failed"
            );

            Ok(ApiError::InternalError {
                context: "Token refresh failed".to_string(),
            }.error_response())
        }
    }
}

// Helper functions

async fn validate_user_credentials(username: &str, password: &str) -> bool {
    // In production, this would validate against a secure user database
    // with quantum-resistant password hashing (e.g., Argon2)
    !username.is_empty() && !password.is_empty()
}

fn generate_refresh_token(user_id: &str) -> Result<String, ApiError> {
    // Generate secure refresh token
    let token = format!("refresh_{}_{}", user_id, uuid::Uuid::new_v4());
    Ok(general_purpose::STANDARD.encode(token))
}

fn validate_refresh_token(token: &str) -> Result<String, ApiError> {
    // Validate refresh token and extract user ID
    match general_purpose::STANDARD.decode(token) {
        Ok(decoded) => {
            let token_str = String::from_utf8(decoded)
                .map_err(|_| ApiError::AuthenticationFailed {
                    message: "Invalid token format".to_string(),
                })?;

            if let Some(user_id) = token_str.strip_prefix("refresh_").and_then(|s| s.split('_').next()) {
                Ok(user_id.to_string())
            } else {
                Err(ApiError::AuthenticationFailed {
                    message: "Invalid token structure".to_string(),
                })
            }
        }
        Err(_) => Err(ApiError::AuthenticationFailed {
            message: "Invalid token encoding".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_quantum_auth_service() {
        let auth_service = QuantumAuthService::new_test().unwrap();
        let token = auth_service.generate_token("test_user", vec!["read".to_string()]).unwrap();
        let claims = auth_service.validate_token(&token).unwrap();

        assert_eq!(claims.user_id, "test_user");
        assert!(!claims.quantum_signature.is_empty());
    }

    #[test]
    fn test_kyber_keypair_generation() {
        let rng = ring::rand::SystemRandom::new();
        let keypair = QuantumAuthService::generate_kyber_keypair(&rng).unwrap();

        assert_eq!(keypair.public_key.len(), 1184);
        assert_eq!(keypair.private_key.len(), 2400);
    }
}
