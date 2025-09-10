use actix_web::{test, App};
use nocodo_services::api::health;
use serde_json::Value;

#[tokio::test]
async fn test_health_check() {
    let app = test::init_service(App::new().route(
        "/api/health",
        actix_web::web::get().to(health::health_check),
    ))
    .await;

    let req = test::TestRequest::get().uri("/api/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["timestamp"].is_string());
}

#[tokio::test]
async fn test_version_info() {
    let app = test::init_service(App::new().route(
        "/api/version",
        actix_web::web::get().to(health::version_info),
    ))
    .await;

    let req = test::TestRequest::get().uri("/api/version").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["version"], "0.1.0");
    assert_eq!(body["service"], "nocodo-services");
}
