use crate::error::ApiError;
use actix_web::{dev::ServiceRequest, Error, HttpMessage};
use redis::{AsyncCommands, Client as RedisClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_window: u32,
    pub window_size_seconds: u32,
    pub burst_allowance: Option<u32>,
    pub redis_url: Option<String>,
    pub fallback_to_memory: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_window: 100,
            window_size_seconds: 3600, // 1 hour
            burst_allowance: Some(10),
            redis_url: None,
            fallback_to_memory: true,
        }
    }
}

/// Rate limit bucket for token bucket algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RateLimitBucket {
    tokens: u32,
    last_refill: u64,
    window_start: u64,
    request_count: u32,
}

impl RateLimitBucket {
    fn new(max_tokens: u32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            tokens: max_tokens,
            last_refill: now,
            window_start: now,
            request_count: 0,
        }
    }

    fn try_consume(&mut self, config: &RateLimitConfig) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Reset window if needed
        if now >= self.window_start + config.window_size_seconds as u64 {
            self.window_start = now;
            self.request_count = 0;
            self.tokens = config.requests_per_window;
        }

        // Refill tokens based on time elapsed
        let time_elapsed = now - self.last_refill;
        if time_elapsed > 0 {
            let tokens_to_add = (time_elapsed * config.requests_per_window as u64 
                / config.window_size_seconds as u64) as u32;
            
            self.tokens = (self.tokens + tokens_to_add).min(config.requests_per_window);
            self.last_refill = now;
        }

        // Check if we can consume a token
        if self.tokens > 0 && self.request_count < config.requests_per_window {
            self.tokens -= 1;
            self.request_count += 1;
            true
        } else {
            false
        }
    }

    fn remaining_tokens(&self) -> u32 {
        self.tokens
    }

    fn time_until_reset(&self, config: &RateLimitConfig) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        (self.window_start + config.window_size_seconds as u64).saturating_sub(now)
    }
}

/// Rate limiting service supporting both Redis and in-memory storage
#[derive(Clone)]
pub struct RateLimitService {
    config: RateLimitConfig,
    redis_client: Option<RedisClient>,
    memory_store: Arc<RwLock<HashMap<String, RateLimitBucket>>>,
}

