use crate::auth::{ApiKey, AuthService};
use crate::error::{ApiError, AuthToken};
use crate::jwt::JwtService;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
    http::header::{HeaderName, HeaderValue},
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tracing::{debug, info, warn};

/// Authentication middleware that supports both JWT and API key authentication
pub struct AuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            // Skip authentication for certain public endpoints
            let path = req.path();
            if is_public_endpoint(path) {
                return service.call(req).await;
            }

            // Try JWT authentication first
            if let Some(auth_header) = req.headers().get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    if let Some(token) = auth_str.strip_prefix("Bearer ") {
                        // Extract JWT service from app data
                        if let Some(jwt_service) = req.app_data::<actix_web::web::Data<JwtService>>() {
                            match jwt_service.validate_token(token) {
                                Ok(claims) => {
                                    debug!("✅ JWT authentication successful for user: {}", claims.sub);
                                    req.extensions_mut().insert(claims);
                                    return service.call(req).await;
                                }
                                Err(e) => {
                                    warn!("❌ JWT validation failed: {:?}", e);
                                }
                            }
                        }
                    }
                }
            }

            // Try API key authentication
            if let Some(api_key_header) = req.headers().get("X-API-Key") {
                if let Ok(api_key_str) = api_key_header.to_str() {
                    if let Some(auth_service) = req.app_data::<actix_web::web::Data<AuthService>>() {
                        if let Some(api_key) = auth_service.validate_api_key(api_key_str).await {
                            debug!("✅ API key authentication successful for: {}", api_key.name);
                            req.extensions_mut().insert(api_key);
                            return service.call(req).await;
                        } else {
                            warn!("❌ Invalid API key provided");
                        }
                    }
                }
            }

            // No valid authentication found
            warn!("🚫 Authentication required for endpoint: {}", path);

            // Return proper error response
            let response = HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Authentication required",
                "code": "AUTH_REQUIRED",
                "message": "Please provide a valid JWT token or API key"
            }));

            Ok(req.into_response(response))
        })
    }
}

/// Transform factory for authentication middleware
pub struct AuthMiddlewareFactory;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

/// Helper function to determine if an endpoint is public
fn is_public_endpoint(path: &str) -> bool {
    matches!(path,
        "/health" |
        "/metrics" |
        "/api-docs" |
        "/api-docs/" |
        "/api/v1/auth/login" |
        "/api/v1/auth/refresh"
    ) || path.starts_with("/api-docs/")
}

/// Convenience function to create auth middleware
pub fn auth_middleware() -> AuthMiddlewareFactory {
    AuthMiddlewareFactory
}

/// Security headers middleware
pub struct SecurityHeadersMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            let mut res = service.call(req).await?;

            // Add security headers
            let headers = res.headers_mut();

            // Strict Transport Security
            headers.insert(
                HeaderName::from_static("strict-transport-security"),
                HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
            );

            // Content Security Policy
            headers.insert(
                HeaderName::from_static("content-security-policy"),
                HeaderValue::from_static(
                    "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'"
                ),
            );

            // X-Frame-Options
            headers.insert(
                HeaderName::from_static("x-frame-options"),
                HeaderValue::from_static("DENY"),
            );

            // X-Content-Type-Options
            headers.insert(
                HeaderName::from_static("x-content-type-options"),
                HeaderValue::from_static("nosniff"),
            );

            // Referrer Policy
            headers.insert(
                HeaderName::from_static("referrer-policy"),
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            );

            // Permissions Policy
            headers.insert(
                HeaderName::from_static("permissions-policy"),
                HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
            );

            Ok(res)
        })
    }
}

pub struct SecurityHeadersMiddlewareFactory;

impl<S, B> Transform<S, ServiceRequest> for SecurityHeadersMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = SecurityHeadersMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityHeadersMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub fn security_headers_middleware() -> SecurityHeadersMiddlewareFactory {
    SecurityHeadersMiddlewareFactory
}

/// Request validation middleware
pub struct RequestValidationMiddleware<S> {
    service: Rc<S>,
    max_payload_size: usize,
}

impl<S, B> Service<ServiceRequest> for RequestValidationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let max_payload_size = self.max_payload_size;

        Box::pin(async move {
            // Check payload size
            if let Some(content_length) = req.headers().get("content-length") {
                if let Ok(length_str) = content_length.to_str() {
                    if let Ok(length) = length_str.parse::<usize>() {
                        if length > max_payload_size {
                            warn!("🚫 Request payload too large: {} bytes (max: {})", length, max_payload_size);
                            return Ok(req.error_response(
                                HttpResponse::PayloadTooLarge().json(serde_json::json!({
                                    "error": "Payload too large",
                                    "code": "PAYLOAD_TOO_LARGE",
                                    "max_size_bytes": max_payload_size,
                                    "provided_size_bytes": length
                                }))
                            ));
                        }
                    }
                }
            }

            // Validate content type for POST/PUT requests
            let method = req.method();
            if method == "POST" || method == "PUT" || method == "PATCH" {
                if let Some(content_type) = req.headers().get("content-type") {
                    if let Ok(content_type_str) = content_type.to_str() {
                        if !content_type_str.starts_with("application/json") {
                            warn!("🚫 Invalid content type: {}", content_type_str);
                            return Ok(req.error_response(
                                HttpResponse::UnsupportedMediaType().json(serde_json::json!({
                                    "error": "Unsupported media type",
                                    "code": "UNSUPPORTED_MEDIA_TYPE",
                                    "expected": "application/json",
                                    "provided": content_type_str
                                }))
                            ));
                        }
                    }
                } else {
                    warn!("🚫 Missing content type header for {} request", method);
                    return Ok(req.error_response(
                        HttpResponse::BadRequest().json(serde_json::json!({
                            "error": "Content-Type header required",
                            "code": "MISSING_CONTENT_TYPE"
                        }))
                    ));
                }
            }

            service.call(req).await
        })
    }
}

