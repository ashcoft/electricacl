use std::fmt::Write as _;
use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;

use crate::auth::{self, Auth, AuthStatus, LoginError};
use crate::cable::{CableCircuit, CableReference, CableStateView};
use crate::cable_store::CableStore;
use crate::calc::{self, ItemView, ScenarioSummary, StateView};
use crate::models::{CalcSettings, LoadItem, MotorRef, ProjectMeta, Scenario};
use crate::store::Store;

/// Same-origin reverse proxy for the Cap server, so the widget never needs a
/// publicly reachable CAPTCHA host and no CORS rules are involved.
#[derive(Debug, Clone)]
pub struct CapProxy {
    pub upstream: String,
    pub site_key: String,
}

pub struct AppState {
    pub store: RwLock<Store>,
    pub cable: RwLock<CableStore>,
    pub auth: Auth,
    pub cap_proxy: Option<CapProxy>,
}

/// User name attached to authenticated requests.
#[derive(Clone)]
pub struct CurrentUser(pub String);

type Shared = Arc<AppState>;

pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    TooManyRequests(String),
    Internal(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message),
            ApiError::TooManyRequests(message) => (StatusCode::TOO_MANY_REQUESTS, message),
            ApiError::Internal(error) => {
                tracing::error!(%error, "request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        ApiError::Internal(error)
    }
}

pub fn router(state: Shared) -> Router {
    let public = Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/config", get(auth_config))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/cap/{site_key}/{*path}", get(cap_proxy).post(cap_proxy));

    let protected = Router::new()
        .route("/api/state", get(get_state))
        .route("/api/summaries", get(get_summaries))
        .route("/api/motor-table", get(get_motor_table))
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/load-items", post(create_item).put(replace_items))
        .route("/api/load-items/reset", post(reset_items))
        .route("/api/load-items/{id}", put(update_item).delete(delete_item))
        .route("/api/load-items/{id}/duplicate", post(duplicate_item))
        .route("/api/export/csv", get(export_csv))
        .route("/api/export/pdf", get(export_pdf))
        .route("/api/cable/state", get(get_cable_state))
        .route("/api/cable/reference", put(put_cable_reference))
        .route("/api/cable/circuits", post(create_cable_circuit))
        .route("/api/cable/circuits/reset", post(reset_cable_circuits))
        .route(
            "/api/cable/circuits/{id}",
            put(update_cable_circuit).delete(delete_cable_circuit),
        )
        .route(
            "/api/cable/circuits/{id}/duplicate",
            post(duplicate_cable_circuit),
        )
        .route("/api/export/cable.csv", get(export_cable_csv))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    public.merge(protected).with_state(state)
}

/// Baseline hardening applied to every response: no MIME sniffing, no framing
/// (clickjacking protection for the sign-in form), no referrer leakage, and
/// `no-store` on the API so authenticated payloads are never cached.
pub async fn security_headers(request: Request, next: Next) -> Response {
    let is_api = request.uri().path().starts_with("/api/");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    if is_api {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}

fn session_user(state: &AppState, headers: &HeaderMap) -> Option<String> {
    let header = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    auth::session_from_cookie_header(header).and_then(|token| state.auth.verify_token(&token))
}

async fn require_auth(State(state): State<Shared>, mut request: Request, next: Next) -> Response {
    match session_user(&state, request.headers()) {
        Some(user) => {
            request.extensions_mut().insert(CurrentUser(user));
            next.run(request).await
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "authentication required" })),
        )
            .into_response(),
    }
}

/// Forward `GET|POST /cap/<site>/<path>` to `<upstream>/<site>/<path>`.
///
/// Only the configured site key is proxied, so this cannot be used to reach
/// anything else on the Cap host.
async fn cap_proxy(
    State(state): State<Shared>,
    Path((site_key, path)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let Some(proxy) = state.cap_proxy.clone() else {
        return Err(ApiError::NotFound(
            "CAPTCHA proxy is not enabled on this deployment".into(),
        ));
    };
    if site_key != proxy.site_key {
        return Err(ApiError::NotFound("unknown CAPTCHA site".into()));
    }
    let url = format!(
        "{}/{}/{}",
        proxy.upstream.trim_end_matches('/'),
        site_key,
        path
    );
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json")
        .to_string();
    let is_get = method == Method::GET;

    let (status, content_type, payload) =
        tokio::task::spawn_blocking(move || -> anyhow::Result<(u16, String, Vec<u8>)> {
            let result = if is_get {
                ureq::get(&url).call()
            } else {
                ureq::post(&url)
                    .set("Content-Type", &content_type)
                    .send_bytes(&body)
            };
            let response = match result {
                Ok(response) => response,
                Err(ureq::Error::Status(_, response)) => response,
                Err(error) => anyhow::bail!("CAPTCHA upstream unreachable: {error}"),
            };
            let status = response.status();
            let content_type = response
                .header("content-type")
                .unwrap_or("application/json")
                .to_string();
            let mut payload = Vec::new();
            response.into_reader().read_to_end(&mut payload)?;
            Ok((status, content_type, payload))
        })
        .await
        .map_err(|error| ApiError::Internal(anyhow::anyhow!("CAPTCHA proxy failed: {error}")))??;

    Ok((
        StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        payload,
    )
        .into_response())
}

async fn auth_config(State(state): State<Shared>, headers: HeaderMap) -> Json<AuthStatus> {
    let user = session_user(&state, &headers);
    Json(state.auth.status(user.as_deref()))
}

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
    #[serde(default)]
    cap_token: Option<String>,
}

