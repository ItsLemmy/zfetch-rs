use std::fs;
use std::fs::File;
use std::os::unix::io::AsRawFd;

use super::{format_display_str, format_screens};

// DRM ioctl structures for getting connector mode info and rotation
#[repr(C)]
struct DrmModeGetConnector {
    encoders_ptr: u64,
    modes_ptr: u64,
    props_ptr: u64,
    prop_values_ptr: u64,
    count_modes: u32,
    count_props: u32,
    count_encoders: u32,
    encoder_id: u32,
    connector_id: u32,
    connector_type: u32,
    connector_type_id: u32,
    connection: u32,
    mm_width: u32,
    mm_height: u32,
    subpixel: u32,
    _pad: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct DrmModeModeinfo {
    clock: u32,
    hdisplay: u16,
    hsync_start: u16,
    hsync_end: u16,
    htotal: u16,
    hskew: u16,
    vdisplay: u16,
    vsync_start: u16,
    vsync_end: u16,
    vtotal: u16,
    vscan: u16,
    vrefresh: u32,
    flags: u32,
    type_: u32,
    name: [u8; 32],
}

#[repr(C)]
struct DrmModeGetEncoder {
    encoder_id: u32,
    encoder_type: u32,
    crtc_id: u32,
    possible_crtcs: u32,
    possible_clones: u32,
}

#[repr(C)]
struct DrmModeGetCrtc {
    set_connectors_ptr: u64,
    count_connectors: u32,
    crtc_id: u32,
    fb_id: u32,
    x: u32,
    y: u32,
    gamma_size: u32,
    mode_valid: u32,
    mode: DrmModeModeinfo,
}

#[repr(C)]
struct DrmModeGetPlaneRes {
    plane_id_ptr: u64,
    count_planes: u32,
}

#[repr(C)]
struct DrmModeGetPlane {
    plane_id: u32,
    crtc_id: u32,
    fb_id: u32,
    possible_crtcs: u32,
    gamma_size: u32,
    count_format_types: u32,
    format_type_ptr: u64,
}

#[repr(C)]
struct DrmModeObjGetProperties {
    props_ptr: u64,
    prop_values_ptr: u64,
    count_props: u32,
    obj_id: u32,
    obj_type: u32,
}

#[repr(C)]
struct DrmModeGetProperty {
    values_ptr: u64,
    enum_blob_ptr: u64,
    prop_id: u32,
    flags: u32,
    name: [u8; 32],
    count_values: u32,
    count_enum_blobs: u32,
}

// DRM ioctl numbers
const DRM_IOCTL_MODE_GETCONNECTOR: libc::Ioctl = 0xc05064a7u32 as libc::Ioctl;
const DRM_IOCTL_MODE_GETENCODER: libc::Ioctl = 0xc01464a6u32 as libc::Ioctl;
const DRM_IOCTL_MODE_GETCRTC: libc::Ioctl = 0xc06864a1u32 as libc::Ioctl;
const DRM_IOCTL_MODE_GETPLANERESOURCES: libc::Ioctl = 0xc01064b5u32 as libc::Ioctl;
const DRM_IOCTL_MODE_GETPLANE: libc::Ioctl = 0xc02064b6u32 as libc::Ioctl;
const DRM_IOCTL_MODE_OBJ_GETPROPERTIES: libc::Ioctl = 0xc02064b9u32 as libc::Ioctl;
const DRM_IOCTL_MODE_GETPROPERTY: libc::Ioctl = 0xc04064aau32 as libc::Ioctl;
const DRM_IOCTL_SET_CLIENT_CAP: libc::Ioctl = 0x4010640du32 as libc::Ioctl;

// DRM object types
const DRM_MODE_OBJECT_PLANE: u32 = 0xeeeeeeee;

// DRM rotation values (bitmask)
const DRM_MODE_ROTATE_0: u64 = 1 << 0;
const DRM_MODE_ROTATE_90: u64 = 1 << 1;
const DRM_MODE_ROTATE_270: u64 = 1 << 3;

// Get rotation for a CRTC by checking its primary plane's rotation property
fn get_crtc_rotation(fd: i32, crtc_id: u32, plane_ids: &[u32]) -> u64 {
    if plane_ids.is_empty() {
        return DRM_MODE_ROTATE_0;
    }

    // Find the plane associated with this CRTC
    for &plane_id in plane_ids {
        let mut plane = DrmModeGetPlane {
            plane_id,
            crtc_id: 0,
            fb_id: 0,
            possible_crtcs: 0,
            gamma_size: 0,
            count_format_types: 0,
            format_type_ptr: 0,
        };

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETPLANE, &mut plane) } < 0 {
            continue;
        }