pub struct RequestValidationMiddlewareFactory {
    max_payload_size: usize,
}

impl RequestValidationMiddlewareFactory {
    pub fn new(max_payload_size: usize) -> Self {
        Self { max_payload_size }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequestValidationMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestValidationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestValidationMiddleware {
            service: Rc::new(service),
            max_payload_size: self.max_payload_size,
        }))
    }
}

pub fn request_validation_middleware(max_payload_size: usize) -> RequestValidationMiddlewareFactory {
    RequestValidationMiddlewareFactory::new(max_payload_size)
}

/// Circuit breaker middleware for external service calls
#[derive(Clone)]
pub struct CircuitBreaker {
    failure_threshold: u64,
    success_threshold: u64,
    timeout: Duration,
    failure_count: Rc<AtomicU64>,
    last_failure_time: Rc<std::sync::Mutex<Option<Instant>>>,
    state: Rc<std::sync::Mutex<CircuitBreakerState>>,
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u64, success_threshold: u64, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
            failure_count: Rc::new(AtomicU64::new(0)),
            last_failure_time: Rc::new(std::sync::Mutex::new(None)),
            state: Rc::new(std::sync::Mutex::new(CircuitBreakerState::Closed)),
        }
    }

    pub fn call_service<F, R>(&self, service_name: &str, f: F) -> Result<R, ApiError>
    where
        F: FnOnce() -> Result<R, ApiError>,
    {
        let current_state = {
            let state = self.state.lock().unwrap();
            state.clone()
        };

        match current_state {
            CircuitBreakerState::Open => {
                // Check if we should transition to half-open
                let should_try = {
                    let last_failure = self.last_failure_time.lock().unwrap();
                    if let Some(last_time) = *last_failure {
                        last_time.elapsed() >= self.timeout
                    } else {
                        true
                    }
                };

                if should_try {
                    let mut state = self.state.lock().unwrap();
                    *state = CircuitBreakerState::HalfOpen;
                    drop(state);
                    info!("🔄 Circuit breaker transitioning to half-open for service: {}", service_name);
                } else {
                    warn!("⚡ Circuit breaker is open for service: {}", service_name);
                    return Err(ApiError::CircuitBreakerOpen {
                        service: service_name.to_string(),
                    });
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Allow limited requests through
                info!("🟡 Circuit breaker half-open, allowing request for service: {}", service_name);
            }
            CircuitBreakerState::Closed => {
                // Normal operation
                debug!("🟢 Circuit breaker closed, allowing request for service: {}", service_name);
            }
        }

        // Execute the service call
        match f() {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(error) => {
                self.on_failure();
                Err(error)
            }
        }
    }

    fn on_success(&self) {
        let current_state = {
            let state = self.state.lock().unwrap();
            state.clone()
        };

        if current_state == CircuitBreakerState::HalfOpen {
            // Reset failure count and close the circuit
            self.failure_count.store(0, Ordering::SeqCst);
            let mut state = self.state.lock().unwrap();
            *state = CircuitBreakerState::Closed;
            info!("✅ Circuit breaker closed after successful recovery");
        }
    }

    fn on_failure(&self) {
        let failure_count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;

        {
            let mut last_failure = self.last_failure_time.lock().unwrap();
            *last_failure = Some(Instant::now());
        }

        if failure_count >= self.failure_threshold {
            let mut state = self.state.lock().unwrap();
            *state = CircuitBreakerState::Open;
            warn!("🔴 Circuit breaker opened due to {} failures", failure_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_public_endpoint() {
        assert!(is_public_endpoint("/health"));
        assert!(is_public_endpoint("/metrics"));
        assert!(is_public_endpoint("/api/v1/auth/login"));
        assert!(is_public_endpoint("/api-docs/"));
        assert!(!is_public_endpoint("/api/v1/tables"));
        assert!(!is_public_endpoint("/api/v1/neural/train"));
    }

    #[test]
    fn test_circuit_breaker_states() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(30));

        // Test successful calls
        let result = cb.call_service("test", || Ok("success"));
        assert!(result.is_ok());

        // Test failure threshold
        for _ in 0..3 {
            let _ = cb.call_service("test", || {
                Err(ApiError::ServiceUnavailable {
                    service: "test".to_string(),
                    reason: "test failure".to_string(),
                })
            });
        }

        // Circuit should be open now
        let result = cb.call_service("test", || Ok("should fail"));
        assert!(matches!(result, Err(ApiError::CircuitBreakerOpen { .. })));
    }
}
