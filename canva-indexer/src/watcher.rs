use anyhow::{anyhow, Result};
use pubky::Pubky;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::db::{self, Db, PixelState};
use crate::pixel::{self, CanvaPixel};

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type")]
pub enum SseEvent {
    #[serde(rename = "pixel")]
    Pixel(PixelState),
    #[serde(rename = "resize")]
    Resize {
        old_width: u32,
        old_height: u32,
        new_width: u32,
        new_height: u32,
    },
}

pub async fn run(
    db: Db,
    pubky: Arc<Pubky>,
    config: Config,
    sse_tx: broadcast::Sender<SseEvent>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    let poll_interval = std::time::Duration::from_millis(config.watcher.poll_interval_ms);
    let mut interval = tokio::time::interval(poll_interval);

    info!("Watcher started, polling every {}ms", config.watcher.poll_interval_ms);

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                info!("Watcher shutting down");
                break;
            }
            _ = interval.tick() => {
                if let Err(e) = poll_cycle(&db, &pubky, &config, &sse_tx).await {
                    error!("Poll cycle error: {e:?}");
                }
            }
        }
    }
}

async fn poll_cycle(
    db: &Db,
    pubky: &Pubky,
    config: &Config,
    sse_tx: &broadcast::Sender<SseEvent>,
) -> Result<()> {
    // Get all users grouped by homeserver
    let groups = {
        let db = db.clone();
        tokio::task::spawn_blocking(move || db::get_users_by_homeserver(&db))
            .await??
    };

    if groups.is_empty() {
        debug!("No users to poll");
        return Ok(());
    }

    for (hs_pk, users) in &groups {
        if let Err(e) = poll_homeserver(db, pubky, config, sse_tx, hs_pk, users).await {
            warn!("Error polling homeserver {hs_pk}: {e}");
        }
    }

    check_resize(db, config, sse_tx).await?;

    Ok(())
}

