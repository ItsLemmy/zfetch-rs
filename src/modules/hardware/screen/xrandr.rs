use std::process::Command;

use memchr::memchr_iter;
use memchr::memmem;

use super::{format_display_str, format_screens};

// Parse xrandr --current output
pub fn screen_from_xrandr() -> Option<Vec<(String, String)>> {
    let output = Command::new("xrandr")
        .arg("--current")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = &output.stdout;
    let mut screens: Vec<(bool, String)> = Vec::new();
    let mut current_is_primary = false;
    let mut current_is_portrait = false;

    let mut start = 0;
    for end in memchr_iter(b'\n', stdout) {
        let line = &stdout[start..end];
        start = end + 1;

        if memmem::find(line, b" connected").is_some() {
            current_is_primary = memmem::find(line, b" primary ").is_some();
            let before_paren = memchr::memchr(b'(', line)
                .map(|p| &line[..p])
                .unwrap_or(line);
            current_is_portrait = memmem::find(before_paren, b" left").is_some()
                || memmem::find(before_paren, b" right").is_some();
        } else if memchr::memchr(b'*', line).is_some() {
            let Ok(line_str) = std::str::from_utf8(line) else {
                continue;
            };
            let parts: Vec<&str> = line_str.split_whitespace().collect();
            if parts.len() >= 2 {
                let res = parts[0];
                let rate: String = parts[1]
                    .chars()
                    .filter(|c| c.is_ascii_digit() || *c == '.')
                    .collect();

                let rate_u = rate.parse::<f64>().map(|f| f.round() as u64).unwrap_or(0);
                let display_str = format_display_str(current_is_portrait, res, rate_u);
                screens.push((current_is_primary, display_str));
            }
        }
    }

    if screens.is_empty() {
        return None;
    }

    Some(format_screens(screens))
}