        // Check if this plane is attached to our CRTC
        if plane.crtc_id != crtc_id {
            continue;
        }

        // Get plane properties to find rotation
        let mut obj_props = DrmModeObjGetProperties {
            props_ptr: 0,
            prop_values_ptr: 0,
            count_props: 0,
            obj_id: plane_id,
            obj_type: DRM_MODE_OBJECT_PLANE,
        };

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_OBJ_GETPROPERTIES, &mut obj_props) } < 0 {
            continue;
        }

        if obj_props.count_props == 0 {
            continue;
        }

        let mut props: Vec<u32> = vec![0; obj_props.count_props as usize];
        let mut prop_values: Vec<u64> = vec![0; obj_props.count_props as usize];
        obj_props.props_ptr = props.as_mut_ptr() as u64;
        obj_props.prop_values_ptr = prop_values.as_mut_ptr() as u64;

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_OBJ_GETPROPERTIES, &mut obj_props) } < 0 {
            continue;
        }

        // Look for "rotation" property
        for i in 0..obj_props.count_props as usize {
            let mut prop = DrmModeGetProperty {
                values_ptr: 0,
                enum_blob_ptr: 0,
                prop_id: props[i],
                flags: 0,
                name: [0; 32],
                count_values: 0,
                count_enum_blobs: 0,
            };

            if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETPROPERTY, &mut prop) } < 0 {
                continue;
            }

            let name = std::str::from_utf8(&prop.name)
                .unwrap_or("")
                .trim_end_matches('\0');

            if name == "rotation" {
                return prop_values[i];
            }
        }
    }

    DRM_MODE_ROTATE_0
}

