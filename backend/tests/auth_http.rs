//! The HTTP surface: public routes stay open, everything else needs a session.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use electricacl::api::{self, AppState};
use electricacl::auth::Auth;
use electricacl::store::Store;
use http_body_util::BodyExt;
use tokio::sync::RwLock;
use tower::ServiceExt;

fn temp_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "eacl-http-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

async fn app() -> Router {
    let dir = temp_dir();
    let store = Store::load(&dir).expect("store loads from the embedded seed");
    let (auth, _) =
        Auth::load(&dir, "admin", Some("correct-horse"), None, false).expect("auth initialises");
    let state = Arc::new(AppState {
        store: RwLock::new(store),
        auth,
    });
    api::router(state)
}

fn request(method: &str, uri: &str) -> axum::http::request::Builder {
    Request::builder()
        .method(method)
        .uri(uri)
        .extension(ConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            5555,
        ))))
}

#[tokio::test]
async fn health_is_public_and_state_is_not() {
    let app = app().await;

    let health = app
        .clone()
        .oneshot(request("GET", "/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);

    let state = app
        .clone()
        .oneshot(request("GET", "/api/state").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(state.status(), StatusCode::UNAUTHORIZED);

    let export = app
        .oneshot(
            request("GET", "/api/export/csv")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_sets_a_session_cookie_that_unlocks_the_api() {
    let app = app().await;

    let login = app
        .clone()
        .oneshot(
            request("POST", "/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"correct-horse"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);

    let cookie = login
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .expect("login returns a cookie")
        .split(';')
        .next()
        .unwrap()
        .to_string();
    assert!(cookie.starts_with("eacl_session="));
    assert!(login.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .contains("HttpOnly"));

    let state = app
        .oneshot(
            request("GET", "/api/state")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(state.status(), StatusCode::OK);
    let body = state.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 656);
}

#[tokio::test]
async fn wrong_password_is_rejected_without_a_cookie() {
    let app = app().await;
    let response = app
        .oneshot(
            request("POST", "/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username":"admin","password":"nope"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
}

#[tokio::test]
async fn logout_clears_the_cookie() {
    let app = app().await;
    let response = app
        .oneshot(
            request("POST", "/api/auth/logout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(cookie.contains("Max-Age=0"));
}
