use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PICO8_PALETTE: [&str; 16] = [
    "#000000", // 0: Black
    "#1D2B53", // 1: Dark Blue
    "#7E2553", // 2: Dark Purple
    "#008751", // 3: Dark Green
    "#AB5236", // 4: Brown
    "#5F574F", // 5: Dark Grey
    "#C2C3C7", // 6: Light Grey
    "#FFF1E8", // 7: White
    "#FF004D", // 8: Red
    "#FFA300", // 9: Orange
    "#FFEC27", // 10: Yellow
    "#00E436", // 11: Green
    "#29ADFF", // 12: Blue
    "#83769C", // 13: Lavender
    "#FF77A8", // 14: Pink
    "#FFCCAA", // 15: Peach
];

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CanvaPixel {
    pub x: u32,
    pub y: u32,
    pub color: u8,
}

impl CanvaPixel {
    pub fn validate(
        &self,
        canvas_size: u32,
        resize_history: &[(u32, i64)],
        timestamp: i64,
    ) -> Result<(), String> {
        if self.color > 15 {
            return Err(format!(
                "Invalid color index: {} (must be 0-15)",
                self.color
            ));
        }

        if self.x >= canvas_size || self.y >= canvas_size {
            return Err(format!(
                "Coordinates ({}, {}) out of bounds for canvas size {}",
                self.x, self.y, canvas_size
            ));
        }

        // Anti-cheat: ensure pixel wasn't pre-placed before the canvas expanded to include it
        let required_size = self.x.max(self.y) + 1;
        for &(size, activated_at) in resize_history {
            if size >= required_size {
                if timestamp < activated_at {
                    return Err(format!(
                        "Pixel at ({}, {}) placed before canvas expanded to include it",
                        self.x, self.y
                    ));
                }
                return Ok(());
            }
        }

        Err(format!(
            "No canvas size found that includes ({}, {})",
            self.x, self.y
        ))
    }
}

/// Parse a timestamp ID back into unix microseconds.
/// Uses the same Crockford Base32 encoding as the frontend:
/// 64-bit integer → 13 chars, each representing 5 bits from MSB to LSB.
pub fn parse_timestamp_id(id: &str) -> Result<i64, String> {
    if id.len() != 13 {
        return Err(format!(
            "Invalid ID length: {} (expected 13)",
            id.len()
        ));
    }

    let mut value: u64 = 0;
    for c in id.chars() {
        let digit = crockford_char_value(c)
            .ok_or_else(|| format!("Invalid Crockford Base32 character: {c}"))?;
        value = (value << 5) | digit as u64;
    }

    Ok(value as i64)
}

fn crockford_char_value(c: char) -> Option<u8> {
    match c.to_ascii_uppercase() {
        '0' | 'O' => Some(0),
        '1' | 'I' | 'L' => Some(1),
        '2' => Some(2),
        '3' => Some(3),
        '4' => Some(4),
        '5' => Some(5),
        '6' => Some(6),
        '7' => Some(7),
        '8' => Some(8),
        '9' => Some(9),
        'A' => Some(10),
        'B' => Some(11),
        'C' => Some(12),
        'D' => Some(13),
        'E' => Some(14),
        'F' => Some(15),
        'G' => Some(16),
        'H' => Some(17),
        'J' => Some(18),
        'K' => Some(19),
        'M' => Some(20),
        'N' => Some(21),
        'P' => Some(22),
        'Q' => Some(23),
        'R' => Some(24),
        'S' => Some(25),
        'T' => Some(26),
        'V' => Some(27),
        'W' => Some(28),
        'X' => Some(29),
        'Y' => Some(30),
        'Z' => Some(31),
        _ => None,
    }
}

pub fn timestamp_micros() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64
}

/// Validate that a timestamp is not too far in the future (2 minute tolerance)
pub fn validate_timestamp(timestamp: i64) -> Result<(), String> {
    let now = timestamp_micros();
    let max_future = now + 2 * 60 * 1_000_000; // 2 minutes

    if timestamp > max_future {
        return Err("Timestamp is too far in the future".into());
    }

    // Must be after Oct 1, 2024
    let oct_first_2024 = 1727740800000000_i64;
    if timestamp < oct_first_2024 {
        return Err("Timestamp is before October 1st, 2024".into());
    }

    Ok(())
}
