use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use std::{
    collections::HashSet,
    future::{ready, Ready},
    rc::Rc,
};
use tracing::{debug, warn};

use crate::auth::AuthService;

pub struct ApiKeyAuth {
    auth_service: Rc<AuthService>,
}

impl ApiKeyAuth {
    pub fn new(auth_service: AuthService) -> Self {
        Self {
            auth_service: Rc::new(auth_service),
        }
    }
}

impl Default for ApiKeyAuth {
    fn default() -> Self {
        Self::new(AuthService::new())
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiKeyAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ApiKeyAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiKeyAuthMiddleware {
            service: Rc::new(service),
            auth_service: self.auth_service.clone(),
        }))
    }
}

pub struct ApiKeyAuthMiddleware<S> {
    service: Rc<S>,
    auth_service: Rc<AuthService>,
}

impl<S, B> Service<ServiceRequest> for ApiKeyAuthMiddleware<S>
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
        let auth_service = self.auth_service.clone();

        Box::pin(async move {
            let path = req.path();
            let method = req.method().as_str();
            let client_ip = req
                .connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown")
                .to_string();

            // Log all access attempts for security monitoring
            debug!("Access attempt: {} {} from {}", method, path, client_ip);

            // Define public endpoints that don't require authentication
            let public_endpoints: HashSet<&str> = [
                "/health",
                "/api/v1/health",
            ]
            .iter()
            .cloned()
            .collect();

            // Check if this is a public endpoint
            if public_endpoints.contains(path) {
                return service.call(req).await;
            }

            // All other endpoints require API key authentication
            let api_key = req
                .headers()
                .get("X-API-Key")
                .and_then(|h| h.to_str().ok());

            match api_key {
                Some(key) => {
                    // Validate the API key - simplified to avoid generics issues
                    if let Some(api_key_data) = auth_service.validate_api_key(key).await {
                        if !auth_service.is_key_expired(&api_key_data) {
                            // Check basic permissions
                            let has_permission = if path.starts_with("/api/v1/auth") || path.starts_with("/metrics")
                            {
                                api_key_data.permissions.contains(&"admin".to_string())
                            } else if path.starts_with("/api/v1/neuromorphic") {
                                api_key_data.permissions.contains(&"neuromorphic".to_string())
                                    || api_key_data.permissions.contains(&"admin".to_string())
                            } else if path.starts_with("/api/v1/quantum") {
                                api_key_data.permissions.contains(&"quantum".to_string())
                                    || api_key_data.permissions.contains(&"admin".to_string())
                            } else if path.starts_with("/api/v1/dna") {
                                api_key_data.permissions.contains(&"dna".to_string())
                                    || api_key_data.permissions.contains(&"admin".to_string())
                            } else {
                                api_key_data.permissions.contains(&"read".to_string())
                                    || api_key_data.permissions.contains(&"admin".to_string())
                            };

                            if has_permission {
                                debug!(
                                    "Authorized access: {} {} from {} with key {}",
                                    method,
                                    path,
                                    client_ip,
                                    &key[..8]
                                );
                                req.extensions_mut().insert(api_key_data);
                                return service.call(req).await;
                            }
                        }
                    }

                    // Authentication failed
                    warn!("Unauthorized access attempt to {} from {}", path, client_ip);

                    // Use actix_web's error handling instead of complex generics
                    Err(actix_web::error::ErrorUnauthorized("Authentication required"))
                }
                None => {
                    warn!("No API key provided for {} from {}", path, client_ip);
                    Err(actix_web::error::ErrorUnauthorized("API key required"))
                }
            }
        })
    }
}
