use crate::auth::validate_token;
use crate::config::AppConfig;
use crate::error::AppError;
use crate::models::UserInfo;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    web, Error, HttpMessage,
};
use futures_util::future::{ready, Ready};
use std::sync::Arc;

/// Authentication middleware that extracts JWT token and attaches user info to request
pub struct AuthenticationMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthenticationMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthenticationMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationMiddlewareService { service }))
    }
}

pub struct AuthenticationMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future =
        futures_util::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let config = req.app_data::<web::Data<Arc<AppConfig>>>().cloned();

        // Skip authentication for health check, login, and register endpoints
        let path = req.path().to_string();
        if path == "/api/health" || path == "/api/auth/login" || path == "/api/auth/register" {
            let fut = self.service.call(req);
            return Box::pin(async move { fut.await });
        }

        let config = match config {
            Some(cfg) => cfg,
            None => {
                return Box::pin(async {
                    Err(ErrorUnauthorized("Server configuration error").into())
                });
            }
        };

        // Extract Authorization header
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let auth_header = match auth_header {
            Some(h) => h,
            None => {
                return Box::pin(async {
                    Err(ErrorUnauthorized("Missing Authorization header").into())
                });
            }
        };

        let token = match auth_header.strip_prefix("Bearer ") {
            Some(t) => t.to_string(),
            None => {
                return Box::pin(async {
                    Err(ErrorUnauthorized(
                        "Invalid Authorization header format. Expected 'Bearer <token>'",
                    )
                    .into())
                });
            }
        };

        // Validate token
        let jwt_secret = match config.jwt_secret.as_ref() {
            Some(secret) => secret.clone(),
            None => {
                return Box::pin(async {
                    Err(ErrorUnauthorized("JWT secret not configured").into())
                });
            }
        };

        match validate_token(&token, &jwt_secret) {
            Ok(claims) => {
                // Attach user info to request extensions
                let user_info = UserInfo {
                    id: claims.sub.parse().unwrap_or(0),
                    username: claims.username,
                    email: "".to_string(), // We don't store email in JWT claims
                };

                req.extensions_mut().insert(user_info);

                let fut = self.service.call(req);
                Box::pin(async move { fut.await })
            }
            Err(_) => Box::pin(async { Err(ErrorUnauthorized("Invalid or expired token").into()) }),
        }
    }
}

/// Extract user info from request extensions
pub fn get_user_from_request(req: &ServiceRequest) -> Result<UserInfo, AppError> {
    req.extensions()
        .get::<UserInfo>()
        .cloned()
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))
}

/// Extract user ID from request extensions
pub fn get_user_id_from_request(req: &ServiceRequest) -> Result<i64, AppError> {
    let user = get_user_from_request(req)?;
    Ok(user.id)
}

/// Permission requirement for a route
#[derive(Debug, Clone)]
pub struct PermissionRequirement {
    pub resource_type: String,
    pub action: String,
    pub resource_id_param: Option<String>, // Parameter name to extract resource_id from URL
}

impl PermissionRequirement {
    pub fn new(resource_type: &str, action: &str) -> Self {
        Self {
            resource_type: resource_type.to_string(),
            action: action.to_string(),
            resource_id_param: None,
        }
    }

    pub fn with_resource_id(mut self, param_name: &str) -> Self {
        self.resource_id_param = Some(param_name.to_string());
        self
    }
}

/// Permission enforcement middleware
pub struct PermissionMiddleware {
    pub requirement: PermissionRequirement,
}

impl PermissionMiddleware {
    pub fn new(requirement: PermissionRequirement) -> Self {
        Self { requirement }
    }
}

impl<S, B> Transform<S, ServiceRequest> for PermissionMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = PermissionMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PermissionMiddlewareService {
            service,
            requirement: self.requirement.clone(),
        }))
    }
}

pub struct PermissionMiddlewareService<S> {
    service: S,
    requirement: PermissionRequirement,
}

impl<S, B> Service<ServiceRequest> for PermissionMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future =
        futures_util::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let requirement = self.requirement.clone();

        // Get database from app data
        let database = match req.app_data::<web::Data<crate::handlers::AppState>>() {
            Some(state) => state.database.clone(),
            None => {
                return Box::pin(async {
                    Err(actix_web::error::ErrorInternalServerError("Database not available").into())
                });
            }
        };

        // Extract user ID from request
        let user_id = match get_user_id_from_request(&req) {
            Ok(id) => id,
            Err(_) => {
                return Box::pin(async {
                    Err(ErrorUnauthorized("Authentication required").into())
                });
            }
        };

        // Extract resource_id if needed
        let resource_id = if let Some(param_name) = &requirement.resource_id_param {
            req.match_info()
                .get(param_name)
                .and_then(|s| s.parse::<i64>().ok())
        } else {
            None
        };

        // Parse resource type and action
        let resource_type =
            match crate::permissions::ResourceType::from_str(&requirement.resource_type) {
                Some(rt) => rt,
                None => {
                    return Box::pin(async move {
                        Err(actix_web::error::ErrorInternalServerError(format!(
                            "Invalid resource type: {}",
                            requirement.resource_type
                        ))
                        .into())
                    });
                }
            };

        let action = match crate::permissions::Action::from_str(&requirement.action) {
            Some(a) => a,
            None => {
                return Box::pin(async move {
                    Err(actix_web::error::ErrorInternalServerError(format!(
                        "Invalid action: {}",
                        requirement.action
                    ))
                    .into())
                });
            }
        };

        let fut = self.service.call(req);
        Box::pin(async move {
            // Check permission
            match crate::permissions::check_permission(
                &database,
                user_id,
                resource_type,
                resource_id,
                action,
            )
            .await
            {
                Ok(true) => {
                    // Permission granted, proceed with request
                    fut.await
                }
                Ok(false) => {
                    // Permission denied
                    Err(actix_web::error::ErrorForbidden("Insufficient permissions").into())
                }
                Err(e) => {
                    // Error checking permission
                    tracing::error!("Permission check error: {}", e);
                    Err(
                        actix_web::error::ErrorInternalServerError("Permission check failed")
                            .into(),
                    )
                }
            }
        })
    }
}
