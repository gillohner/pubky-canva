use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        Json, Sse,
    },
    routing::{get, put},
    Router,
};
use pubky::{PublicKey, Pubky};
use serde::Serialize;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

use crate::config::Config;
use crate::db::{self, Db};
use crate::pixel::PICO8_PALETTE;
use crate::watcher::SseEvent;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub pubky: Arc<Pubky>,
    pub config: Config,
    pub sse_tx: broadcast::Sender<SseEvent>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/canvas", get(get_canvas))
        .route("/api/canvas/pixel/{x}/{y}", get(get_pixel))
        .route("/api/canvas/meta", get(get_meta))
        .route("/api/canvas/palette", get(get_palette))
        .route("/api/events", get(sse_events))
        .route("/api/ingest/{public_key}", put(ingest_user))
        .route("/api/user/{public_key}/credits", get(get_credits))
        .route("/api/user/{public_key}/profile", get(get_profile))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[derive(Serialize)]
struct CanvasResponse {
    size: u32,
    pixels: Vec<db::PixelState>,
}

async fn get_canvas(State(state): State<AppState>) -> Result<Json<CanvasResponse>, StatusCode> {
    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<CanvasResponse> {
        let size = db::get_canvas_size(&db)?;
        let pixels = db::get_canvas_state(&db)?;
        Ok(CanvasResponse { size, pixels })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|e| {
        error!("get_canvas error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(result))
}

async fn get_pixel(
    State(state): State<AppState>,
    Path((x, y)): Path<(u32, u32)>,
) -> Result<Json<db::PixelInfo>, StatusCode> {
    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || db::get_pixel_info(&db, x, y))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|e| {
            error!("get_pixel error: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match result {
        Some(info) => Ok(Json(info)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Serialize)]
struct MetaResponse {
    size: u32,
    total_pixels: u32,
    filled: u32,
    overwritten: u32,
    max_credits: u32,
    credit_regen_seconds: u64,
}

async fn get_meta(State(state): State<AppState>) -> Result<Json<MetaResponse>, StatusCode> {
    let db = state.db.clone();
    let config = state.config.clone();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<MetaResponse> {
        let size = db::get_canvas_size(&db)?;
        let (filled, overwritten) = db::get_fill_stats(&db)?;
        Ok(MetaResponse {
            size,
            total_pixels: size * size,
            filled,
            overwritten,
            max_credits: config.canvas.max_credits,
            credit_regen_seconds: config.canvas.credit_regen_seconds,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|e| {
        error!("get_meta error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(result))
}

async fn get_palette() -> Json<Vec<&'static str>> {
    Json(PICO8_PALETTE.to_vec())
}

async fn sse_events(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.sse_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(event) => {
            let data = serde_json::to_string(&event).unwrap_or_default();
            let event_type = match &event {
                SseEvent::Pixel(_) => "pixel",
                SseEvent::Resize { .. } => "resize",
            };
            Some(Ok(Event::default().event(event_type).data(data)))
        }
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn ingest_user(
    State(state): State<AppState>,
    Path(public_key): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Check if user already exists
    {
        let db = state.db.clone();
        let pk = public_key.clone();
        let exists = tokio::task::spawn_blocking(move || db::user_exists(&db, &pk))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if exists {
            return Ok(StatusCode::OK);
        }
    }

    // Resolve homeserver via Pkarr/DHT
    let user_pk: PublicKey = public_key
        .parse()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid public key: {e}")))?;

    let hs_url = state
        .pubky
        .get_homeserver_of(&user_pk)
        .await
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("No homeserver found for {public_key}"),
            )
        })?;

    let hs_pk = hs_url.to_string();
    // The homeserver URL might be in format like "https://<pk>/" or just the pk
    // Extract just the host/pk part
    let homeserver_id = extract_homeserver_id(&hs_pk);

    info!("Ingesting user {public_key} on homeserver {homeserver_id}");

    let db = state.db.clone();
    let pk = public_key.clone();
    let hs = homeserver_id.to_string();
    tokio::task::spawn_blocking(move || db::add_user(&db, &pk, &hs))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

fn extract_homeserver_id(url_or_pk: &str) -> &str {
    // If it looks like a URL, extract the host
    if let Some(rest) = url_or_pk.strip_prefix("https://") {
        rest.split('/').next().unwrap_or(url_or_pk)
    } else if let Some(rest) = url_or_pk.strip_prefix("http://") {
        rest.split('/').next().unwrap_or(url_or_pk)
    } else {
        url_or_pk
    }
}

#[derive(Serialize)]
struct CreditsResponse {
    credits: u32,
    max_credits: u32,
    next_credit_in_seconds: Option<u64>,
}

async fn get_credits(
    State(state): State<AppState>,
    Path(public_key): Path<String>,
) -> Result<Json<CreditsResponse>, StatusCode> {
    let db = state.db.clone();
    let config = state.config.clone();
    let pk = public_key.clone();

    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<CreditsResponse> {
        let now = crate::pixel::timestamp_micros();
        let regen_us = config.canvas.credit_regen_seconds as i64 * 1_000_000;
        let recent = db::count_recent_placements(&db, &pk, now, regen_us)?;
        let credits = config.canvas.max_credits.saturating_sub(recent);

        let next_credit_in = if credits < config.canvas.max_credits {
            // Find the oldest placement in the window to know when next credit regens
            let last = db::get_user_last_placement(&db, &pk)?;
            match last {
                Some(_last_placed_at) => {
                    // Find the earliest placement in the regen window
                    let cutoff = now - regen_us;
                    let conn = db.lock().unwrap();
                    let earliest_in_window: Option<i64> = conn
                        .query_row(
                            "SELECT MIN(placed_at) FROM pixel_events WHERE user_pk = ?1 AND placed_at > ?2",
                            rusqlite::params![pk, cutoff],
                            |row| row.get(0),
                        )
                        .ok();
                    drop(conn);

                    match earliest_in_window {
                        Some(earliest) => {
                            let regen_at = earliest + regen_us;
                            let remaining_us = (regen_at - now).max(0);
                            Some((remaining_us / 1_000_000) as u64)
                        }
                        None => None,
                    }
                }
                None => None,
            }
        } else {
            None
        };

        Ok(CreditsResponse {
            credits,
            max_credits: config.canvas.max_credits,
            next_credit_in_seconds: next_credit_in,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|e| {
        error!("get_credits error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(result))
}

async fn get_profile(
    State(state): State<AppState>,
    Path(public_key): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uri = format!("pubky://{}/pub/pubky.app/profile.json", public_key);

    let response = state
        .pubky
        .public_storage()
        .get(&uri)
        .await
        .map_err(|e| {
            error!("Failed to fetch profile for {public_key}: {e}");
            StatusCode::BAD_GATEWAY
        })?;

    if !response.status().is_success() {
        return Err(StatusCode::NOT_FOUND);
    }

    let bytes = response.bytes().await.map_err(|e| {
        error!("Failed to read profile body for {public_key}: {e}");
        StatusCode::BAD_GATEWAY
    })?;

    let profile: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
        error!("Invalid profile JSON for {public_key}: {e}");
        StatusCode::BAD_GATEWAY
    })?;

    Ok(Json(profile))
}
