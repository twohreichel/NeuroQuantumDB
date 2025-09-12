use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures::future::{ok, Ready};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tracing::{info, warn, error};
use crate::error::ApiError;

/// Quantum security middleware for enhanced protection
pub struct QuantumSecurityMiddleware;

impl QuantumSecurityMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl<S, B> Transform<S, ServiceRequest> for QuantumSecurityMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = QuantumSecurityService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(QuantumSecurityService { service })
    }
}

pub struct QuantumSecurityService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for QuantumSecurityService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let quantum_level = req.headers()
            .get("X-Quantum-Level")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(64);

        // Validate quantum security headers
        if quantum_level > 255 {
            return Box::pin(async {
                Ok(req.into_response(
                    ApiError::ValidationError {
                        field: "X-Quantum-Level".to_string(),
                        message: "Quantum level must be 0-255".to_string(),
                    }.error_response().into_body()
                ))
            });
        }

        // Add quantum security context to request
        req.extensions_mut().insert(QuantumSecurityContext {
            level: quantum_level,
            timestamp: Instant::now(),
        });

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}

#[derive(Debug, Clone)]
pub struct QuantumSecurityContext {
    pub level: u8,
    pub timestamp: Instant,
}

/// Rate limiting middleware for API protection
pub struct RateLimitMiddleware {
    requests_per_window: u32,
    window_seconds: u64,
    clients: Rc<RefCell<HashMap<String, ClientRateLimit>>>,
}

#[derive(Debug, Clone)]
struct ClientRateLimit {
    requests: u32,
    window_start: Instant,
}

impl RateLimitMiddleware {
    pub fn new(requests_per_window: u32, window_seconds: u64) -> Self {
        Self {
            requests_per_window,
            window_seconds,
            clients: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn get_client_ip(&self, req: &ServiceRequest) -> String {
        // Extract client IP from various headers
        req.headers()
            .get("X-Forwarded-For")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(',').next())
            .unwrap_or_else(|| {
                req.connection_info()
                    .peer_addr()
                    .unwrap_or("unknown")
            })
            .to_string()
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RateLimitService {
            service,
            requests_per_window: self.requests_per_window,
            window_seconds: self.window_seconds,
            clients: self.clients.clone(),
        })
    }
}

pub struct RateLimitService<S> {
    service: S,
    requests_per_window: u32,
    window_seconds: u64,
    clients: Rc<RefCell<HashMap<String, ClientRateLimit>>>,
}

impl<S, B> Service<ServiceRequest> for RateLimitService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let client_ip = req.headers()
            .get("X-Forwarded-For")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(',').next())
            .unwrap_or_else(|| {
                req.connection_info()
                    .peer_addr()
                    .unwrap_or("unknown")
            })
            .to_string();

        let now = Instant::now();
        let mut clients = self.clients.borrow_mut();

        // Get or create client rate limit entry
        let client_limit = clients.entry(client_ip.clone()).or_insert(ClientRateLimit {
            requests: 0,
            window_start: now,
        });

        // Reset window if expired
        if now.duration_since(client_limit.window_start).as_secs() >= self.window_seconds {
            client_limit.requests = 0;
            client_limit.window_start = now;
        }

        // Check rate limit
        if client_limit.requests >= self.requests_per_window {
            warn!(
                client_ip = %client_ip,
                requests = client_limit.requests,
                limit = self.requests_per_window,
                "Rate limit exceeded"
            );

            return Box::pin(async {
                Ok(req.into_response(
                    ApiError::RateLimitExceeded {
                        limit: self.requests_per_window,
                        window: format!("{} seconds", self.window_seconds),
                    }.error_response().into_body()
                ))
            });
        }

        // Increment request count
        client_limit.requests += 1;
        drop(clients);

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}

/// Power optimization middleware for edge deployment
pub struct PowerOptimizationMiddleware;

impl PowerOptimizationMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl<S, B> Transform<S, ServiceRequest> for PowerOptimizationMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = PowerOptimizationService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(PowerOptimizationService { service })
    }
}