/// Poll a homeserver using /events-stream with per-user cursors and path filtering
async fn poll_homeserver(
    db: &Db,
    pubky: &Pubky,
    config: &Config,
    sse_tx: &broadcast::Sender<SseEvent>,
    hs_pk: &str,
    users: &[(String, String)],
) -> Result<()> {
    // Build events-stream URL with user filters and path prefix
    // Format: /events-stream?path=/pub/pubky-canva/pixels/&user=pk1:cursor1&user=pk2:cursor2
    let mut url = format!(
        "https://{}/events-stream?path=/pub/pubky-canva/pixels/",
        hs_pk
    );
    for (user_pk, cursor) in users {
        if cursor.is_empty() {
            url.push_str(&format!("&user={}", user_pk));
        } else {
            url.push_str(&format!("&user={}:{}", user_pk, cursor));
        }
    }

    debug!("Polling events-stream: {url}");

    let response = pubky
        .client()
        .request(pubky::Method::GET, &url)
        .send()
        .await
        .map_err(|e| anyhow!("HTTP error polling {hs_pk}: {e}"))?;

    let text = response.text().await?;
    if text.trim().is_empty() {
        return Ok(());
    }

    // Parse SSE events from the response
    // Format:
    //   event: PUT
    //   data: pubky://user_pk/pub/pubky-canva/pixels/id
    //   data: cursor: 42
    //   data: content_hash: ...
    //   (blank line)
    let events = parse_sse_response(&text);
    debug!("Homeserver {hs_pk}: {} SSE events", events.len());

    for event in &events {
        if event.event_type != "PUT" {
            continue;
        }

        if let Some((user_pk, pixel_id)) = parse_pixel_uri(&event.uri) {
            // Check if this user is one we're tracking
            let is_tracked = users.iter().any(|(pk, _)| pk == user_pk);
            if !is_tracked {
                continue;
            }

            match process_pixel_event(db, pubky, config, sse_tx, user_pk, pixel_id, &event.uri).await {
                Ok(()) => {}
                Err(e) => warn!("Error processing pixel {pixel_id} from {user_pk}: {e}"),
            }

            // Update this user's cursor
            if !event.cursor.is_empty() {
                let db = db.clone();
                let upk = user_pk.to_string();
                let cur = event.cursor.clone();
                tokio::task::spawn_blocking(move || db::update_user_cursor(&db, &upk, &cur))
                    .await??;
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct SseEventParsed {
    event_type: String,
    uri: String,
    cursor: String,
}

/// Parse SSE-format response into structured events
fn parse_sse_response(text: &str) -> Vec<SseEventParsed> {
    let mut events = Vec::new();
    let mut current_type = String::new();
    let mut current_uri = String::new();
    let mut current_cursor = String::new();

    for line in text.lines() {
        if let Some(event_type) = line.strip_prefix("event: ") {
            current_type = event_type.trim().to_string();
        } else if let Some(data) = line.strip_prefix("data: ") {
            let data = data.trim();
            if let Some(cursor) = data.strip_prefix("cursor: ") {
                current_cursor = cursor.to_string();
            } else if data.starts_with("content_hash:") {
                // Skip content_hash lines
            } else if !data.is_empty() {
                current_uri = data.to_string();
            }
        } else if line.is_empty() && !current_type.is_empty() {
            // End of event block
            events.push(SseEventParsed {
                event_type: current_type.clone(),
                uri: current_uri.clone(),
                cursor: current_cursor.clone(),
            });
            current_type.clear();
            current_uri.clear();
            current_cursor.clear();
        }
    }

    // Handle last event if no trailing blank line
    if !current_type.is_empty() && !current_uri.is_empty() {
        events.push(SseEventParsed {
            event_type: current_type,
            uri: current_uri,
            cursor: current_cursor,
        });
    }

    events
}

/// Parse a pubky URI to extract user_pk and pixel_id
/// URI format: pubky://<user_pk>/pub/pubky-canva/pixels/<id>
fn parse_pixel_uri(uri: &str) -> Option<(&str, &str)> {
    let rest = uri.strip_prefix("pubky://")?;
    let (user_pk, path) = rest.split_once('/')?;
    let pixel_id = path.strip_prefix("pub/pubky-canva/pixels/")?;
    if pixel_id.is_empty() {
        return None;
    }
    Some((user_pk, pixel_id))
}

async fn process_pixel_event(
    db: &Db,
    pubky: &Pubky,
    config: &Config,
    sse_tx: &broadcast::Sender<SseEvent>,
    user_pk: &str,
    pixel_id: &str,
    uri: &str,
) -> Result<()> {
    // Check if already processed
    {
        let db = db.clone();
        let id = pixel_id.to_string();
        if tokio::task::spawn_blocking(move || db::pixel_event_exists(&db, &id)).await?? {
            debug!("Pixel event {pixel_id} already processed");
            return Ok(());
        }
    }

    // Parse timestamp from ID
    let timestamp = pixel::parse_timestamp_id(pixel_id)
        .map_err(|e| anyhow!("Invalid pixel ID {pixel_id}: {e}"))?;

    // Validate timestamp
    pixel::validate_timestamp(timestamp)
        .map_err(|e| anyhow!("Invalid timestamp for {pixel_id}: {e}"))?;

    // Fetch pixel data from homeserver
    let response = pubky.public_storage().get(uri).await
        .map_err(|e| anyhow!("Failed to fetch pixel data: {e}"))?;

    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch pixel: HTTP {}", response.status()));
    }

    let blob = response.bytes().await?;
    let pixel: CanvaPixel = serde_json::from_slice(&blob)
        .map_err(|e| anyhow!("Invalid pixel JSON: {e}"))?;

    // Get canvas dimensions and resize history for validation
    let (canvas_width, canvas_height, resize_history) = {
        let db = db.clone();
        tokio::task::spawn_blocking(move || -> Result<(u32, u32, Vec<(u32, u32, i64)>)> {
            let (w, h) = db::get_canvas_dimensions(&db)?;
            let history = db::get_resize_history(&db)?;
            Ok((w, h, history))
        })
        .await??
    };

    // Validate pixel
    pixel
        .validate(canvas_width, canvas_height, &resize_history, timestamp)
        .map_err(|e| anyhow!("Pixel validation failed: {e}"))?;

    // Check credits
    let regen_us = config.canvas.credit_regen_seconds as i64 * 1_000_000;
    let recent_count = {
        let db = db.clone();
        let upk = user_pk.to_string();
        tokio::task::spawn_blocking(move || {
            db::count_recent_placements(&db, &upk, timestamp, regen_us)
        })
        .await??
    };

    if recent_count >= config.canvas.max_credits {
        return Err(anyhow!(
            "User {} has no credits (used {}/{})",
            user_pk,
            recent_count,
            config.canvas.max_credits
        ));
    }

    // Insert pixel
    let (was_new, was_overwritten) = {
        let db = db.clone();
        let id = pixel_id.to_string();
        let upk = user_pk.to_string();
        let px = pixel.clone();
        tokio::task::spawn_blocking(move || {
            db::insert_pixel(&db, &id, &upk, px.x, px.y, px.color, timestamp)
        })
        .await??
    };

    info!(
        "Pixel placed at ({}, {}) color={} by {} (new={}, overwritten={})",
        pixel.x, pixel.y, pixel.color, user_pk, was_new, was_overwritten
    );

    // Broadcast SSE event
    let _ = sse_tx.send(SseEvent::Pixel(PixelState {
        x: pixel.x,
        y: pixel.y,
        color: pixel.color,
        user_pk: user_pk.to_string(),
        placed_at: timestamp,
    }));

    Ok(())
}

async fn check_resize(
    db: &Db,
    _config: &Config,
    sse_tx: &broadcast::Sender<SseEvent>,
) -> Result<()> {
    let (canvas_width, canvas_height, filled, overwritten) = {
        let db = db.clone();
        tokio::task::spawn_blocking(move || -> Result<(u32, u32, u32, u32)> {
            let (w, h) = db::get_canvas_dimensions(&db)?;
            let (filled, overwritten) = db::get_fill_stats(&db)?;
            Ok((w, h, filled, overwritten))
        })
        .await??
    };

    let total_pixels = canvas_width * canvas_height;
    let half_pixels = total_pixels / 2;

    if filled >= total_pixels && overwritten >= half_pixels {
        // Alternate expansion: if square, double width; if wider, double height
        // 16x16 → 32x16 → 32x32 → 64x32 → 64x64 → 128x64 → ...
        let (new_width, new_height) = if canvas_width == canvas_height {
            (canvas_width * 2, canvas_height)
        } else {
            (canvas_width, canvas_height * 2)
        };

        let now = pixel::timestamp_micros();

        info!(
            "Canvas resize triggered! {}x{} -> {}x{} (filled={}, overwritten={}/{})",
            canvas_width, canvas_height, new_width, new_height, filled, overwritten, half_pixels
        );

        let db = db.clone();
        tokio::task::spawn_blocking(move || db::resize_canvas(&db, new_width, new_height, now)).await??;

        let _ = sse_tx.send(SseEvent::Resize {
            old_width: canvas_width,
            old_height: canvas_height,
            new_width,
            new_height,
        });
    }

    Ok(())
}
