use anyhow::{ensure, Result};
use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use subtle::ConstantTimeEq;
use work_review_core::{database::Store, model::*};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub agent_token: String,
    pub view_token: String,
}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:47831".into(),
            agent_token: format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ),
            view_token: format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ),
        }
    }
}
impl ServerConfig {
    pub fn validate(&self) -> Result<()> {
        self.bind.parse::<std::net::SocketAddr>()?;
        ensure!(
            self.agent_token.len() >= 32
                && self.view_token.len() >= 32
                && self.agent_token != self.view_token,
            "use distinct collector/viewer tokens of at least 32 characters"
        );
        Ok(())
    }
}
pub struct Hub {
    pub store: Mutex<Store>,
    pub directory: PathBuf,
    pub config: ServerConfig,
}

#[derive(rust_embed::RustEmbed)]
#[folder = "../../dist/"]
struct Assets;

pub fn router(hub: Arc<Hub>) -> Router {
    let view = Router::new()
        .route("/api/devices", get(devices))
        .route("/api/overview", get(overview))
        .route("/api/activities", get(activities))
        .route("/api/report", get(report))
        .route("/api/screenshots/{device}/{id}", get(screenshot))
        .route("/api/activities/{device}/{id}/note", put(note))
        .route_layer(middleware::from_fn_with_state(hub.clone(), viewer_auth));
    let ingest = Router::new()
        .route("/api/ingest", post(ingest))
        .route_layer(middleware::from_fn_with_state(hub.clone(), agent_auth));
    view.merge(ingest)
        .route(
            "/api/health",
            get(|| async { Json(serde_json::json!({"ok":true})) }),
        )
        .fallback(asset)
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
        .layer(middleware::from_fn(response_headers))
        .with_state(hub)
}
fn authorized(request: &Request, token: &str) -> bool {
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .is_some_and(|provided| bool::from(provided.as_bytes().ct_eq(token.as_bytes())))
}
async fn viewer_auth(State(hub): State<Arc<Hub>>, request: Request, next: Next) -> Response {
    if authorized(&request, &hub.config.view_token) {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"查看密钥无效"})),
        )
            .into_response()
    }
}
async fn agent_auth(State(hub): State<Arc<Hub>>, request: Request, next: Next) -> Response {
    if authorized(&request, &hub.config.agent_token) {
        next.run(request).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
async fn response_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("referrer-policy", "no-referrer".parse().unwrap());
    headers.insert("content-security-policy","default-src 'self'; img-src 'self' blob:; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'".parse().unwrap());
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
}

struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({"error":self.1}))).into_response()
    }
}
impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        log::error!("hub operation failed: {error:#}");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "服务处理失败，请查看服务日志".into(),
        )
    }
}
type ApiResult<T> = std::result::Result<T, ApiError>;
fn bad(error: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, error.to_string())
}
fn uuid(value: &str) -> ApiResult<()> {
    let parsed = uuid::Uuid::parse_str(value).map_err(bad)?;
    if parsed.to_string() != value {
        return Err(bad("use canonical UUID"));
    }
    Ok(())
}

async fn ingest(
    State(hub): State<Arc<Hub>>,
    Json(batch): Json<Batch>,
) -> ApiResult<Json<Acknowledgment>> {
    uuid(&batch.device.id)?;
    if batch.device.name.trim().is_empty()
        || batch.device.name.len() > 256
        || !matches!(
            batch.device.platform.as_str(),
            "windows" | "macos" | "linux"
        )
        || batch.activities.len() > 16
        || batch
            .status
            .last_error
            .as_ref()
            .is_some_and(|s| s.len() > 4096)
        || batch.status.state.len() > 64
        || batch.status.version.len() > 64
    {
        return Err(bad("invalid batch"));
    }
    let mut decoded = Vec::new();
    // Validate every item before writing any screenshot or committing any record.
    for upload in &batch.activities {
        uuid(&upload.activity.id)?;
        upload.activity.validate(&batch.device.id).map_err(bad)?;
        if upload.activity.screenshot != upload.screenshot_base64.is_some() {
            return Err(bad("screenshot missing"));
        }
        let bytes = upload
            .screenshot_base64
            .as_ref()
            .map(|s| base64::engine::general_purpose::STANDARD.decode(s))
            .transpose()
            .map_err(bad)?;
        if let Some(bytes) = &bytes {
            if bytes.len() > 8 * 1024 * 1024
                || !(bytes.starts_with(&[0xff, 0xd8, 0xff])
                    || bytes.starts_with(b"\x89PNG\r\n\x1a\n"))
            {
                return Err(bad("invalid screenshot"));
            }
        }
        decoded.push(bytes);
    }
    let directory = hub.directory.clone();
    let ack = tokio::task::spawn_blocking(move || -> Result<Acknowledgment> {
        let mut store = hub
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut paths = Vec::new();
        for (upload, bytes) in batch.activities.iter().zip(decoded) {
            let path = if let Some(bytes) = bytes {
                let directory = directory.join("images").join(&batch.device.id);
                std::fs::create_dir_all(&directory)?;
                let path = directory.join(format!("{}.jpg", upload.activity.id));
                if !path.exists() {
                    let temporary = directory.join(format!("{}.tmp", upload.activity.id));
                    std::fs::write(&temporary, bytes)?;
                    std::fs::rename(&temporary, &path)?;
                }
                Some(path)
            } else {
                None
            };
            paths.push(path);
        }
        store.ingest(&batch, &paths, chrono::Utc::now().timestamp())
    })
    .await
    .map_err(|e| ApiError::from(anyhow::anyhow!(e)))??;
    Ok(Json(ack))
}