impl RateLimitService {
    pub async fn new(config: RateLimitConfig) -> Result<Self, ApiError> {
        let redis_client = if let Some(redis_url) = &config.redis_url {
            match RedisClient::open(redis_url.as_str()) {
                Ok(client) => {
                    // Test connection
                    match client.get_async_connection().await {
                        Ok(mut conn) => {
                            let _: Result<String, _> = conn.ping().await;
                            Some(client)
                        }
                        Err(e) => {
                            warn!("Failed to connect to Redis: {}. Falling back to memory store.", e);
                            if !config.fallback_to_memory {
                                return Err(ApiError::ConnectionPoolError {
                                    details: format!("Redis connection failed: {}", e),
                                });
                            }
                            None
                        }
                    }
                }
                Err(e) => {
                    warn!("Invalid Redis URL: {}. Falling back to memory store.", e);
                    if !config.fallback_to_memory {
                        return Err(ApiError::ConnectionPoolError {
                            details: format!("Invalid Redis URL: {}", e),
                        });
                    }
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            config,
            redis_client,
            memory_store: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Check rate limit for a given key (e.g., user ID, IP address)
    pub async fn check_rate_limit(&self, key: &str) -> Result<RateLimitResult, ApiError> {
        if let Some(ref client) = self.redis_client {
            self.check_rate_limit_redis(client, key).await
        } else {
            self.check_rate_limit_memory(key).await
        }
    }

    async fn check_rate_limit_redis(
        &self,
        client: &RedisClient,
        key: &str,
    ) -> Result<RateLimitResult, ApiError> {
        let mut conn = client.get_async_connection().await.map_err(|e| {
            ApiError::ConnectionPoolError {
                details: format!("Redis connection failed: {}", e),
            }
        })?;

        let redis_key = format!("rate_limit:{}", key);
        let bucket_key = format!("{}:bucket", redis_key);

        // Get or create bucket
        let bucket_data: Option<String> = conn.get(&bucket_key).await.map_err(|e| {
            ApiError::InternalServerError {
                message: format!("Redis GET failed: {}", e),
            }
        })?;

        let mut bucket = if let Some(data) = bucket_data {
            serde_json::from_str(&data).unwrap_or_else(|_| {
                RateLimitBucket::new(self.config.requests_per_window)
            })
        } else {
            RateLimitBucket::new(self.config.requests_per_window)
        };

        let allowed = bucket.try_consume(&self.config);

        // Store updated bucket
        let bucket_json = serde_json::to_string(&bucket).map_err(|e| {
            ApiError::InternalServerError {
                message: format!("Bucket serialization failed: {}", e),
            }
        })?;

        conn.set_ex(&bucket_key, bucket_json, self.config.window_size_seconds as u64)
            .await
            .map_err(|e| ApiError::InternalServerError {
                message: format!("Redis SET failed: {}", e),
            })?;

        Ok(RateLimitResult {
            allowed,
            remaining: bucket.remaining_tokens(),
            reset_time: bucket.time_until_reset(&self.config),
            limit: self.config.requests_per_window,
        })
    }

    async fn check_rate_limit_memory(&self, key: &str) -> Result<RateLimitResult, ApiError> {
        let mut store = self.memory_store.write().await;
        
        let bucket = store
            .entry(key.to_string())
            .or_insert_with(|| RateLimitBucket::new(self.config.requests_per_window));

        let allowed = bucket.try_consume(&self.config);

        Ok(RateLimitResult {
            allowed,
            remaining: bucket.remaining_tokens(),
            reset_time: bucket.time_until_reset(&self.config),
            limit: self.config.requests_per_window,
        })
    }

    /// Get rate limit status without consuming a token
    pub async fn get_rate_limit_status(&self, key: &str) -> Result<RateLimitResult, ApiError> {
        if let Some(ref client) = self.redis_client {
            self.get_rate_limit_status_redis(client, key).await
        } else {
            self.get_rate_limit_status_memory(key).await
        }
    }

    async fn get_rate_limit_status_redis(
        &self,
        client: &RedisClient,
        key: &str,
    ) -> Result<RateLimitResult, ApiError> {
        let mut conn = client.get_async_connection().await.map_err(|e| {
            ApiError::ConnectionPoolError {
                details: format!("Redis connection failed: {}", e),
            }
        })?;

        let redis_key = format!("rate_limit:{}:bucket", key);
        let bucket_data: Option<String> = conn.get(&redis_key).await.map_err(|e| {
            ApiError::InternalServerError {
                message: format!("Redis GET failed: {}", e),
            }
        })?;

        let bucket = if let Some(data) = bucket_data {
            serde_json::from_str(&data).unwrap_or_else(|_| {
                RateLimitBucket::new(self.config.requests_per_window)
            })
        } else {
            RateLimitBucket::new(self.config.requests_per_window)
        };

        Ok(RateLimitResult {
            allowed: true, // Status check doesn't consume tokens
            remaining: bucket.remaining_tokens(),
            reset_time: bucket.time_until_reset(&self.config),
            limit: self.config.requests_per_window,
        })
    }

    async fn get_rate_limit_status_memory(&self, key: &str) -> Result<RateLimitResult, ApiError> {
        let store = self.memory_store.read().await;
        
        if let Some(bucket) = store.get(key) {
            Ok(RateLimitResult {
                allowed: true,
                remaining: bucket.remaining_tokens(),
                reset_time: bucket.time_until_reset(&self.config),
                limit: self.config.requests_per_window,
            })
        } else {
            Ok(RateLimitResult {
                allowed: true,
                remaining: self.config.requests_per_window,
                reset_time: self.config.window_size_seconds as u64,
                limit: self.config.requests_per_window,
            })
        }
    }

    /// Reset rate limit for a specific key (admin function)
    pub async fn reset_rate_limit(&self, key: &str) -> Result<(), ApiError> {
        if let Some(ref client) = self.redis_client {
            let mut conn = client.get_async_connection().await.map_err(|e| {
                ApiError::ConnectionPoolError {
                    details: format!("Redis connection failed: {}", e),
                }
            })?;

            let redis_key = format!("rate_limit:{}:bucket", key);
            let _: () = conn.del(&redis_key).await.map_err(|e| {
                ApiError::InternalServerError {
                    message: format!("Redis DEL failed: {}", e),
                }
            })?;
        } else {
            let mut store = self.memory_store.write().await;
            store.remove(key);
        }

        Ok(())
    }

    /// Clean up expired entries (for memory store)
    pub async fn cleanup_expired(&self) {
        if self.redis_client.is_none() {
            let mut store = self.memory_store.write().await;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            store.retain(|_, bucket| {
                bucket.window_start + self.config.window_size_seconds as u64 > now
            });

            debug!("Cleaned up expired rate limit entries. Remaining: {}", store.len());
        }
    }
}

/// Rate limit check result
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u32,
    pub reset_time: u64,
    pub limit: u32,
}

/// Rate limiting middleware
pub struct RateLimitMiddleware {
    service: RateLimitService,
    key_extractor: Box<dyn Fn(&ServiceRequest) -> String + Send + Sync>,
}

impl RateLimitMiddleware {
    pub fn new<F>(service: RateLimitService, key_extractor: F) -> Self
    where
        F: Fn(&ServiceRequest) -> String + Send + Sync + 'static,
    {
        Self {
            service,
            key_extractor: Box::new(key_extractor),
        }
    }

    /// Create middleware that uses IP address as the rate limiting key
    pub fn by_ip(service: RateLimitService) -> Self {
        Self::new(service, |req| {
            req.connection_info()
                .peer_addr()
                .unwrap_or("unknown")
                .to_string()
        })
    }

    /// Create middleware that uses user ID from JWT token as the rate limiting key
    pub fn by_user(service: RateLimitService) -> Self {
        Self::new(service, |req| {
            if let Some(auth_token) = req.extensions().get::<crate::error::AuthToken>() {
                format!("user:{}", auth_token.sub)
            } else {
                format!("ip:{}", req.connection_info().peer_addr().unwrap_or("unknown"))
            }
        })
    }

    /// Create middleware that uses API key as the rate limiting key
    pub fn by_api_key(service: RateLimitService) -> Self {
        Self::new(service, |req| {
            if let Some(api_key) = req.extensions().get::<crate::auth::ApiKey>() {
                format!("api_key:{}", api_key.key)
            } else {
                format!("ip:{}", req.connection_info().peer_addr().unwrap_or("unknown"))
            }
        })
    }
}

impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for RateLimitMiddleware
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
    type Transform = RateLimitMiddlewareService<S>;
    type InitError = ();
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(RateLimitMiddlewareService {
            service: std::rc::Rc::new(service),
            rate_limit_service: self.service.clone(),
            key_extractor: self.key_extractor.as_ref(),
        }))
    }
}

