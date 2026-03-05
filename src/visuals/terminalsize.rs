// Terminal size detection for zfetch.
// Uses libc ioctl syscall to query terminal dimensions.

use std::os::unix::io::AsRawFd;
use libc;

#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

// TIOCGWINSZ constant for Linux
const TIOCGWINSZ: libc::Ioctl = 0x5413 as libc::Ioctl;

// Get the terminal size as, columns and rows
// Returns None if the terminal size cannot be determined.
pub fn get_terminal_size() -> Option<(u16, u16)> {
    use std::io::stdout;

    unsafe {
        let mut ws = std::mem::MaybeUninit::<Winsize>::zeroed();
        let fd = stdout().as_raw_fd();
                #[cfg(target_os = "linux")]
                {
        let result = libc::ioctl(fd, TIOCGWINSZ, ws.as_mut_ptr());
            if result == 0 {
                let ws = ws.assume_init();
                if ws.ws_col > 0 && ws.ws_row > 0 {
                    return Some((ws.ws_col, ws.ws_row));
                }
            }
        }
    }

    // Fallback to environment variables
    get_size_from_env()
}

fn get_size_from_env() -> Option<(u16, u16)> {
    let cols = std::env::var("COLUMNS").ok()?.parse().ok()?;
    let rows = std::env::var("LINES").ok()?.parse().ok()?;
    Some((cols, rows))
}