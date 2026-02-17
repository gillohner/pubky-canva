use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};


pub type Db = Arc<Mutex<Connection>>;

pub fn open(path: &str) -> Result<Db> {
    let conn = Connection::open(path).context("Failed to open SQLite database")?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS users (
            public_key TEXT PRIMARY KEY,
            homeserver_pk TEXT NOT NULL,
            cursor TEXT NOT NULL DEFAULT '',
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS pixel_events (
            id TEXT PRIMARY KEY,
            user_pk TEXT NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            color INTEGER NOT NULL,
            placed_at INTEGER NOT NULL,
            FOREIGN KEY (user_pk) REFERENCES users(public_key)
        );
        CREATE INDEX IF NOT EXISTS idx_pixel_events_user_placed
            ON pixel_events(user_pk, placed_at);

        CREATE TABLE IF NOT EXISTS canvas_state (
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            color INTEGER NOT NULL,
            user_pk TEXT NOT NULL,
            first_user_pk TEXT NOT NULL,
            placed_at INTEGER NOT NULL,
            was_overwritten INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (x, y)
        );

        CREATE TABLE IF NOT EXISTS canvas_resizes (
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            activated_at INTEGER NOT NULL
        );
        ",
    )?;

    // Seed initial canvas size if no resizes exist
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM canvas_resizes",
        [],
        |row| row.get(0),
    )?;
    if count == 0 {
        // Will be set properly by main.rs based on config
        // Default to 16x16 here as fallback
        conn.execute(
            "INSERT INTO canvas_resizes (width, height, activated_at) VALUES (?1, ?2, 0)",
            params![16, 16],
        )?;
    }

    Ok(Arc::new(Mutex::new(conn)))
}

pub fn set_initial_size(db: &Db, size: u32) -> Result<()> {
    let conn = db.lock().unwrap();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM canvas_resizes",
        [],
        |row| row.get(0),
    )?;
    if count == 1 {
        conn.execute(
            "UPDATE canvas_resizes SET width = ?1, height = ?2 WHERE activated_at = 0",
            params![size, size],
        )?;
    }
    Ok(())
}

/// Get current canvas dimensions (width, height) from latest resize
pub fn get_canvas_dimensions(db: &Db) -> Result<(u32, u32)> {
    let conn = db.lock().unwrap();
    let dims: (u32, u32) = conn.query_row(
        "SELECT width, height FROM canvas_resizes ORDER BY activated_at DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(dims)
}

/// Get resize history ordered by activated_at ascending (for validation)
pub fn get_resize_history(db: &Db) -> Result<Vec<(u32, u32, i64)>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT width, height, activated_at FROM canvas_resizes ORDER BY activated_at ASC",
    )?;
    let rows = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Check if a user exists
pub fn user_exists(db: &Db, public_key: &str) -> Result<bool> {
    let conn = db.lock().unwrap();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM users WHERE public_key = ?1",
        params![public_key],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Add a new user
pub fn add_user(db: &Db, public_key: &str, homeserver_pk: &str) -> Result<()> {
    let conn = db.lock().unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        "INSERT OR IGNORE INTO users (public_key, homeserver_pk, created_at) VALUES (?1, ?2, ?3)",
        params![public_key, homeserver_pk, now],
    )?;

    Ok(())
}

/// Get all users grouped by homeserver: (homeserver_pk, [(user_pk, cursor)])
pub fn get_users_by_homeserver(db: &Db) -> Result<Vec<(String, Vec<(String, String)>)>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT public_key, homeserver_pk, cursor FROM users ORDER BY homeserver_pk",
    )?;
    let rows: Vec<(String, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // Group by homeserver
    let mut groups: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();
    for (user_pk, hs_pk, cursor) in rows {
        groups.entry(hs_pk).or_default().push((user_pk, cursor));
    }
    Ok(groups.into_iter().collect())
}

/// Update a user's event cursor
pub fn update_user_cursor(db: &Db, user_pk: &str, cursor: &str) -> Result<()> {
    let conn = db.lock().unwrap();
    conn.execute(
        "UPDATE users SET cursor = ?1 WHERE public_key = ?2",
        params![cursor, user_pk],
    )?;
    Ok(())
}