pub struct RateLimitMiddlewareService<S> {
    service: std::rc::Rc<S>,
    rate_limit_service: RateLimitService,
    key_extractor: &'static (dyn Fn(&ServiceRequest) -> String + Send + Sync),
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for RateLimitMiddlewareService<S>
where
    S: actix_web::dev::Service<
        ServiceRequest,
        Response = actix_web::dev::ServiceResponse<B>,
        Error = Error,
    >,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let rate_limit_service = self.rate_limit_service.clone();
        let key = (self.key_extractor)(&req);

        Box::pin(async move {
            match rate_limit_service.check_rate_limit(&key).await {
                Ok(result) => {
                    if result.allowed {
                        // Add rate limit headers
                        let mut response = service.call(req).await?;
                        let headers = response.headers_mut();
                        headers.insert(
                            actix_web::http::header::HeaderName::from_static("x-ratelimit-limit"),
                            actix_web::http::HeaderValue::from_str(&result.limit.to_string()).unwrap(),
                        );
                        headers.insert(
                            actix_web::http::header::HeaderName::from_static("x-ratelimit-remaining"),
                            actix_web::http::HeaderValue::from_str(&result.remaining.to_string()).unwrap(),
                        );
                        headers.insert(
                            actix_web::http::header::HeaderName::from_static("x-ratelimit-reset"),
                            actix_web::http::HeaderValue::from_str(&result.reset_time.to_string()).unwrap(),
                        );
                        Ok(response)
                    } else {
                        Ok(req.error_response(
                            actix_web::HttpResponse::TooManyRequests()
                                .insert_header(("x-ratelimit-limit", result.limit.to_string()))
                                .insert_header(("x-ratelimit-remaining", result.remaining.to_string()))
                                .insert_header(("x-ratelimit-reset", result.reset_time.to_string()))
                                .insert_header(("retry-after", result.reset_time.to_string()))
                                .json(serde_json::json!({
                                    "error": "Rate limit exceeded",
                                    "code": "RATE_LIMIT_EXCEEDED",
                                    "limit": result.limit,
                                    "remaining": result.remaining,
                                    "reset_in_seconds": result.reset_time
                                }))
                        ))
                    }
                }
                Err(e) => {
                    warn!("Rate limiting error: {:?}", e);
                    // In case of rate limiting service error, allow the request through
                    service.call(req).await
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_rate_limiting() {
        let config = RateLimitConfig {
            requests_per_window: 5,
            window_size_seconds: 60,
            burst_allowance: Some(2),
            redis_url: None,
            fallback_to_memory: true,
        };

        let service = RateLimitService::new(config).await.unwrap();

        // Test normal operation
        for i in 0..5 {
            let result = service.check_rate_limit("test_user").await.unwrap();
            assert!(result.allowed, "Request {} should be allowed", i + 1);
        }

        // Test rate limit exceeded
        let result = service.check_rate_limit("test_user").await.unwrap();
        assert!(!result.allowed, "Request should be rate limited");
    }

    #[tokio::test]
    async fn test_rate_limit_reset() {
        let config = RateLimitConfig {
            requests_per_window: 2,
            window_size_seconds: 60,
            burst_allowance: None,
            redis_url: None,
            fallback_to_memory: true,
        };

        let service = RateLimitService::new(config).await.unwrap();

        // Consume all tokens
        service.check_rate_limit("test_reset").await.unwrap();
        service.check_rate_limit("test_reset").await.unwrap();
        
        let result = service.check_rate_limit("test_reset").await.unwrap();
        assert!(!result.allowed);

        // Reset and try again
        service.reset_rate_limit("test_reset").await.unwrap();
        let result = service.check_rate_limit("test_reset").await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_different_keys() {
        let config = RateLimitConfig::default();
        let service = RateLimitService::new(config).await.unwrap();

        // Different keys should have independent limits
        let result1 = service.check_rate_limit("user1").await.unwrap();
        let result2 = service.check_rate_limit("user2").await.unwrap();
        
        assert!(result1.allowed);
        assert!(result2.allowed);
        assert_eq!(result1.remaining, result2.remaining);
    }
}