// Get screen info using DRM ioctl - much faster than spawning subprocesses
pub fn screen_from_drm() -> Option<Vec<(String, String)>> {
    // On hybrid GPU systems, card0 may be the iGPU with no active displays.
    // Collect all connected entries in one pass, then open the matching card fd.
    let all_entries: Vec<_> = fs::read_dir("/sys/class/drm")
        .ok()?
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let n = name.to_string_lossy();
            // Only connector entries, no Writeback
            if n.contains("Writeback") || !n.contains('-') {
                return false;
            }
            fs::read_to_string(e.path().join("status"))
                .map(|s| s.trim() == "connected")
                .unwrap_or(false)
        })
        .collect();

    if all_entries.is_empty() {
        return None;
    }

    // Derive card number from first connected entry (e.g. "card1-eDP-1" -> "card1")
    let card_prefix = {
        let name = all_entries[0].file_name();
        let n = name.to_string_lossy();
        let dash = n.find('-')?;
        format!("{}-", &n[..dash])
    };
    let card_num: u32 = card_prefix
        .trim_start_matches("card")
        .trim_end_matches('-')
        .parse()
        .ok()?;
    let fd = File::open(format!("/dev/dri/card{}", card_num)).ok()?;
    let raw_fd = fd.as_raw_fd();

    // Enable universal planes once — required for rotation property access
    let cap: [u64; 2] = [2, 1];
    unsafe { libc::ioctl(raw_fd, DRM_IOCTL_SET_CLIENT_CAP, cap.as_ptr()) };

    // Fetch plane IDs once — reused for every connector's rotation check
    let plane_ids: Vec<u32> = {
        let mut res = DrmModeGetPlaneRes { plane_id_ptr: 0, count_planes: 0 };
        let mut ids = Vec::new();
        if unsafe { libc::ioctl(raw_fd, DRM_IOCTL_MODE_GETPLANERESOURCES, &mut res) } >= 0
            && res.count_planes > 0
        {
            ids = vec![0u32; res.count_planes as usize];
            res.plane_id_ptr = ids.as_mut_ptr() as u64;
            if unsafe { libc::ioctl(raw_fd, DRM_IOCTL_MODE_GETPLANERESOURCES, &mut res) } < 0 {
                ids.clear();
            }
        }
        ids
    };

    let mut screens: Vec<(bool, String)> = Vec::new();

    // Reuse already-collected connected entries — no second read_dir
    for entry in &all_entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if !name_str.starts_with(&card_prefix) || name_str.contains("Writeback") {
            continue;
        }

        let path = entry.path();
        let Ok(status) = fs::read_to_string(path.join("status")) else {
            continue;
        };

        if status.trim() != "connected" {
            continue;
        }

        // Get connector_id from sysfs
        let connector_id: u32 = match fs::read_to_string(path.join("connector_id"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
        {
            Some(id) => id,
            None => continue,
        };

        // Get connector info via DRM ioctl
        let mut conn = DrmModeGetConnector {
            encoders_ptr: 0,
            modes_ptr: 0,
            props_ptr: 0,
            prop_values_ptr: 0,
            count_modes: 0,
            count_props: 0,
            count_encoders: 0,
            encoder_id: 0,
            connector_id,
            connector_type: 0,
            connector_type_id: 0,
            connection: 0,
            mm_width: 0,
            mm_height: 0,
            subpixel: 0,
            _pad: 0,
        };

        // First ioctl call to get counts
        if unsafe { libc::ioctl(raw_fd, DRM_IOCTL_MODE_GETCONNECTOR, &mut conn) } < 0 {
            continue;
        }

        if conn.count_modes == 0 {
            continue;
        }

        // Only modes buffer is used — encoder/prop buffers required by kernel but values discarded
        let mut modes: Vec<DrmModeModeinfo> = vec![DrmModeModeinfo::default(); conn.count_modes as usize];
        let mut encoders: Vec<u32> = vec![0u32; conn.count_encoders as usize];
        let mut props: Vec<u32> = vec![0u32; conn.count_props as usize];
        let mut prop_values: Vec<u64> = vec![0u64; conn.count_props as usize];

        conn.modes_ptr = modes.as_mut_ptr() as u64;
        conn.encoders_ptr = encoders.as_mut_ptr() as u64;
        conn.props_ptr = props.as_mut_ptr() as u64;
        conn.prop_values_ptr = prop_values.as_mut_ptr() as u64;

        // Second ioctl call to get actual data
        if unsafe { libc::ioctl(raw_fd, DRM_IOCTL_MODE_GETCONNECTOR, &mut conn) } < 0 {
            continue;
        }

        // Get CRTC ID via encoder to get current mode and rotation
        let mut crtc_id: u32 = 0;
        if conn.encoder_id != 0 {
            let mut encoder = DrmModeGetEncoder {
                encoder_id: conn.encoder_id,
                encoder_type: 0,
                crtc_id: 0,
                possible_crtcs: 0,
                possible_clones: 0,
            };
            if unsafe { libc::ioctl(raw_fd, DRM_IOCTL_MODE_GETENCODER, &mut encoder) } >= 0 {
                crtc_id = encoder.crtc_id;
            }
        }

        // Skip if no CRTC (display not active)
        if crtc_id == 0 {
            continue;
        }

        // Get the current mode from CRTC (the actually active mode, not just preferred)
        let mut crtc = DrmModeGetCrtc {
            set_connectors_ptr: 0,
            count_connectors: 0,
            crtc_id,
            fb_id: 0,
            x: 0,
            y: 0,
            gamma_size: 0,
            mode_valid: 0,
            mode: DrmModeModeinfo::default(),
        };

        if unsafe { libc::ioctl(raw_fd, DRM_IOCTL_MODE_GETCRTC, &mut crtc) } < 0 {
            continue;
        }

        // If CRTC has no valid mode, skip (shouldn't happen for connected displays)
        if crtc.mode_valid == 0 {
            continue;
        }

        let mode = &crtc.mode;

        // Check rotation from CRTC's plane
        let rotation = get_crtc_rotation(raw_fd, crtc_id, &plane_ids);

        let is_primary = name_str.contains("eDP");

        // Determine portrait mode: 90° or 270° rotation, or physical portrait panel
        let is_portrait = (rotation & (DRM_MODE_ROTATE_90 | DRM_MODE_ROTATE_270)) != 0
            || mode.vdisplay > mode.hdisplay;

        // Calculate refresh rate from timing parameters if vrefresh is 0 or missing
        // Formula: refresh = (clock * 1000) / (htotal * vtotal)
        // clock is in kHz, so multiply by 1000 to get Hz
        let refresh = if mode.vrefresh > 0 {
            mode.vrefresh
        } else if mode.htotal > 0 && mode.vtotal > 0 {
            let htotal = mode.htotal as u64;
            let vtotal = mode.vtotal as u64;
            let clock = mode.clock as u64;
            // clock is in kHz, multiply by 1000 to get Hz, then divide by total pixels
            ((clock * 1000) / (htotal * vtotal)) as u32
        } else {
            0
        };

        let res = format!("{}x{}", mode.hdisplay, mode.vdisplay);
        let display_str = format_display_str(is_portrait, &res, refresh as u64);
        screens.push((is_primary, display_str));
    }

    if screens.is_empty() {
        return None;
    }

    Some(format_screens(screens))
}