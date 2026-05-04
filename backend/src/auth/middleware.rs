use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    web, Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
};

use super::AuthConfig;

pub struct RequireAuth;

impl<S, B> Transform<S, ServiceRequest> for RequireAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RequireAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequireAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct RequireAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequireAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let config = req.app_data::<web::Data<AuthConfig>>().cloned();

        Box::pin(async move {
            let Some(config) = config else {
                return svc.call(req).await.map(|r| r.map_into_left_body());
            };

            let path = req.path();
            let skip = !config.mandatory
                || path.starts_with("/api/auth/")
                || path == "/api/heartbeat";

            if skip {
                return svc.call(req).await.map(|r| r.map_into_left_body());
            }

            let token = req
                .cookie("nocodo_session")
                .map(|c| c.value().to_string());

            let db_url = config.db_url.clone();
            let valid = match token {
                None => false,
                Some(t) => tokio::task::spawn_blocking(move || {
                    crate::auth::validate_session_sync(&db_url, &t)
                })
                .await
                .unwrap_or(false),
            };

            if valid {
                svc.call(req).await.map(|r| r.map_into_left_body())
            } else {
                let response = HttpResponse::Unauthorized()
                    .json(serde_json::json!({"error": "authentication required"}));
                Ok(req.into_response(response).map_into_right_body())
            }
        })
    }
}
