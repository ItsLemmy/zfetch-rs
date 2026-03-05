use std::fs;

use crate::cache;

// Check if the system is an ostree-based immutable OS
// /run/ostree-booted exists on all ostree-based systems (Silverblue, Kinoite, Bazzite, uBlue, etc.)
#[cfg(target_os = "linux")]
fn is_immutable_os() -> bool {
    std::path::Path::new("/run/ostree-booted").exists()
}

#[cfg(not(target_os = "linux"))]
fn is_immutable_os() -> bool {
    false
}

// Get the OS name from /etc/os-release.
// Uses persistent cache to avoid repeated file reads.
// Note: Caching is disabled for immutable OSes since the OS name changes daily.
pub fn os() -> String {
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly"))]
    {
        if let Some(cached) = cache::get_cached_os() {
            return cached;
        }

        let os_release = fs::read_to_string("/etc/os-release").ok();

        let pretty_name = os_release.as_deref().and_then(|content| {
            content.lines().find_map(|l| {
                l.strip_prefix("PRETTY_NAME=")
                    .map(|v| v.trim_matches(|c| c == '"' || c == '\'').to_string())
            })
        });

        let is_immutable = is_immutable_os();

        let result = pretty_name.unwrap_or_else(|| std::env::consts::OS.to_string());

        if !is_immutable {
            cache::cache_os(&result);
        }

        return result;
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(cached) = cache::get_cached_os() {
            return cached;
        }
        let result = os_fresh();
        cache::cache_os(&result);
        return result;
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly")))]
    {
        "unsupported platform".to_string()
    }
}

// Fetch OS info fresh for macOS only (Linux handled in os() directly)
#[cfg(target_os = "macos")]
fn os_fresh() -> String {
    use std::process::Command;
    if let Ok(output) = Command::new("sw_vers").output() {
        let out = String::from_utf8_lossy(&output.stdout);
        let mut name = "";
        let mut version = "";
        for line in out.lines() {
            if let Some(v) = line.strip_prefix("ProductName:") {
                name = v.trim();
            } else if let Some(v) = line.strip_prefix("ProductVersion:") {
                version = v.trim();
            }
        }
        if !name.is_empty() && !version.is_empty() {
            return format!("{} {}", name, version);
        }
    }
    "macOS".to_string()
}