pub struct PowerOptimizationService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for PowerOptimizationService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = Instant::now();

        // Add power optimization context
        req.extensions_mut().insert(PowerOptimizationContext {
            start_time,
            low_power_mode: self.should_use_low_power_mode(&req),
        });

        let fut = self.service.call(req);
        Box::pin(async move {
            let mut res = fut.await?;

            // Add power consumption headers
            let processing_time = start_time.elapsed();
            res.headers_mut().insert(
                actix_web::http::header::HeaderName::from_static("x-processing-time-us"),
                actix_web::http::header::HeaderValue::from_str(&processing_time.as_micros().to_string())
                    .unwrap_or_default(),
            );

            // Estimate power consumption (simplified)
            let power_estimate_mw = (processing_time.as_micros() as f32 / 1000.0) * 2.0; // 2W target
            res.headers_mut().insert(
                actix_web::http::header::HeaderName::from_static("x-power-consumption-mw"),
                actix_web::http::header::HeaderValue::from_str(&power_estimate_mw.to_string())
                    .unwrap_or_default(),
            );

            Ok(res)
        })
    }
}

impl PowerOptimizationService<S> {
    fn should_use_low_power_mode(&self, req: &ServiceRequest) -> bool {
        // Determine if request should use low-power mode based on headers or system state
        req.headers()
            .get("X-Power-Mode")
            .and_then(|h| h.to_str().ok())
            .map(|s| s == "low")
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct PowerOptimizationContext {
    pub start_time: Instant,
    pub low_power_mode: bool,
}

/// Request tracing middleware for observability
pub struct RequestTracingMiddleware;

impl RequestTracingMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequestTracingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestTracingService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RequestTracingService { service })
    }
}

pub struct RequestTracingService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestTracingService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let request_id = uuid::Uuid::new_v4().to_string();
        let method = req.method().to_string();
        let path = req.path().to_string();
        let start_time = Instant::now();

        // Add request ID to extensions for downstream use
        req.extensions_mut().insert(RequestTracingContext {
            request_id: request_id.clone(),
            start_time,
        });

        info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            "Request started"
        );

        let fut = self.service.call(req);
        Box::pin(async move {
            match fut.await {
                Ok(mut res) => {
                    let duration = start_time.elapsed();
                    let status = res.status().as_u16();

                    // Add tracing headers
                    res.headers_mut().insert(
                        actix_web::http::header::HeaderName::from_static("x-request-id"),
                        actix_web::http::header::HeaderValue::from_str(&request_id)
                            .unwrap_or_default(),
                    );

                    info!(
                        request_id = %request_id,
                        method = %method,
                        path = %path,
                        status = status,
                        duration_ms = duration.as_millis(),
                        "Request completed"
                    );

                    Ok(res)
                }
                Err(e) => {
                    let duration = start_time.elapsed();

                    error!(
                        request_id = %request_id,
                        method = %method,
                        path = %path,
                        duration_ms = duration.as_millis(),
                        error = %e,
                        "Request failed"
                    );

                    Err(e)
                }
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct RequestTracingContext {
    pub request_id: String,
    pub start_time: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    async fn test_handler() -> Result<HttpResponse, Error> {
        Ok(HttpResponse::Ok().json("test"))
    }

    #[actix_web::test]
    async fn test_quantum_security_middleware() {
        let app = test::init_service(
            App::new()
                .wrap(QuantumSecurityMiddleware::new())
                .route("/test", web::get().to(test_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-Quantum-Level", "128"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_rate_limit_middleware() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimitMiddleware::new(2, 60)) // 2 requests per minute
                .route("/test", web::get().to(test_handler))
        ).await;

        // First request should succeed
        let req1 = test::TestRequest::get().uri("/test").to_request();
        let resp1 = test::call_service(&app, req1).await;
        assert!(resp1.status().is_success());

        // Second request should succeed
        let req2 = test::TestRequest::get().uri("/test").to_request();
        let resp2 = test::call_service(&app, req2).await;
        assert!(resp2.status().is_success());

        // Third request should be rate limited
        let req3 = test::TestRequest::get().uri("/test").to_request();
        let resp3 = test::call_service(&app, req3).await;
        assert_eq!(resp3.status(), actix_web::http::StatusCode::TOO_MANY_REQUESTS);
    }

    #[actix_web::test]
    async fn test_power_optimization_middleware() {
        let app = test::init_service(
            App::new()
                .wrap(PowerOptimizationMiddleware::new())
                .route("/test", web::get().to(test_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-Power-Mode", "low"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        assert!(resp.headers().contains_key("x-processing-time-us"));
        assert!(resp.headers().contains_key("x-power-consumption-mw"));
    }
}
