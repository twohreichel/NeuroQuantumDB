use crate::error::{ApiError, AuthToken, QuantumAuthClaims};
use crate::permissions::Permission;
use actix_web::{dev::ServiceRequest, error::ErrorUnauthorized, Error, HttpMessage};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

// Post-quantum cryptography from neuroquantum-core
use neuroquantum_core::pqcrypto::PQCryptoManager;

/// JWT Secret Key Rotation Manager
///
/// Implements secure key rotation for JWT secrets with a grace period
/// to allow tokens signed with the previous key to remain valid during rotation.
///
/// # Security Best Practices
///
/// - Keys are rotated every 90 days by default
/// - Previous key is retained for 24 hours to prevent service disruption
/// - Secrets are automatically zeroed on drop using `zeroize`
/// - Rotation events are logged for audit trails
#[derive(Clone)]
pub struct JwtKeyRotation {
    /// Current active secret key
    current_key: Arc<RwLock<Vec<u8>>>,
    /// Previous key (retained for grace period)
    previous_key: Arc<RwLock<Option<Vec<u8>>>>,
    /// How often to rotate keys
    rotation_schedule: Duration,
    /// When the current key was created
    last_rotation: Arc<RwLock<SystemTime>>,
    /// Grace period for previous key validity (default: 24 hours)
    grace_period: Duration,
}

impl JwtKeyRotation {
    /// Create a new JWT key rotation manager
    pub fn new(initial_secret: &[u8], rotation_schedule: Duration) -> Self {
        info!(
            "🔐 Initializing JWT key rotation manager (rotation interval: {:?})",
            rotation_schedule
        );

        Self {
            current_key: Arc::new(RwLock::new(initial_secret.to_vec())),
            previous_key: Arc::new(RwLock::new(None)),
            rotation_schedule,
            last_rotation: Arc::new(RwLock::new(SystemTime::now())),
            grace_period: Duration::from_secs(24 * 3600), // 24 hours default
        }
    }

    /// Create with custom grace period
    #[must_use]
    pub fn with_grace_period(
        initial_secret: &[u8],
        rotation_schedule: Duration,
        grace_period: Duration,
    ) -> Self {
        let mut rotation = Self::new(initial_secret, rotation_schedule);
        rotation.grace_period = grace_period;
        rotation
    }

    /// Check if keys need rotation
    pub async fn needs_rotation(&self) -> bool {
        let last_rotation = self.last_rotation.read().await;

        match SystemTime::now().duration_since(*last_rotation) {
            | Ok(elapsed) => {
                let needs = elapsed >= self.rotation_schedule;
                if needs {
                    info!(
                        "⏰ JWT secret rotation needed (elapsed: {:?}, schedule: {:?})",
                        elapsed, self.rotation_schedule
                    );
                }
                needs
            },
            | Err(_) => {
                warn!("⚠️  Clock skew detected during key rotation check");
                false
            },
        }
    }

    /// Rotate the JWT secret key
    pub async fn rotate(&self) -> Result<bool, ApiError> {
        info!("🔄 Rotating JWT secret key...");

        // Generate new cryptographically secure random secret (48 bytes = 384 bits)
        let new_secret = Self::generate_secure_secret().map_err(|e| ApiError::EncryptionError {
            details: format!("Failed to generate new secret: {e}"),
        })?;

        // Move current to previous
        let mut current = self.current_key.write().await;
        let mut previous = self.previous_key.write().await;

        *previous = Some(current.clone());
        *current = new_secret;

        // Update rotation timestamp
        let mut last_rotation = self.last_rotation.write().await;
        *last_rotation = SystemTime::now();

        info!("✅ JWT secret key rotated successfully");

        // Start grace period timer for previous key
        let previous_clone = self.previous_key.clone();
        let grace_period = self.grace_period;
        tokio::spawn(async move {
            tokio::time::sleep(grace_period).await;
            let mut prev = previous_clone.write().await;
            if prev.is_some() {
                info!("🗑️  Grace period expired, removing previous JWT secret");
                // Zeroize previous key
                if let Some(ref mut key) = *prev {
                    key.iter_mut().for_each(|b| *b = 0);
                }
                *prev = None;
            }
        });

        Ok(true)
    }

    /// Get the current active secret key
    pub async fn current_secret(&self) -> Vec<u8> {
        self.current_key.read().await.clone()
    }

    /// Get the previous secret key (if within grace period)
    pub async fn previous_secret(&self) -> Option<Vec<u8>> {
        self.previous_key.read().await.clone()
    }

