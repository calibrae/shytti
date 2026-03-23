use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    routing::{delete, get, post},
};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::bridge::HermyttBridge;
use crate::config::Config;
use crate::error::Error;
use crate::shell::{ShellInfo, ShellManager, SpawnRequest};

/// Default max shells if not configured.
const DEFAULT_MAX_SHELLS: usize = 64;

struct AppState {
    manager: ShellManager,
    bridge: HermyttBridge,
    api_key: String,
    max_shells: usize,
    allowed_hosts: Vec<String>,
    /// shell_id → hermytt session_id
    sessions: Mutex<HashMap<String, String>>,
}

pub fn router(cfg: &Config, manager: ShellManager, bridge: HermyttBridge) -> Router {
    let allowed_hosts: Vec<String> = cfg.shells.iter()
        .filter_map(|s| s.host.clone())
        .collect();

    let state = Arc::new(AppState {
        manager,
        bridge,
        api_key: cfg.daemon.hermytt_key.clone(),
        max_shells: cfg.daemon.max_shells.unwrap_or(DEFAULT_MAX_SHELLS),
        allowed_hosts,
        sessions: Mutex::new(HashMap::new()),
    });

    Router::new()
        .route("/shells", post(spawn_shell))
        .route("/shells", get(list_shells))
        .route("/shells/{id}", delete(kill_shell))
        .route("/shells/{id}/resize", post(resize_shell))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(axum::extract::DefaultBodyLimit::max(65536)) // 64KB max body
        .with_state(state)
}

pub async fn serve(cfg: Config, manager: ShellManager, bridge: HermyttBridge) -> Result<(), Error> {
    let app = router(&cfg, manager, bridge);
    let listener = tokio::net::TcpListener::bind(&cfg.daemon.listen).await?;
    tracing::info!(addr = %cfg.daemon.listen, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    // If no API key is configured, skip auth (local dev)
    if state.api_key.is_empty() {
        return Ok(next.run(request).await);
    }

    let provided = headers
        .get("x-shytti-key")
        .and_then(|v| v.to_str().ok());

    match provided {
        Some(key) if key == state.api_key => Ok(next.run(request).await),
        _ => {
            tracing::warn!("rejected unauthenticated request");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

async fn spawn_shell(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SpawnRequest>,
) -> Result<Json<ShellInfo>, Error> {
    let id = state.manager.spawn_with_limits(
        req,
        state.max_shells,
        &state.allowed_hosts,
    ).await?;

    // Bridge to Hermytt
    match state.bridge.attach(&id, &state.manager).await {
        Ok(session_id) => {
            state.sessions.lock().await.insert(id.clone(), session_id);
        }
        Err(e) => tracing::warn!(shell_id = %id, "hermytt bridge failed: {e}"),
    }

    let shells = state.manager.list().await;
    let info = shells.into_iter().find(|s| s.id == id)
        .ok_or_else(|| Error::NotFound(id))?;
    Ok(Json(info))
}

async fn list_shells(State(state): State<Arc<AppState>>) -> Json<Vec<ShellInfo>> {
    Json(state.manager.list().await)
}

async fn kill_shell(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ShellInfo>, Error> {
    // Detach from Hermytt
    if let Some(session_id) = state.sessions.lock().await.remove(&id) {
        if let Err(e) = state.bridge.detach(&session_id).await {
            tracing::warn!(%id, "hermytt detach failed: {e}");
        }
    }

    Ok(Json(state.manager.kill(&id).await?))
}

#[derive(Deserialize)]
struct ResizeRequest {
    rows: u16,
    cols: u16,
}

async fn resize_shell(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ResizeRequest>,
) -> Result<(), Error> {
    state.manager.resize(&id, req.rows, req.cols).await
}
