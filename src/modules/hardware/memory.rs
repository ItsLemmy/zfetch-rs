use std::fs;

use memchr::memmem;

use crate::helpers::create_bar;

// Get memory usage as a visual bar, 10 blocks = 100% usage
// Uses byte-level parsing with memchr for speed
pub fn memory() -> String {
    let mut total: u64 = 0;
    let mut available: u64 = 0;

    if let Ok(content) = fs::read("/proc/meminfo") {
        // Find MemTotal using SIMD search
        if let Some(pos) = memmem::find(&content, b"MemTotal:") {
            let after = &content[pos + 9..]; // "MemTotal:" is 9 bytes
            // Skip whitespace and find the number
            if let Some(start) = after.iter().position(|&b| b.is_ascii_digit()) {
                let num_start = &after[start..];
                let end = num_start.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(num_start.len());
                if let Ok(s) = std::str::from_utf8(&num_start[..end]) {
                    total = s.parse().unwrap_or(0);
                }
            }
        }

        // Find MemAvailable using SIMD search
        if let Some(pos) = memmem::find(&content, b"MemAvailable:") {
            let after = &content[pos + 13..]; // "MemAvailable:" is 13 bytes
            if let Some(start) = after.iter().position(|&b| b.is_ascii_digit()) {
                let num_start = &after[start..];
                let end = num_start.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(num_start.len());
                if let Ok(s) = std::str::from_utf8(&num_start[..end]) {
                    available = s.parse().unwrap_or(0);
                }
            }
        }
    }

    if total > 0 {
        let used = total - available;
        let usage_percent = (used as f64 / total as f64) * 100.0;
        let bar = create_bar(usage_percent);

        // Convert to GB (decimal: 1 KiB = 1024 bytes, meminfo reports in KiB)
        let used_gb = used as f64 / 1_048_576.0;
        let total_gb = total as f64 / 1_048_576.0;

        return format!(" {} {:.1}GB/{:.0}GB", bar, used_gb, total_gb);
    }
    "unknown".to_string()
}