/// Count recent placements for credit calculation.
/// Returns how many pixels the user placed within the regen window before `timestamp`.
pub fn count_recent_placements(
    db: &Db,
    user_pk: &str,
    timestamp: i64,
    regen_us: i64,
) -> Result<u32> {
    let conn = db.lock().unwrap();
    let cutoff = timestamp - regen_us;
    let count: u32 = conn.query_row(
        "SELECT COUNT(*) FROM pixel_events WHERE user_pk = ?1 AND placed_at > ?2 AND placed_at <= ?3",
        params![user_pk, cutoff, timestamp],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Check if a pixel event ID already exists
pub fn pixel_event_exists(db: &Db, id: &str) -> Result<bool> {
    let conn = db.lock().unwrap();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pixel_events WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Insert a valid pixel event and update canvas state.
/// Returns (was_new_cell, was_overwritten_by_different_user)
pub fn insert_pixel(
    db: &Db,
    id: &str,
    user_pk: &str,
    x: u32,
    y: u32,
    color: u8,
    placed_at: i64,
) -> Result<(bool, bool)> {
    let conn = db.lock().unwrap();

    // Insert pixel event
    conn.execute(
        "INSERT INTO pixel_events (id, user_pk, x, y, color, placed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, user_pk, x, y, color, placed_at],
    )?;

    // Check existing canvas state
    let existing: Option<(String, i32)> = conn
        .query_row(
            "SELECT first_user_pk, was_overwritten FROM canvas_state WHERE x = ?1 AND y = ?2",
            params![x, y],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    match existing {
        None => {
            // New cell
            conn.execute(
                "INSERT INTO canvas_state (x, y, color, user_pk, first_user_pk, placed_at, was_overwritten) VALUES (?1, ?2, ?3, ?4, ?4, ?5, 0)",
                params![x, y, color, user_pk, placed_at],
            )?;
            Ok((true, false))
        }
        Some((first_user, was_overwritten)) => {
            let newly_overwritten = was_overwritten == 0 && first_user != user_pk;
            let ow_val = if newly_overwritten || was_overwritten != 0 {
                1
            } else {
                0
            };
            conn.execute(
                "UPDATE canvas_state SET color = ?1, user_pk = ?2, placed_at = ?3, was_overwritten = ?4 WHERE x = ?5 AND y = ?6",
                params![color, user_pk, placed_at, ow_val, x, y],
            )?;
            Ok((false, newly_overwritten))
        }
    }
}

/// Get canvas fill stats for resize check
pub fn get_fill_stats(db: &Db) -> Result<(u32, u32)> {
    let conn = db.lock().unwrap();
    let filled: u32 = conn.query_row(
        "SELECT COUNT(*) FROM canvas_state",
        [],
        |row| row.get(0),
    )?;
    let overwritten: u32 = conn.query_row(
        "SELECT COUNT(*) FROM canvas_state WHERE was_overwritten = 1",
        [],
        |row| row.get(0),
    )?;
    Ok((filled, overwritten))
}

/// Perform canvas resize
pub fn resize_canvas(db: &Db, new_width: u32, new_height: u32, activated_at: i64) -> Result<()> {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO canvas_resizes (width, height, activated_at) VALUES (?1, ?2, ?3)",
        params![new_width, new_height, activated_at],
    )?;
    Ok(())
}

/// Get full canvas state for API response
pub fn get_canvas_state(db: &Db) -> Result<Vec<PixelState>> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT x, y, color, user_pk, placed_at FROM canvas_state",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PixelState {
                x: row.get(0)?,
                y: row.get(1)?,
                color: row.get(2)?,
                user_pk: row.get(3)?,
                placed_at: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Get info for a single pixel
pub fn get_pixel_info(db: &Db, x: u32, y: u32) -> Result<Option<PixelInfo>> {
    let conn = db.lock().unwrap();

    let current: Option<PixelState> = conn
        .query_row(
            "SELECT x, y, color, user_pk, placed_at FROM canvas_state WHERE x = ?1 AND y = ?2",
            params![x, y],
            |row| {
                Ok(PixelState {
                    x: row.get(0)?,
                    y: row.get(1)?,
                    color: row.get(2)?,
                    user_pk: row.get(3)?,
                    placed_at: row.get(4)?,
                })
            },
        )
        .optional()?;

    let current = match current {
        Some(c) => c,
        None => return Ok(None),
    };

    let mut stmt = conn.prepare(
        "SELECT id, user_pk, color, placed_at FROM pixel_events WHERE x = ?1 AND y = ?2 ORDER BY placed_at DESC LIMIT 10",
    )?;
    let history = stmt
        .query_map(params![x, y], |row| {
            Ok(PixelHistoryEntry {
                id: row.get(0)?,
                user_pk: row.get(1)?,
                color: row.get(2)?,
                placed_at: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(Some(PixelInfo {
        current,
        history,
    }))
}

/// Get a user's last placement timestamp
pub fn get_user_last_placement(db: &Db, user_pk: &str) -> Result<Option<i64>> {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT placed_at FROM pixel_events WHERE user_pk = ?1 ORDER BY placed_at DESC LIMIT 1",
        params![user_pk],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct PixelState {
    pub x: u32,
    pub y: u32,
    pub color: u8,
    pub user_pk: String,
    pub placed_at: i64,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct PixelInfo {
    pub current: PixelState,
    pub history: Vec<PixelHistoryEntry>,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct PixelHistoryEntry {
    pub id: String,
    pub user_pk: String,
    pub color: u8,
    pub placed_at: i64,
}

