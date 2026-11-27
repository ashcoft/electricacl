use std::fmt::Write as _;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;

use crate::calc::{self, ItemView, ScenarioSummary, StateView};
use crate::models::{CalcSettings, LoadItem, MotorRef, ProjectMeta};
use crate::store::Store;

pub struct AppState {
    pub store: RwLock<Store>,
}

type Shared = Arc<AppState>;

pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Internal(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
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
    Router::new()
        .route("/api/health", get(health))
        .route("/api/state", get(get_state))
        .route("/api/summaries", get(get_summaries))
        .route("/api/motor-table", get(get_motor_table))
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/load-items", post(create_item).put(replace_items))
        .route("/api/load-items/reset", post(reset_items))
        .route("/api/load-items/{id}", put(update_item).delete(delete_item))
        .route("/api/load-items/{id}/duplicate", post(duplicate_item))
        .route("/api/export/csv", get(export_csv))
        .with_state(state)
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

/// Expose the calculation for the unit tests without going through HTTP.
pub fn summaries_for(
    items: &[LoadItem],
    table: &calc::MotorTable,
    settings: &CalcSettings,
) -> Vec<ScenarioSummary> {
    calc::summarize(items, table, settings)
}
