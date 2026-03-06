use std::fs;

use memchr::memmem;

use crate::cache;
use crate::helpers::read_first_line;

// Get the CPU model name with boost clock.
// Uses persistent cache to avoid repeated /proc reads.
pub fn cpu() -> String {
    // Check cache first (unless --refresh was passed)
    if let Some(cached) = cache::get_cached_cpu() {
        return cached;
    }

    // No cache hit, fetch fresh value
    let result = cpu_fresh();

    // Cache the result for next time
    cache::cache_cpu(&result);

    result
}

// Fetch CPU info fresh (no cache)
// Uses byte-level parsing with memchr for speed
fn cpu_fresh() -> String {
    let model = fs::read("/proc/cpuinfo").ok().and_then(|content| {
        // Find "model name" using SIMD search
        let needle = b"model name";
        let pos = memmem::find(&content, needle)?;
        let after_needle = &content[pos + needle.len()..];

        // Find the ':' separator
        let colon_pos = memchr::memchr(b':', after_needle)?;
        let after_colon = &after_needle[colon_pos + 1..];

        // Find end of line
        let line_end = memchr::memchr(b'\n', after_colon).unwrap_or(after_colon.len());
        let name_bytes = &after_colon[..line_end];

        // Convert to string and process
        let name = std::str::from_utf8(name_bytes).ok()?;
        let words: Vec<&str> = name.split_whitespace().collect();

        // Find where GPU info starts (e.g., "with Radeon Graphics", "w/ Intel UHD")
        let gpu_start = words
            .iter()
            .position(|&w| w.eq_ignore_ascii_case("with") || w.eq_ignore_ascii_case("w/"));
        let words = match gpu_start {
            Some(idx) => &words[..idx],
            None => &words[..],
        };

        Some(
            words
                .iter()
                .map(|w| {
                    w.replace("(R)", "")
                     .replace("(TM)", "")
                     .replace("(C)", "")
                })
                .filter(|w| {
                    !w.ends_with("-Core")
                        && w != "Processor"
                        && w != "AMD"
                        && w != "Intel"
                        && w != "Apple"
                        && !w.is_empty()
                })
                .collect::<Vec<_>>()
                .join(" "),
        )
    });

    let model = match model {
        Some(m) => m,
        None => return "unknown".to_string(),
    };

    // Strip existing @ clock from model name if present (e.g. "i7-10750H @ 2.60GHz")
    let model = if let Some(at_pos) = memchr::memchr(b'@', model.as_bytes()) {
        model[..at_pos].trim_end().to_string()
    } else {
        model
    };

    // Get boost clock from cpufreq (in kHz)
    let boost_clock = read_first_line("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
        .and_then(|khz_str| khz_str.parse::<u64>().ok())
        .map(|khz| {
            let ghz = khz as f64 / 1_000_000.0;
            format!(" @ {:.2}GHz", ghz)
        })
        .unwrap_or_default();

    format!("{}{}", model, boost_clock)
}