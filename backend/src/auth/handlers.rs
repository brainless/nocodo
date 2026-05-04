use actix_web::{cookie::time::Duration, get, post, web, HttpRequest, HttpResponse};
use actix_web::cookie::{Cookie, SameSite};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::{
    create_session, delete_session, generate_otp, get_session_email, send_otp_email, store_otp,
    verify_and_consume_otp, AuthConfig,
};

#[derive(Deserialize)]
pub struct OtpRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct VerifyOtpRequest {
    pub email: String,
    pub otp: String,
}

#[derive(Serialize)]
struct MeResponse {
    email: String,
}

fn session_cookie(token: &str, max_age: Duration) -> Cookie<'static> {
    Cookie::build("nocodo_session", token.to_owned())
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(max_age)
        .finish()
}

#[post("/api/auth/request-otp")]
pub async fn request_otp(
    body: web::Json<OtpRequest>,
    config: web::Data<AuthConfig>,
) -> HttpResponse {
    let email = body.email.trim().to_lowercase();
    let otp = generate_otp();
    let db_url = config.db_url.clone();
    let otp_store = otp.clone();
    let email_store = email.clone();

    let store_result = tokio::task::spawn_blocking(move || {
        Connection::open(&db_url).and_then(|conn| store_otp(&conn, &email_store, &otp_store))
    })
    .await;

    if store_result.is_err() || store_result.unwrap().is_err() {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "failed to store OTP"}));
    }

    match (&config.resend_api_key, &config.from_email) {
        (Some(api_key), Some(from_email)) => {
            if let Err(e) = send_otp_email(api_key, from_email, &email, &otp).await {
                log::error!("Failed to send OTP email to {}: {}", email, e);
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "failed to send email"}));
            }
        }
        _ => {
            log::error!("RESEND_API_KEY or AUTH_FROM_EMAIL not configured");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "email service not configured"}));
        }
    }

    HttpResponse::Ok().json(serde_json::json!({"message": "OTP sent"}))
}

#[post("/api/auth/verify-otp")]
pub async fn verify_otp(
    body: web::Json<VerifyOtpRequest>,
    config: web::Data<AuthConfig>,
) -> HttpResponse {
    let email = body.email.trim().to_lowercase();
    let otp = body.otp.trim().to_owned();
    let db_url = config.db_url.clone();

    let result = tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&db_url)?;
        let valid = verify_and_consume_otp(&conn, &email, &otp)?;
        if valid {
            create_session(&conn, &email).map(Some)
        } else {
            Ok(None)
        }
    })
    .await;

    match result {
        Ok(Ok(Some(token))) => HttpResponse::Ok()
            .cookie(session_cookie(&token, Duration::hours(24)))
            .json(serde_json::json!({"message": "authenticated"})),
        Ok(Ok(None)) => HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "invalid or expired OTP"})),
        _ => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "internal error"})),
    }
}

#[post("/api/auth/logout")]
pub async fn logout(req: HttpRequest, config: web::Data<AuthConfig>) -> HttpResponse {
    if let Some(token) = req.cookie("nocodo_session").map(|c| c.value().to_string()) {
        let db_url = config.db_url.clone();
        let _ = tokio::task::spawn_blocking(move || {
            Connection::open(&db_url).and_then(|conn| delete_session(&conn, &token))
        })
        .await;
    }
    HttpResponse::Ok()
        .cookie(session_cookie("", Duration::ZERO))
        .json(serde_json::json!({"message": "logged out"}))
}

#[get("/api/auth/me")]
pub async fn me(req: HttpRequest, config: web::Data<AuthConfig>) -> HttpResponse {
    let Some(token) = req.cookie("nocodo_session").map(|c| c.value().to_string()) else {
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "not authenticated"}));
    };

    let db_url = config.db_url.clone();
    let result = tokio::task::spawn_blocking(move || {
        Connection::open(&db_url).and_then(|conn| get_session_email(&conn, &token))
    })
    .await;

    match result {
        Ok(Ok(Some(email))) => HttpResponse::Ok().json(MeResponse { email }),
        _ => HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "not authenticated"})),
    }
}
