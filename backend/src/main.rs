use std::env;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::sync::RwLock;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use electricacl::api::{self, AppState};
use electricacl::auth::{Auth, CapConfig};
use electricacl::store::Store;

fn env_or(name: &str, fallback: &str) -> String {
    env::var(name).unwrap_or_else(|_| fallback.to_string())
}

/// Minimal HTTP probe so the container image needs no curl/wget.
fn healthcheck(port: u16) -> i32 {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_secs(3)) else {
        eprintln!("healthcheck: cannot connect to {address}");
        return 1;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let request = "GET /api/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    if stream.write_all(request.as_bytes()).is_err() {
        eprintln!("healthcheck: cannot send request");
        return 1;
    }
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    if response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200") {
        0
    } else {
        eprintln!(
            "healthcheck: unexpected response: {}",
            response.lines().next().unwrap_or("")
        );
        1
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "electricacl=info,tower_http=warn".into()),
        )
        .init();

    let port: u16 = env_or("PORT", "8080")
        .parse()
        .context("PORT must be a number")?;

    if env::args().any(|arg| arg == "--healthcheck") {
        std::process::exit(healthcheck(port));
    }

    let data_dir = PathBuf::from(env_or("DATA_DIR", "./data"));
    let static_dir = PathBuf::from(env_or("STATIC_DIR", "./static"));
    let auth_user = env_or("AUTH_USERNAME", "admin");

    if env::args().any(|arg| arg == "--reset-password") {
        let password = Auth::reset_password(&data_dir, &auth_user)?;
        println!("New password for '{auth_user}': {password}");
        return Ok(());
    }

    let cap_endpoint = env::var("CAP_API_ENDPOINT")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let cap_secret = env::var("CAP_SECRET").ok().filter(|v| !v.trim().is_empty());
    let cap = match (cap_endpoint, cap_secret) {
        (Some(endpoint), Some(secret)) => Some(CapConfig { endpoint, secret }),
        (None, None) => None,
        _ => {
            return Err(anyhow::anyhow!(
                "CAP_API_ENDPOINT and CAP_SECRET must be set together (or neither)"
            ))
        }
    };
    let cookie_secure = matches!(
        env_or("COOKIE_SECURE", "false")
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    );

    let auth_password = env::var("AUTH_PASSWORD").ok().filter(|v| !v.is_empty());
    let (auth, generated_password) = Auth::load(
        &data_dir,
        &auth_user,
        auth_password.as_deref(),
        cap,
        cookie_secure,
    )?;
    if let Some(password) = generated_password {
        tracing::warn!("==============================================================");
        tracing::warn!("  initial login: {auth_user} / {password}");
        tracing::warn!("  change it with AUTH_PASSWORD or: electricacl --reset-password");
        tracing::warn!("==============================================================");
    }
    if !auth.cap_required() {
        tracing::info!("CAPTCHA disabled (set CAP_API_ENDPOINT and CAP_SECRET to enable Cap)");
    }

    let store = Store::load(&data_dir)?;
    if let Err(error) = store.persist() {
        tracing::warn!(%error, "state file is not writable, changes will not survive a restart");
    }
    tracing::info!(
        items = store.items().len(),
        motors = store.motor_table().rows().len(),
        "load list loaded"
    );

    let state = Arc::new(AppState {
        store: RwLock::new(store),
        auth,
    });

    let index = static_dir.join("index.html");
    if !index.exists() {
        tracing::warn!(
            path = %index.display(),
            "frontend bundle not found - only the API will be served"
        );
    }
    let spa = ServeDir::new(&static_dir).not_found_service(ServeFile::new(&index));

    let app = api::router(state)
        .fallback_service(spa)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("binding {address}"))?;
    tracing::info!("listening on http://{address}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received");
    })
    .await
    .context("server error")?;

    Ok(())
}
