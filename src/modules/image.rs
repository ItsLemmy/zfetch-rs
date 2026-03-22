// Image handling module for zfetch
// Supports Kitty graphics protocol for Ghostty, Kitty, and Konsole terminals
// Fine for now , but needs work (fix in v3)

use std::env;
use std::path::Path;

// Default zfetch image embedded in the binary
const DEFAULT_IMAGE: &[u8] = include_bytes!("../assets/default/zfetch.png");

// Check if terminal requires direct transmission (no file-based Kitty protocol support)
fn needs_direct_transmission() -> bool {
    env::var("KONSOLE_VERSION").is_ok() || env::var("WEZTERM_PANE").is_ok()
}

// Display an image using the Kitty graphics protocol.
// Automatically handles animated GIFs vs static images.
pub fn display_image(path: &Path, box_cols: u16, box_rows: u16) -> Result<String, String> {
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Failed to get current dir: {e}"))?
            .join(path)
    };
//decoding, which is much faster than transmitting raw pixel data. The temp file write is a one-time cost and the file gets reused/overwritten on subsequent runs.

    if !abs_path.exists() {
        return Err(format!("Image file not found: {}", abs_path.display()));
    }

    if is_gif(&abs_path) {
        display_kitty_gif(&abs_path, box_cols, box_rows)
    } else {
        display_kitty_static(&abs_path, box_cols, box_rows)
    }
}

// Display a static image using Kitty protocol
fn display_kitty_static(path: &Path, box_cols: u16, box_rows: u16) -> Result<String, String> {
    // Konsole only supports direct transmission, not file-based
    if needs_direct_transmission() {
        return display_kitty_direct(path, box_cols, box_rows);
    }

    // Use file-based transmission for terminals that support it (faster)
    let action = kitty_image::Action::TransmitAndDisplay(
        kitty_image::ActionTransmission {
            format: kitty_image::Format::Png,
            medium: kitty_image::Medium::File,
            ..Default::default()
        },
        kitty_image::ActionPut {
            columns: box_cols as u32,
            rows: box_rows as u32,
            ..Default::default()
        },
    );

    let mut command = kitty_image::Command::with_payload_from_path(action, path);
    command.quietness = kitty_image::Quietness::SuppressAll;
    Ok(kitty_image::WrappedCommand::new(command).to_string())
}

// Display image using direct transmission for terminals like Konsole
fn display_kitty_direct(path: &Path, box_cols: u16, box_rows: u16) -> Result<String, String> {

    // Load and encode image as PNG
    let img = image::open(path).map_err(|e| format!("Failed to load image: {e}"))?;
    let rgba = img.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());

    let action = kitty_image::Action::TransmitAndDisplay(
        kitty_image::ActionTransmission {
            format: kitty_image::Format::Rgba32,
            medium: kitty_image::Medium::Direct,
            width,
            height,
            ..Default::default()
        },
        kitty_image::ActionPut {
            columns: box_cols as u32,
            rows: box_rows as u32,
            ..Default::default()
        },
    );

    let raw_payload = rgba.into_raw();

    // Send chunked with q=2 on every chunk to suppress terminal responses
    send_kitty_chunked(&action, &raw_payload, box_cols, box_rows)
}

// Send image data in chunks with q=2 (suppress responses) on every chunk.
// The kitty_image crate's send_chunked only sets quietness on the first chunk,
// causing terminals like WezTerm to send responses on continuation chunks.
fn send_kitty_chunked(action: &kitty_image::Action, payload: &[u8], _cols: u16, _rows: u16) -> Result<String, String> {
    use std::io::Write;
    use base64::Engine;

    let mut stdout = std::io::stdout().lock();
    let chunks: Vec<&[u8]> = payload.chunks(3072).collect();
    let total = chunks.len();

    for (i, chunk) in chunks.iter().enumerate() {
        let more = if i + 1 < total { 1 } else { 0 };

        if i == 0 {
            // First chunk: include full action header with q=2
            let mut cmd = kitty_image::Command::new(*action);
            cmd.quietness = kitty_image::Quietness::SuppressAll;
            cmd.m = more == 1;
            cmd.payload = chunk.to_vec().into();
            let wrapped = kitty_image::WrappedCommand::new(cmd);
            write!(stdout, "{wrapped}").map_err(|e| format!("Failed to send image: {e}"))?;
        } else {
            // Continuation chunks: include q=2 to suppress responses
            let encoded = base64::engine::general_purpose::STANDARD.encode(chunk);
            write!(stdout, "\x1b_Gq=2,a={},m={more};{encoded}\x1b\\", action.char())
                .map_err(|e| format!("Failed to send image: {e}"))?;
        }
    }

    let _ = stdout.flush();
    Ok(String::new())
}

