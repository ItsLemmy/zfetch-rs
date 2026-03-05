use std::process::Command;

use super::{format_display_str, format_screens};

// Get screen info from GNOME via D-Bus
pub fn screen_from_gnome_dbus() -> Option<Vec<(String, String)>> {
    // Query Mutter's DisplayConfig D-Bus interface for monitor information
    // This works on native GNOME Wayland sessions
    let output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest=org.gnome.Mutter.DisplayConfig",
            "--object-path=/org/gnome/Mutter/DisplayConfig",
            "--method=org.gnome.Mutter.DisplayConfig.GetCurrentState",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut screens: Vec<(bool, String)> = Vec::new();

    // The D-Bus output format for each mode is a tuple:
    // ('2880x1800@120.000', 2880, 1800, 119.999, 2.0, [...], {'is-current': <true>, ...})
    // We look for modes that have 'is-current': <true> in their properties dict

    // Find all occurrences of 'is-current': <true> and extract the mode info before it
    let mut search_start = 0;
    while let Some(current_pos) = stdout[search_start..].find("'is-current': <true>") {
        let abs_pos = search_start + current_pos;

        // Look backwards to find the mode tuple - find the mode string pattern
        // The mode string looks like ('2880x1800@120.000', ...)
        let region_start = abs_pos.saturating_sub(300);
        let region = &stdout[region_start..abs_pos];

        // Find the last mode pattern like '2880x1800@120.000' before this is-current
        // Mode format: 'WIDTHxHEIGHT@RATE'
        if let Some(mode_match) = region.rfind("('") {
            let mode_start = region_start + mode_match + 2;
            if let Some(mode_end) = stdout[mode_start..].find("',") {
                let mode_str = &stdout[mode_start..mode_start + mode_end];
                // Parse 'WIDTHxHEIGHT@RATE' format
                if let Some(at_pos) = mode_str.find('@') {
                    let res = &mode_str[..at_pos];
                    let rate = &mode_str[at_pos + 1..];

                    let after_region = &stdout[abs_pos..(abs_pos + 500).min(stdout.len())];
                    let before_region = &stdout[region_start..abs_pos];

                    // For primary: look for ", true, [(" pattern which indicates primary=true before monitors list
                    let is_primary =
                        before_region.contains(", true, [('") || after_region.contains(", true, [('");

                    // Transform values: 0=normal, 1=90°, 2=180°, 3=270°
                    // Portrait if transform is 1 or 3
                    let is_portrait = before_region.contains(" uint32 1,")
                        || before_region.contains(" uint32 3,")
                        || after_region.contains(" uint32 1,")
                        || after_region.contains(" uint32 3,");

                    if let Ok(rate_f) = rate.parse::<f64>() {
                        let display_str = format_display_str(is_portrait, res, rate_f.round() as u64);
                        screens.push((is_primary, display_str));
                    }
                }
            }
        }

        search_start = abs_pos + 20;
    }

    if screens.is_empty() {
        return None;
    }

    Some(format_screens(screens))
}