async fn database<T: Send + 'static>(
    hub: Arc<Hub>,
    operation: impl FnOnce(&Store) -> Result<T> + Send + 'static,
) -> ApiResult<T> {
    tokio::task::spawn_blocking(move || {
        let store = hub
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        operation(&store)
    })
    .await
    .map_err(|e| ApiError::from(anyhow::anyhow!(e)))?
    .map_err(Into::into)
}
async fn devices(State(hub): State<Arc<Hub>>) -> ApiResult<Json<Vec<DeviceView>>> {
    Ok(Json(database(hub, |s| s.devices()).await?))
}
async fn overview(
    State(hub): State<Arc<Hub>>,
    Query(query): Query<RangeQuery>,
) -> ApiResult<Json<Overview>> {
    query.validate().map_err(bad)?;
    Ok(Json(database(hub, move |s| s.overview(&query)).await?))
}
async fn activities(
    State(hub): State<Arc<Hub>>,
    Query(query): Query<RangeQuery>,
) -> ApiResult<Json<ActivityPage>> {
    query.validate().map_err(bad)?;
    Ok(Json(database(hub, move |s| s.activities(&query)).await?))
}
async fn screenshot(
    State(hub): State<Arc<Hub>>,
    Path((device, id)): Path<(String, String)>,
) -> ApiResult<Response> {
    uuid(&device)?;
    uuid(&id)?;
    let path = database(hub, move |s| s.screenshot_path(&device, &id))
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "截图不存在".into()))?;
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|_| ApiError(StatusCode::NOT_FOUND, "截图文件不存在".into()))?;
    let mime = if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else {
        "image/jpeg"
    };
    Ok(([(header::CONTENT_TYPE, mime)], Bytes::from(bytes)).into_response())
}
#[derive(Deserialize)]
struct Note {
    note: String,
}
async fn note(
    State(hub): State<Arc<Hub>>,
    Path((device, id)): Path<(String, String)>,
    Json(note): Json<Note>,
) -> ApiResult<StatusCode> {
    uuid(&device)?;
    uuid(&id)?;
    if note.note.len() > 16384 {
        return Err(bad("备注过长"));
    }
    if database(hub, move |s| s.set_note(&device, &id, &note.note)).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError(StatusCode::NOT_FOUND, "记录不存在".into()))
    }
}
async fn report(
    State(hub): State<Arc<Hub>>,
    Query(query): Query<RangeQuery>,
) -> ApiResult<Response> {
    query.validate().map_err(bad)?;
    let overview = database(hub, move |s| s.overview(&query)).await?;
    let mut markdown=format!("# 工作记录\n\n活跃时间：{} 分钟\n设备合计：{} 分钟\n记录数：{}\n\n| 应用 | 分钟 |\n| --- | ---: |\n",overview.active_seconds/60,overview.device_seconds/60,overview.count);
    for app in overview.apps {
        markdown.push_str(&format!(
            "| {} | {} |\n",
            app.app_name.replace('|', "\\|").replace(['\n', '\r'], " "),
            app.duration / 60
        ));
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=work-review.md",
            ),
        ],
        markdown,
    )
        .into_response())
}
async fn asset(request: Request) -> Response {
    if request.method() != axum::http::Method::GET && request.method() != axum::http::Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let path = request.uri().path().trim_start_matches('/');
    if path.starts_with("api/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let path = if path.is_empty() { "index.html" } else { path };
    if let Some(asset) = Assets::get(path) {
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        (
            [(header::CONTENT_TYPE, mime)],
            Body::from(asset.data.into_owned()),
        )
            .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