// Display an animated GIF using kitten icat
fn display_kitty_gif(path: &Path, box_cols: u16, box_rows: u16) -> Result<String, String> {
    use std::process::{Command, Stdio};

    let (col, row) = get_cursor_position().unwrap_or((1, 1));

    let status = Command::new("kitten")
        .args([
            "icat",
            "--stdin=no",
            "--scale-up",
            &format!("--place={}x{}@{}x{}", box_cols, box_rows, col - 1, row - 1),
            "--loop=-1",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to run kitten icat: {e}"))?;

    if status.success() {
        Ok(String::new())
    } else {
        Err("kitten icat failed".into())
    }
}

// Display the embedded default zfetch image
pub fn display_default_image(box_cols: u16, box_rows: u16) -> Result<String, String> {
    // For Konsole, use direct transmission (it doesn't support file-based)
    if needs_direct_transmission() {
        return display_image_bytes(DEFAULT_IMAGE, box_cols, box_rows);
    }

    // For Kitty/Ghostty, write to cache file and use fast file-based transmission
    let cache_dir = env::var("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".cache"))
                .unwrap_or_else(|_| std::env::temp_dir())
        })
        .join("zfetch");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache dir: {e}"))?;
    let cache_path = cache_dir.join("default.png");
    std::fs::write(&cache_path, DEFAULT_IMAGE)
        .map_err(|e| format!("Failed to write cache image: {e}"))?;

    let action = kitty_image::Action::TransmitAndDisplay(
        kitty_image::ActionTransmission {
            format: kitty_image::Format::Png,
            medium: kitty_image::Medium::File,
            ..Default::default()
        },
        kitty_image::ActionPut {
            columns: box_cols as u32,
            rows: box_rows as u32,
            ..Default::default()
        },
    );

    let mut command = kitty_image::Command::with_payload_from_path(action, &cache_path);
    command.quietness = kitty_image::Quietness::SuppressAll;
    Ok(kitty_image::WrappedCommand::new(command).to_string())
}

// Display an image from raw bytes using Kitty protocol
fn display_image_bytes(data: &[u8], box_cols: u16, box_rows: u16) -> Result<String, String> {

    let img = image::load_from_memory(data).map_err(|e| format!("Failed to load image: {e}"))?;
    let rgba = img.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());

    let action = kitty_image::Action::TransmitAndDisplay(
        kitty_image::ActionTransmission {
            format: kitty_image::Format::Rgba32,
            medium: kitty_image::Medium::Direct,
            width,
            height,
            ..Default::default()
        },
        kitty_image::ActionPut {
            columns: box_cols as u32,
            rows: box_rows as u32,
            ..Default::default()
        },
    );

    let raw_payload = rgba.into_raw();
    send_kitty_chunked(&action, &raw_payload, box_cols, box_rows)?;

    Ok(String::new())
}

fn is_gif(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("gif"))
}

// Query current cursor position using ANSI DSR
fn get_cursor_position() -> Option<(u16, u16)> {
    use std::io::{Read, Write};

    let mut termios: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(0, &mut termios) } != 0 {
        return None;
    }
    let original = termios;

    termios.c_lflag &= !(libc::ICANON | libc::ECHO);
    termios.c_cc[libc::VMIN] = 0;
    termios.c_cc[libc::VTIME] = 1;

    if unsafe { libc::tcsetattr(0, libc::TCSANOW, &termios) } != 0 {
        return None;
    }

    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"\x1b[6n");
    let _ = stdout.flush();

    let mut buf = [0u8; 32];
    let mut len = 0;
    for byte in std::io::stdin().lock().bytes().flatten() {
        buf[len] = byte;
        len += 1;
        if byte == b'R' || len >= 31 {
            break;
        }
    }

    unsafe { libc::tcsetattr(0, libc::TCSANOW, &original) };

    let s = std::str::from_utf8(&buf[..len]).ok()?;
    let coords = s.strip_prefix("\x1b[")?.strip_suffix('R')?;
    let (row, col) = coords.split_once(';')?;
    Some((col.parse().ok()?, row.parse().ok()?))
}