async fn login(
    State(state): State<Shared>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(body): Json<LoginBody>,
) -> Result<Response, ApiError> {
    let ip = peer.ip();
    match state
        .auth
        .login(
            ip,
            &body.username,
            &body.password,
            body.cap_token.as_deref(),
        )
        .await
    {
        Ok(token) => {
            let cookie = state.auth.session_cookie(&token);
            Ok((
                [(header::SET_COOKIE, cookie)],
                Json(json!({ "user": body.username.trim() })),
            )
                .into_response())
        }
        Err(LoginError::RateLimited) => Err(ApiError::TooManyRequests(
            "Too many failed attempts, try again in a few minutes".into(),
        )),
        Err(LoginError::BadCredentials) => Err(ApiError::Unauthorized(
            "Invalid login ID or password".into(),
        )),
        Err(LoginError::Captcha(message)) => Err(ApiError::BadRequest(message)),
        Err(LoginError::Internal(error)) => Err(ApiError::Internal(error)),
    }
}

async fn logout(State(state): State<Shared>, headers: HeaderMap) -> Response {
    let header = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    if let Some(token) = auth::session_from_cookie_header(header) {
        state.auth.revoke(&token);
    }
    (
        [(header::SET_COOKIE, state.auth.expired_cookie())],
        Json(json!({ "ok": true })),
    )
        .into_response()
}

fn app_version() -> String {
    std::env::var("APP_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "electricacl",
        "version": app_version(),
    }))
}

async fn get_cable_state(State(state): State<Shared>) -> Json<CableStateView> {
    let cable = state.cable.read().await;
    Json(cable.state())
}

async fn create_cable_circuit(
    State(state): State<Shared>,
    Json(circuit): Json<CableCircuit>,
) -> Result<(StatusCode, Json<CableCircuit>), ApiError> {
    let mut cable = state.cable.write().await;
    let created = cable.create(circuit)?;
    Ok((StatusCode::CREATED, Json(created)))
}

async fn update_cable_circuit(
    State(state): State<Shared>,
    Path(id): Path<String>,
    Json(circuit): Json<CableCircuit>,
) -> Result<Json<CableCircuit>, ApiError> {
    let mut cable = state.cable.write().await;
    match cable.update(&id, circuit)? {
        Some(updated) => Ok(Json(updated)),
        None => Err(ApiError::NotFound(format!("cable circuit {id} not found"))),
    }
}

async fn delete_cable_circuit(
    State(state): State<Shared>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut cable = state.cable.write().await;
    if cable.delete(&id)? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("cable circuit {id} not found")))
    }
}

async fn duplicate_cable_circuit(
    State(state): State<Shared>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<CableCircuit>), ApiError> {
    let mut cable = state.cable.write().await;
    match cable.duplicate(&id)? {
        Some(copy) => Ok((StatusCode::CREATED, Json(copy))),
        None => Err(ApiError::NotFound(format!("cable circuit {id} not found"))),
    }
}

#[derive(Deserialize)]
struct ReferenceBody {
    reference: Vec<CableReference>,
}

async fn put_cable_reference(
    State(state): State<Shared>,
    Json(body): Json<ReferenceBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut cable = state.cable.write().await;
    let count = cable.replace_reference(body.reference)?;
    Ok(Json(json!({ "reference_rows": count })))
}

async fn reset_cable_circuits(
    State(state): State<Shared>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut cable = state.cable.write().await;
    let count = cable.reset()?;
    Ok(Json(json!({ "restored": count })))
}

