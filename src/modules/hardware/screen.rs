mod drm;
mod gnome;
mod niri;
mod xrandr;

use crate::helpers::get_cached_is_nerd_font;

// Get screen resolution and refresh rate
// Returns a Vec of (key, value) pairs for each monitor, primary first
// Tries DRM ioctl first (fastest), then xrandr, then Wayland-specific methods
pub fn screen() -> Vec<(String, String)> {
    // Try DRM ioctl first - fastest method (~200µs vs ~5ms for subprocess)
    if let Some(screens) = drm::screen_from_drm() {
        return screens;
    }

    // Try xrandr (works on X11 and XWayland)
    if let Some(screens) = xrandr::screen_from_xrandr() {
        return screens;
    }

    // Wayland fallbacks based on compositor/DE
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();

    // Niri compositor
    if desktop == "niri" || std::env::var("NIRI_SOCKET").is_ok() {
        if let Some(screens) = niri::screen_from_niri() {
            return screens;
        }
    }

    // GNOME via D-Bus
    if desktop.contains("gnome") {
        if let Some(screens) = gnome::screen_from_gnome_dbus() {
            return screens;
        }
    }

    vec![]
}

// Shared by all screen backends — builds the display string with icon
pub(super) fn format_display_str(is_portrait: bool, res: &str, refresh: u64) -> String {
    let icon = if is_portrait {
        if get_cached_is_nerd_font() { "󰆡" } else { "Portrait" }
    } else {
        if get_cached_is_nerd_font() { "󰏠" } else { "Landscape" }
    };
    format!("{} {} @ {}Hz", icon, res, refresh)
}

// Format screens list into the output format (primary first, tree-style for multiple)
pub(super) fn format_screens(mut screens: Vec<(bool, String)>) -> Vec<(String, String)> {
    // Sort so primary monitor comes first
    screens.sort_by(|a, b| b.0.cmp(&a.0));

    if screens.len() == 1 {
        return vec![("Display".to_string(), screens[0].1.clone())];
    }

    // Multiple monitors: header line + tree-style entries
    let mut result = vec![("Displays".to_string(), String::new())];
    let last_idx = screens.len() - 1;
    for (i, (_, s)) in screens.iter().enumerate() {
        if i == last_idx {
            result.push(("╰─".to_string(), s.clone()));
        } else {
            result.push(("├─".to_string(), s.clone()));
        }
    }
    result
}