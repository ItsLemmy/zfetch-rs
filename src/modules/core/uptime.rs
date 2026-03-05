use crate::helpers::read_first_line;

const UNKNOWN: &str = "unknown";

// Get the system uptime
pub fn uptime() -> String {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use std::mem::MaybeUninit;

        let Ok(name) = CString::new("kern.boottime") else {
            return UNKNOWN.to_string();
        };
        let mut boottime: MaybeUninit<libc::timeval> = MaybeUninit::uninit();
        let mut size = std::mem::size_of::<libc::timeval>();

        let ret = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                boottime.as_mut_ptr() as *mut libc::c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };

        if ret == 0 {
            let boottime = unsafe { boottime.assume_init() };
            if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                let s = now.as_secs().saturating_sub(boottime.tv_sec as u64);
                return format_hm(s);
            }
        }
        UNKNOWN.to_string()
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(line) = read_first_line("/proc/uptime") {
            if let Some(seconds_str) = line.split_whitespace().next() {
                if let Ok(s) = seconds_str.split('.').next().unwrap_or("0").parse::<u64>() {
                    return format_hm(s);
                }
            }
        }
        UNKNOWN.to_string()
    }

    #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly"))]
    {
        use std::mem::MaybeUninit;

        // kern.boottime MIB: [CTL_KERN, KERN_BOOTTIME]
        let mib: [libc::c_int; 2] = [libc::CTL_KERN, libc::KERN_BOOTTIME];
        let mut boottime: MaybeUninit<libc::timeval> = MaybeUninit::uninit();
        let mut size = std::mem::size_of::<libc::timeval>();

        let ret = unsafe {
            libc::sysctl(
                mib.as_ptr(),
                2,
                boottime.as_mut_ptr() as *mut libc::c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };

        if ret == 0 {
            let boottime = unsafe { boottime.assume_init() };
            if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                let s = now.as_secs().saturating_sub(boottime.tv_sec as u64);
                return format_hm(s);
            }
        }
        UNKNOWN.to_string()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly")))]
    {
        "unsupported platform".to_string()
    }
}

// Format seconds into hours/minutes string
fn format_hm(seconds: u64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    }
}