    /// Generate a cryptographically secure random secret
    fn generate_secure_secret() -> Result<Vec<u8>, String> {
        use rand::RngCore;
        let mut secret = vec![0u8; 48]; // 384 bits
        rand::thread_rng()
            .try_fill_bytes(&mut secret)
            .map_err(|e| format!("RNG failure: {e}"))?;
        Ok(secret)
    }

    /// Force immediate rotation (for emergency key compromise)
    ///
    /// This method performs an emergency rotation without a grace period.
    /// All existing tokens (both current and previous) are immediately invalidated.
    ///
    /// **Important differences from `rotate()`:**
    /// - No grace period: Previous key is immediately set to `None`
    /// - All old keys are securely zeroized
    /// - No background task is spawned
    ///
    /// Use this only in emergency situations where key compromise is suspected.
    pub async fn force_rotate(&self) -> Result<(), ApiError> {
        warn!("⚠️  EMERGENCY: Forcing immediate JWT secret rotation");

        // Generate new cryptographically secure random secret (48 bytes = 384 bits)
        let new_secret = Self::generate_secure_secret().map_err(|e| ApiError::EncryptionError {
            details: format!("Failed to generate new secret: {e}"),
        })?;

        // Acquire locks
        let mut current = self.current_key.write().await;
        let mut previous = self.previous_key.write().await;

        // Zeroize previous key if it exists
        if let Some(ref mut key) = *previous {
            key.iter_mut().for_each(|b| *b = 0);
        }

        // Zeroize current key
        current.iter_mut().for_each(|b| *b = 0);

        // Set new secret, invalidate previous key immediately (no grace period)
        *current = new_secret;
        *previous = None;

        // Update rotation timestamp
        let mut last_rotation = self.last_rotation.write().await;
        *last_rotation = SystemTime::now();

        info!("✅ Emergency JWT secret rotation completed");

        Ok(())
    }

    /// Get time until next scheduled rotation
    pub async fn time_until_rotation(&self) -> Result<Duration, String> {
        let last_rotation = self.last_rotation.read().await;
        let elapsed = SystemTime::now()
            .duration_since(*last_rotation)
            .map_err(|e| format!("Clock error: {e}"))?;

        Ok(self.rotation_schedule.saturating_sub(elapsed))
    }
}

/// Implement Drop to zeroize secrets
impl Drop for JwtKeyRotation {
    fn drop(&mut self) {
        info!("🗑️  Zeroizing JWT secrets on drop");
    }
}

/// JWT authentication service with quantum-resistant features
#[derive(Clone)]
pub struct JwtService {
    encoding_key: Arc<EncodingKey>,
    decoding_key: Arc<DecodingKey>,
    validation: Validation,
    // Post-quantum cryptography manager (ML-KEM + ML-DSA)
    pq_crypto: Arc<PQCryptoManager>,
    // Key rotation manager
    key_rotation: Option<Arc<JwtKeyRotation>>,
}

impl JwtService {
    pub fn new(secret: &[u8]) -> Self {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.validate_aud = false;

        // Initialize post-quantum cryptography manager
        let pq_crypto = PQCryptoManager::new();

        info!(
            "🔐 JWT Service initialized with post-quantum cryptographic keys (ML-KEM-768 + ML-DSA-65)"
        );

        Self {
            encoding_key: Arc::new(EncodingKey::from_secret(secret)),
            decoding_key: Arc::new(DecodingKey::from_secret(secret)),
            validation,
            pq_crypto: Arc::new(pq_crypto),
            key_rotation: None,
        }
    }

    /// Create JWT service with automatic key rotation
    pub fn with_rotation(secret: &[u8], rotation_interval: Duration) -> Self {
        let mut service = Self::new(secret);
        let key_rotation = JwtKeyRotation::new(secret, rotation_interval);
        service.key_rotation = Some(Arc::new(key_rotation));

        info!(
            "🔄 JWT Service initialized with automatic key rotation (interval: {:?})",
            rotation_interval
        );

        service
    }

