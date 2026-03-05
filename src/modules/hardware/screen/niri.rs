use std::process::Command;

use memchr::memchr_iter;
use memchr::memmem;

use super::{format_display_str, format_screens};

// Parse niri msg outputs
pub fn screen_from_niri() -> Option<Vec<(String, String)>> {
    let output = Command::new("niri")
        .args(["msg", "outputs"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = &output.stdout;
    let mut screens: Vec<(bool, String)> = Vec::new();
    let mut current_is_portrait = false;
    let mut is_first = true;

    let mut start = 0;
    for end in memchr_iter(b'\n', stdout) {
        let line = &stdout[start..end];
        start = end + 1;

        // Output line starts with "Output " (no leading whitespace)
        if line.starts_with(b"Output ") {
            // First output is treated as primary
            is_first = screens.is_empty();
            current_is_portrait = false;
        }
        // Transform line: "  Transform: 90° counter-clockwise" or "  Transform: normal"
        else if memmem::find(line, b"Transform:").is_some() {
            // Portrait if rotated 90° or 270°
            current_is_portrait = memmem::find(line, b"90").is_some()
                || memmem::find(line, b"270").is_some();
        }
        // Current mode line: "  Current mode: 2560x1440 @ 74.968 Hz"
        else if memmem::find(line, b"Current mode:").is_some() {
            let Ok(line_str) = std::str::from_utf8(line) else {
                continue;
            };
            // Extract resolution and refresh rate
            if let Some(mode_start) = line_str.find("Current mode:") {
                let mode_part = &line_str[mode_start + 13..].trim();
                let parts: Vec<&str> = mode_part.split_whitespace().collect();
                // Expected: ["2560x1440", "@", "74.968", "Hz"]
                if parts.len() >= 3 {
                    let res = parts[0];
                    let rate = parts[2];

                    let rate_u = rate.parse::<f64>().map(|f| f.round() as u64).unwrap_or(0);
                    let display_str = format_display_str(current_is_portrait, res, rate_u);
                    screens.push((is_first, display_str));
                }
            }
        }
    }

    if screens.is_empty() {
        return None;
    }

    Some(format_screens(screens))
}