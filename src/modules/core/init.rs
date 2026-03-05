use crate::cache;
use crate::helpers::read_first_line;

const UNKNOWN: &str = "unknown";

// Get the init system (PID 1 process name)
// Uses persistent cache to avoid repeated detection (init system doesn't change)
pub fn init() -> String {
    // Check cache first (unless --refresh was passed)
    if let Some(cached) = cache::get_cached_init() {
        return cached;
    }

    // No cache hit, fetch fresh value
    let result = init_fresh();

    // Cache the result for next time
    cache::cache_init(&result);

    result
}

// Fetch init system info fresh (no cache)
fn init_fresh() -> String {
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly"))]
    {
        use std::path::Path;

        // Check /proc/1/comm first on Linux - fastest method, always correct
        #[cfg(target_os = "linux")]
        {
            if let Some(comm) = read_first_line("/proc/1/comm") {
                let trimmed = comm.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }

        // Fallback: probe filesystem markers for more specific names
        // Check systemd first - most common init on modern Linux
        if Path::new("/run/systemd/system").exists() {
            return "systemd".to_string();
        }

        // OpenRC (used by Gentoo, Alpine, Artix, etc.)
        if Path::new("/etc/openrc").exists() || Path::new("/etc/rc.conf").exists() {
            if Path::new("/etc/runlevels").exists() {
                return "openrc".to_string();
            }
        }

        // runit (used by Void Linux, Artix, etc.)
        if Path::new("/etc/runit").exists() {
            return "runit".to_string();
        }

        // s6 (used by Artix, etc.)
        if Path::new("/etc/s6").exists() || Path::new("/etc/s6-rc").exists() {
            return "s6".to_string();
        }

        // dinit (used by Chimera Linux, Artix, etc.)
        if Path::new("/etc/dinit.d").exists() {
            return "dinit".to_string();
        }

        // SysVinit (traditional init)
        if Path::new("/etc/inittab").exists() {
            return "sysvinit".to_string();
        }

        UNKNOWN.to_string()
    }

    #[cfg(target_os = "macos")]
    {
        "launchd".to_string()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly")))]
    {
        UNKNOWN.to_string()
    }
}