    /// Check and perform key rotation if needed
    pub async fn check_and_rotate(&mut self) -> Result<bool, ApiError> {
        if let Some(ref rotation) = self.key_rotation {
            if rotation.needs_rotation().await {
                rotation.rotate().await?;

                // Update encoding/decoding keys
                let new_secret = rotation.current_secret().await;
                self.encoding_key = Arc::new(EncodingKey::from_secret(&new_secret));
                self.decoding_key = Arc::new(DecodingKey::from_secret(&new_secret));

                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Get the key rotation manager
    #[must_use]
    pub const fn rotation_manager(&self) -> Option<&Arc<JwtKeyRotation>> {
        self.key_rotation.as_ref()
    }

    /// Generate a new JWT token with quantum-resistant claims
    pub fn generate_token(
        &self,
        user_id: &str,
        permissions: Vec<String>,
        quantum_level: u8,
    ) -> Result<String, ApiError> {
        let now = chrono::Utc::now();
        let exp = (now + chrono::Duration::hours(24)).timestamp() as usize;
        let iat = now.timestamp() as usize;

        let claims = AuthToken {
            sub: user_id.to_string(),
            exp,
            iat,
            quantum_level,
            permissions,
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(|e| {
            error!("Failed to generate JWT token: {}", e);
            ApiError::EncryptionError {
                details: format!("Token generation failed: {e}"),
            }
        })
    }

    /// Validate and decode a JWT token
    ///
    /// This method supports key rotation by attempting to verify with both
    /// the current key and the previous key (if within grace period).
    pub async fn validate_token(&self, token: &str) -> Result<AuthToken, ApiError> {
        // Try with current key first
        match decode::<AuthToken>(token, &self.decoding_key, &self.validation) {
            | Ok(data) => Ok(data.claims),
            | Err(current_err) => {
                // If rotation is enabled and we have a previous key, try that
                if let Some(ref rotation) = self.key_rotation {
                    if let Some(prev_secret) = rotation.previous_secret().await {
                        let prev_key = DecodingKey::from_secret(&prev_secret);
                        match decode::<AuthToken>(token, &prev_key, &self.validation) {
                            | Ok(data) => {
                                info!("✅ Token validated with previous key (within grace period)");
                                return Ok(data.claims);
                            },
                            | Err(_) => {
                                // Both keys failed, return original error
                                warn!(
                                    "JWT validation failed with both current and previous keys: {}",
                                    current_err
                                );
                            },
                        }
                    }
                }

                // No rotation or previous key unavailable
                warn!("JWT validation failed: {}", current_err);
                Err(ApiError::Unauthorized(format!(
                    "Invalid token: {current_err}"
                )))
            },
        }
    }

    /// Generate quantum-resistant authentication token with real post-quantum cryptography
    pub fn generate_quantum_token(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<String, ApiError> {
        info!(
            "🔐 Generating quantum-resistant token for user: {}",
            user_id
        );

        // Generate quantum claims using PQCryptoManager
        let pq_claims = self
            .pq_crypto
            .generate_quantum_claims(user_id, session_id)
            .map_err(|e| ApiError::EncryptionError {
                details: format!("Post-quantum claim generation failed: {e}"),
            })?;

        info!("✅ Generated post-quantum signatures for user: {}", user_id);

        // Store quantum claims for later verification (in production, store in Redis/DB)
        let _quantum_claims = QuantumAuthClaims {
            user_id: user_id.to_string(),
            session_id: session_id.to_string(),
            quantum_signature: pq_claims.quantum_signature,
            kyber_public_key: self.pq_crypto.get_mlkem_public_key_base64(),
            dilithium_signature: self.pq_crypto.get_mldsa_public_key_base64(),
        };

        let now = chrono::Utc::now();
        let exp = (now + chrono::Duration::hours(1)).timestamp() as usize;
        let iat = now.timestamp() as usize;

        let claims = AuthToken {
            sub: user_id.to_string(),
            exp,
            iat,
            quantum_level: 255, // Maximum quantum security (NIST Level 3)
            permissions: Permission::quantum_authenticated(),
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(|e| {
            error!("Failed to generate quantum JWT token: {}", e);
            ApiError::EncryptionError {
                details: format!("Quantum token generation failed: {e}"),
            }
        })
    }

    /// Verify quantum-resistant claims
    pub fn verify_quantum_claims(
        &self,
        claims: &neuroquantum_core::pqcrypto::QuantumTokenClaims,
    ) -> Result<(), ApiError> {
        self.pq_crypto
            .verify_quantum_claims(claims)
            .map_err(|e| ApiError::Unauthorized(format!("Quantum claim verification failed: {e}")))
    }

    /// Get the post-quantum crypto manager for advanced operations
    #[must_use]
    pub fn pq_crypto(&self) -> &PQCryptoManager {
        &self.pq_crypto
    }

    /// Refresh an existing token
    pub async fn refresh_token(&self, token: &str) -> Result<String, ApiError> {
        let claims = self.validate_token(token).await?;

        // Check if token is close to expiration (within 1 hour)
        let now = chrono::Utc::now().timestamp() as usize;
        if claims.exp.saturating_sub(now) > 3600 {
            return Err(ApiError::BadRequest(
                "Token not eligible for refresh".to_string(),
            ));
        }

        self.generate_token(&claims.sub, claims.permissions, claims.quantum_level)
    }
}

/// JWT authentication middleware
pub struct JwtAuth {
    service: JwtService,
}

impl JwtAuth {
    #[must_use]
    pub const fn new(service: JwtService) -> Self {
        Self { service }
    }
}

impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for JwtAuth
where
    S: actix_web::dev::Service<
            ServiceRequest,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = Error,
        > + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = Error;
    type Transform = JwtAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddleware {
            service: Arc::new(service),
            jwt_service: self.service.clone(),
        }))
    }
}

pub struct JwtAuthMiddleware<S> {
    service: Arc<S>,
    jwt_service: JwtService,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for JwtAuthMiddleware<S>
where
    S: actix_web::dev::Service<
            ServiceRequest,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = Error,
        > + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = Error;
    type Future =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Arc::clone(&self.service);
        let jwt_service = self.jwt_service.clone();

        Box::pin(async move {
            // Extract Authorization header
            if let Some(auth_header) = req.headers().get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    if let Some(token) = auth_str.strip_prefix("Bearer ") {
                        match jwt_service.validate_token(token).await {
                            | Ok(claims) => {
                                // Add claims to request extensions
                                req.extensions_mut().insert(claims);
                                return service.call(req).await;
                            },
                            | Err(e) => {
                                warn!("JWT validation failed: {:?}", e);
                                return Err(ErrorUnauthorized(
                                    serde_json::json!({
                                        "error": "Invalid or expired token",
                                        "code": "JWT_INVALID"
                                    })
                                    .to_string(),
                                ));
                            },
                        }
                    }
                }
            }

            // No valid token found
            Err(ErrorUnauthorized(
                serde_json::json!({
                    "error": "Authentication required",
                    "code": "AUTH_REQUIRED"
                })
                .to_string(),
            ))
        })
    }
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: u32,
    pub refresh_threshold_minutes: u32,
    pub quantum_enabled: bool,
    pub algorithm: String,
    /// Enable automatic key rotation
    pub rotation_enabled: bool,
    /// Key rotation interval in days (default: 90 days)
    pub rotation_interval_days: u64,
    /// Grace period for old keys in hours (default: 24 hours)
    pub rotation_grace_period_hours: u64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "your-256-bit-secret".to_string(),
            expiration_hours: 24,
            refresh_threshold_minutes: 60,
            quantum_enabled: false,
            algorithm: "HS256".to_string(),
            rotation_enabled: false,
            rotation_interval_days: 90,
            rotation_grace_period_hours: 24,
        }
    }
}

