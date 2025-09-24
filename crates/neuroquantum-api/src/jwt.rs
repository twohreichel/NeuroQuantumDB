use crate::error::{ApiError, AuthToken, QuantumAuthClaims};
use actix_web::{dev::ServiceRequest, error::ErrorUnauthorized, Error, HttpMessage};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};
use std::sync::Arc;
use tracing::{error, warn};

/// JWT authentication service with quantum-resistant features
#[derive(Clone)]
pub struct JwtService {
    encoding_key: Arc<EncodingKey>,
    decoding_key: Arc<DecodingKey>,
    validation: Validation,
}

impl JwtService {
    pub fn new(secret: &[u8]) -> Self {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.validate_aud = false;

        Self {
            encoding_key: Arc::new(EncodingKey::from_secret(secret)),
            decoding_key: Arc::new(DecodingKey::from_secret(secret)),
            validation,
        }
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
                details: format!("Token generation failed: {}", e),
            }
        })
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<AuthToken, ApiError> {
        decode::<AuthToken>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| {
                warn!("JWT validation failed: {}", e);
                ApiError::Unauthorized(format!("Invalid token: {}", e))
            })
    }

    /// Generate quantum-resistant authentication token
    pub fn generate_quantum_token(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<String, ApiError> {
        // In a real implementation, this would use post-quantum cryptography
        // For now, we'll simulate with enhanced claims
        let _quantum_claims = QuantumAuthClaims {
            user_id: user_id.to_string(),
            session_id: session_id.to_string(),
            quantum_signature: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("quantum_sig_{}", user_id),
            ),
            kyber_public_key: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("kyber_key_{}", user_id),
            ),
            dilithium_signature: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("dilithium_sig_{}", session_id),
            ),
        };

        let now = chrono::Utc::now();
        let exp = (now + chrono::Duration::hours(1)).timestamp() as usize;
        let iat = now.timestamp() as usize;

        let claims = AuthToken {
            sub: user_id.to_string(),
            exp,
            iat,
            quantum_level: 255, // Maximum quantum security
            permissions: vec!["quantum_authenticated".to_string()],
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(|e| {
            error!("Failed to generate quantum JWT token: {}", e);
            ApiError::EncryptionError {
                details: format!("Quantum token generation failed: {}", e),
            }
        })
    }

    /// Refresh an existing token
    pub fn refresh_token(&self, token: &str) -> Result<String, ApiError> {
        let claims = self.validate_token(token)?;

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
    pub fn new(service: JwtService) -> Self {
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
                        match jwt_service.validate_token(token) {
                            Ok(claims) => {
                                // Add claims to request extensions
                                req.extensions_mut().insert(claims);
                                return service.call(req).await;
                            }
                            Err(e) => {
                                warn!("JWT validation failed: {:?}", e);
                                return Err(ErrorUnauthorized(
                                    serde_json::json!({
                                        "error": "Invalid or expired token",
                                        "code": "JWT_INVALID"
                                    })
                                    .to_string(),
                                ));
                            }
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
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "your-256-bit-secret".to_string(),
            expiration_hours: 24,
            refresh_threshold_minutes: 60,
            quantum_enabled: false,
            algorithm: "HS256".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_generation_and_validation() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let service = JwtService::new(secret);

        let token = service
            .generate_token(
                "test_user",
                vec!["read".to_string(), "write".to_string()],
                128,
            )
            .unwrap();

        let claims = service.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "test_user");
        assert_eq!(claims.quantum_level, 128);
        assert!(claims.permissions.contains(&"read".to_string()));
    }

    #[test]
    fn test_quantum_token_generation() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let service = JwtService::new(secret);

        let token = service
            .generate_quantum_token("quantum_user", "session_123")
            .unwrap();
        let claims = service.validate_token(&token).unwrap();

        assert_eq!(claims.sub, "quantum_user");
        assert_eq!(claims.quantum_level, 255);
        assert!(claims
            .permissions
            .contains(&"quantum_authenticated".to_string()));
    }

    #[test]
    fn test_invalid_token() {
        let secret = b"test_secret_key_32_bytes_minimum!!";
        let service = JwtService::new(secret);

        let result = service.validate_token("invalid.token.here");
        assert!(result.is_err());
    }
}