async fn export_cable_csv(State(state): State<Shared>) -> Result<Response, ApiError> {
    let cable = state.cable.read().await;
    let view = cable.state();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# {} - {} - cable sizing",
        csv_escape(&view.project.name),
        csv_escape(&view.project.document)
    );
    let _ = writeln!(
        out,
        "si_no,tag,group,from,to,feeder_load,voltage_v,phases,power_factor,load_kw,length_m,runs,\
cores,size_mm2,ampacity_a,grouping_factor,temperature_factor,derating,iz_a,iflc_a,runs_required,\
protection_rating_a,resistance_ohm_km,reactance_ohm_km,vd_v,vd_pct,vd_last_pct,max_length_m,\
chain_length_m,check_ampacity,check_protection,check_voltage_drop,check_max_length,acceptable,remark"
    );
    for row in &view.circuits {
        let circuit = &row.circuit;
        let derived = &row.derived;
        let reference = view.reference.iter().find(|entry| {
            entry.cores == circuit.cores && (entry.size_mm2 - circuit.size_mm2).abs() < 1e-9
        });
        let checks = derived.checks;
        let values = [
            circuit.si_no.map(|n| n.to_string()).unwrap_or_default(),
            csv_escape(&optional(&circuit.tag)),
            csv_escape(&optional(&circuit.group)),
            csv_escape(&optional(&circuit.from_tag)),
            csv_escape(&optional(&circuit.to_tag)),
            csv_escape(&optional(&circuit.feeder_load)),
            format!("{:.0}", circuit.voltage_v),
            csv_escape(circuit.phases.key()),
            format!("{:.3}", circuit.power_factor),
            format!("{:.3}", circuit.load_kw),
            format!("{:.1}", circuit.length_m),
            circuit.runs_selected.to_string(),
            csv_escape(circuit.cores.key()),
            format!("{:.2}", circuit.size_mm2),
            format!("{:.1}", derived.ampacity_a),
            format!("{:.3}", circuit.grouping_factor),
            format!("{:.3}", circuit.temperature_factor),
            format!("{:.4}", derived.derating),
            format!("{:.2}", derived.iz_a),
            format!("{:.3}", derived.iflc_a),
            format!("{:.4}", derived.runs_required),
            format!("{:.1}", circuit.protection_rating_a),
            reference
                .map(|r| format!("{:.4}", r.resistance_ohm_km))
                .unwrap_or_default(),
            reference
                .map(|r| format!("{:.4}", r.reactance_ohm_km))
                .unwrap_or_default(),
            format!("{:.4}", derived.vd_v),
            format!("{:.5}", derived.vd_pct),
            derived
                .vd_last_pct
                .map(|v| format!("{v:.5}"))
                .unwrap_or_default(),
            derived
                .max_length_m
                .map(|v| format!("{v:.1}"))
                .unwrap_or_default(),
            derived
                .chain_length_m
                .map(|v| format!("{v:.1}"))
                .unwrap_or_default(),
            verdict(checks.ampacity),
            verdict(checks.protection),
            checks.voltage_drop.map(verdict).unwrap_or_default(),
            checks.max_length.map(verdict).unwrap_or_default(),
            verdict(derived.acceptable),
            csv_escape(&optional(&circuit.remark)),
        ];
        let _ = writeln!(out, "{}", values.join(","));
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"cable-sizing.csv\"".to_string(),
            ),
        ],
        out,
    )
        .into_response())
}

fn verdict(value: bool) -> String {
    if value {
        "ACCEPTABLE".to_string()
    } else {
        "NOT-ACCEPTABLE".to_string()
    }
}

async fn get_state(State(state): State<Shared>) -> Json<StateView> {
    let store = state.store.read().await;
    Json(store.state())
}

async fn get_summaries(State(state): State<Shared>) -> Json<Vec<ScenarioSummary>> {
    let store = state.store.read().await;
    let state = store.state();
    Json(state.summaries)
}

async fn get_motor_table(State(state): State<Shared>) -> Json<Vec<MotorRef>> {
    let store = state.store.read().await;
    Json(store.motor_table().rows().to_vec())
}

async fn get_settings(State(state): State<Shared>) -> Json<CalcSettings> {
    let store = state.store.read().await;
    Json(store.settings().clone())
}

async fn put_settings(
    State(state): State<Shared>,
    Json(settings): Json<CalcSettings>,
) -> Result<Json<CalcSettings>, ApiError> {
    let mut store = state.store.write().await;
    Ok(Json(store.set_settings(settings)?))
}

async fn create_item(
    State(state): State<Shared>,
    Json(item): Json<LoadItem>,
) -> Result<(StatusCode, Json<LoadItem>), ApiError> {
    let mut store = state.store.write().await;
    let created = store.create(item)?;
    Ok((StatusCode::CREATED, Json(created)))
}

async fn update_item(
    State(state): State<Shared>,
    Path(id): Path<String>,
    Json(item): Json<LoadItem>,
) -> Result<Json<LoadItem>, ApiError> {
    let mut store = state.store.write().await;
    match store.update(&id, item)? {
        Some(updated) => Ok(Json(updated)),
        None => Err(ApiError::NotFound(format!("load item {id} not found"))),
    }
}

async fn delete_item(
    State(state): State<Shared>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut store = state.store.write().await;
    if store.delete(&id)? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("load item {id} not found")))
    }
}