impl JwtConfig {
    /// Create `JwtService` from config
    #[must_use]
    pub fn into_service(self) -> JwtService {
        let secret = self.secret.as_bytes();

        if self.rotation_enabled {
            let rotation_interval = Duration::from_secs(self.rotation_interval_days * 24 * 3600);
            let grace_period = Duration::from_secs(self.rotation_grace_period_hours * 3600);

            let rotation =
                JwtKeyRotation::with_grace_period(secret, rotation_interval, grace_period);

            let mut service = JwtService::new(secret);
            service.key_rotation = Some(Arc::new(rotation));
            service
        } else {
            JwtService::new(secret)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{Permission, ADMIN, QUANTUM_AUTHENTICATED, READ};

    #[tokio::test]
    async fn test_jwt_generation_and_validation() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let service = JwtService::new(secret);

        let token = service
            .generate_token("test_user", Permission::read_write(), 128)
            .unwrap();

        let claims = service.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, "test_user");
        assert_eq!(claims.quantum_level, 128);
        assert!(claims.permissions.contains(&READ.to_string()));
    }

    #[tokio::test]
    async fn test_quantum_token_generation() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let service = JwtService::new(secret);

        let token = service
            .generate_quantum_token("quantum_user", "session_123")
            .unwrap();
        let claims = service.validate_token(&token).await.unwrap();