async fn duplicate_item(
    State(state): State<Shared>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<LoadItem>), ApiError> {
    let mut store = state.store.write().await;
    match store.duplicate(&id)? {
        Some(copy) => Ok((StatusCode::CREATED, Json(copy))),
        None => Err(ApiError::NotFound(format!("load item {id} not found"))),
    }
}

#[derive(Deserialize)]
struct ReplaceBody {
    items: Vec<LoadItem>,
}

async fn replace_items(
    State(state): State<Shared>,
    Json(body): Json<ReplaceBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut store = state.store.write().await;
    let count = store.replace_all(body.items)?;
    Ok(Json(json!({ "replaced": count })))
}

async fn reset_items(State(state): State<Shared>) -> Result<Json<serde_json::Value>, ApiError> {
    let mut store = state.store.write().await;
    let count = store.reset()?;
    Ok(Json(json!({ "restored": count })))
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn optional(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

fn build_csv(project: &ProjectMeta, items: &[ItemView]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# {} - {} - revision {}",
        csv_escape(&project.name),
        csv_escape(&project.document),
        csv_escape(&project.revision)
    );
    let _ = writeln!(
        out,
        "line_no,tag,description,area,system,equipment_class,feeder_type,duty,load_type,scenarios,\
voltage_kv,absorbed_kw,rated_kw,load_factor,efficiency,power_factor,flc_a,consumed_kw,\
consumed_kvar,consumed_kva,starting_current_a,starting_kva,switchgear,status,revision,remark"
    );
    for view in items {
        let item = &view.item;
        let derived = &view.derived;
        let scenarios = item
            .scenarios
            .iter()
            .map(|scenario| scenario.key())
            .collect::<Vec<_>>()
            .join(" | ");
        let row = [
            item.line_no.map(|n| n.to_string()).unwrap_or_default(),
            csv_escape(&optional(&item.tag)),
            csv_escape(&optional(&item.description)),
            csv_escape(&optional(&item.area)),
            csv_escape(&optional(&item.system)),
            csv_escape(item.equipment_class.key()),
            csv_escape(&optional(&item.feeder_type)),
            csv_escape(item.duty.map(|d| d.key()).unwrap_or("unassigned")),
            csv_escape(&optional(&item.load_type)),
            csv_escape(&scenarios),
            format!("{:.3}", item.voltage_kv),
            format!("{:.3}", item.absorbed_kw),
            format!("{:.3}", item.rated_kw),
            format!("{:.4}", derived.load_factor),
            format!("{:.4}", derived.efficiency),
            format!("{:.4}", derived.power_factor),
            format!("{:.2}", derived.flc_a),
            format!("{:.3}", derived.consumed_kw),
            format!("{:.3}", derived.consumed_kvar),
            format!("{:.3}", derived.consumed_kva),
            derived
                .starting_current_a
                .map(|v| format!("{v:.2}"))
                .unwrap_or_default(),
            derived
                .starting_kva
                .map(|v| format!("{v:.2}"))
                .unwrap_or_default(),
            csv_escape(&optional(&item.switchgear)),
            csv_escape(&optional(&item.status)),
            csv_escape(&optional(&item.revision)),
            csv_escape(&optional(&item.remark)),
        ];
        let _ = writeln!(out, "{}", row.join(","));
    }
    out
}

async fn export_csv(State(state): State<Shared>) -> Result<Response, ApiError> {
    let store = state.store.read().await;
    let state_view = store.state();
    let csv = build_csv(&state_view.project, &state_view.items);
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"load-list.csv\"",
            ),
        ],
        csv,
    )
        .into_response())
}

#[derive(Deserialize)]
struct ReportQuery {
    #[serde(default)]
    scenario: Option<String>,
}

async fn export_pdf(
    State(state): State<Shared>,
    Query(query): Query<ReportQuery>,
) -> Result<Response, ApiError> {
    let scenario = match query.scenario.as_deref() {
        None => Scenario::Normal,
        Some(key) => Scenario::ALL
            .into_iter()
            .find(|candidate| candidate.key() == key)
            .ok_or_else(|| ApiError::BadRequest(format!("unknown scenario '{key}'")))?,
    };

    let store = state.store.read().await;
    let view = store.state();
    drop(store);

    let pdf = tokio::task::spawn_blocking(move || crate::report::render(&view, scenario))
        .await
        .map_err(|error| ApiError::Internal(anyhow::anyhow!("report task failed: {error}")))??;

    let filename = format!("load-list-{}.pdf", scenario.key());
    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        pdf,
    )
        .into_response())
}

/// Expose the calculation for the unit tests without going through HTTP.
pub fn summaries_for(
    items: &[LoadItem],
    table: &calc::MotorTable,
    settings: &CalcSettings,
) -> Vec<ScenarioSummary> {
    calc::summarize(items, table, settings)
}