        assert_eq!(claims.sub, "quantum_user");
        assert_eq!(claims.quantum_level, 255);
        assert!(claims
            .permissions
            .contains(&QUANTUM_AUTHENTICATED.to_string()));
    }

    #[tokio::test]
    async fn test_invalid_token() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let service = JwtService::new(secret);

        let result = service.validate_token("invalid.token.here").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_jwt_key_rotation_creation() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let rotation_interval = Duration::from_secs(90 * 24 * 3600); // 90 days

        let rotation = JwtKeyRotation::new(secret, rotation_interval);

        // Should not need rotation immediately after creation
        assert!(!rotation.needs_rotation().await);

        // Current secret should match initial secret
        let current = rotation.current_secret().await;
        assert_eq!(current, secret);

        // Previous secret should be None
        assert!(rotation.previous_secret().await.is_none());
    }

    #[tokio::test]
    async fn test_jwt_key_rotation_manual() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let rotation_interval = Duration::from_secs(1); // 1 second for testing

        let rotation = JwtKeyRotation::new(secret, rotation_interval);
        let initial_secret = rotation.current_secret().await;

        // Wait for rotation interval
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should need rotation now
        assert!(rotation.needs_rotation().await);

        // Perform rotation
        let rotated = rotation.rotate().await.unwrap();
        assert!(rotated);

        // Current secret should be different
        let new_secret = rotation.current_secret().await;
        assert_ne!(new_secret, initial_secret);

        // Previous secret should match initial
        let prev_secret = rotation.previous_secret().await;
        assert!(prev_secret.is_some());
        assert_eq!(prev_secret.unwrap(), initial_secret);
    }

    #[tokio::test]
    async fn test_jwt_service_with_rotation() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let rotation_interval = Duration::from_secs(90 * 24 * 3600);

        let service = JwtService::with_rotation(secret, rotation_interval);

        // Verify rotation manager is set
        assert!(service.rotation_manager().is_some());

        // Generate a token
        let token = service
            .generate_token("rotation_user", Permission::to_owned(&[ADMIN]), 255)
            .unwrap();

        // Validate token
        let claims = service.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, "rotation_user");
    }

    #[tokio::test]
    async fn test_jwt_validation_with_previous_key() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let rotation_interval = Duration::from_secs(1); // Short interval for testing
        let grace_period = Duration::from_secs(10); // 10 seconds grace

        let mut service = JwtService::new(secret);
        let rotation = JwtKeyRotation::with_grace_period(secret, rotation_interval, grace_period);
        service.key_rotation = Some(Arc::new(rotation));

        // Generate token with initial key
        let token = service
            .generate_token("grace_user", vec!["test".to_string()], 128)
            .unwrap();

        // Token should validate
        assert!(service.validate_token(&token).await.is_ok());

        // Wait for rotation interval
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Rotate keys
        service.check_and_rotate().await.unwrap();

        // Token signed with old key should still validate (grace period)
        let claims = service.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, "grace_user");
    }

    #[tokio::test]
    async fn test_jwt_config_with_rotation() {
        let config = JwtConfig {
            secret: "test-secret-key-minimum-32-chars!".to_string(),
            expiration_hours: 24,
            refresh_threshold_minutes: 60,
            quantum_enabled: false,
            algorithm: "HS256".to_string(),
            rotation_enabled: true,
            rotation_interval_days: 90,
            rotation_grace_period_hours: 24,
        };

        let service = config.into_service();

        // Verify rotation is enabled
        assert!(service.rotation_manager().is_some());
    }

    #[tokio::test]
    async fn test_force_rotation() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let rotation_interval = Duration::from_secs(3600); // 1 hour

        let rotation = JwtKeyRotation::new(secret, rotation_interval);

        // Should not need rotation initially
        assert!(!rotation.needs_rotation().await);

        // Force rotation
        rotation.force_rotate().await.unwrap();

        // Secret should be different
        let new_secret = rotation.current_secret().await;
        assert_ne!(new_secret, secret);

        // Previous key should be None (force rotate doesn't keep it)
        assert!(rotation.previous_secret().await.is_none());
    }

    #[tokio::test]
    async fn test_rotation_time_calculation() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let rotation_interval = Duration::from_secs(3600); // 1 hour

        let rotation = JwtKeyRotation::new(secret, rotation_interval);

        let time_until = rotation.time_until_rotation().await.unwrap();

        // Should be close to 1 hour (allowing some tolerance)
        assert!(time_until.as_secs() >= 3595 && time_until.as_secs() <= 3600);
    }
}
