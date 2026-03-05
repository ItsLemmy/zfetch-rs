use crate::helpers::read_first_line;

const UNKNOWN: &str = "unknown";

// Get the kernel version
pub fn kernel() -> String {
    #[cfg(target_os = "linux")]
    {
        read_first_line("/proc/sys/kernel/osrelease")
            .map(|s| s.split('-').next().unwrap_or(&s).to_string())
            .unwrap_or_else(|| UNKNOWN.to_string())
    }

    #[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly"))]
    {
        use std::ffi::CStr;
        use std::mem::MaybeUninit;
        let mut info: MaybeUninit<libc::utsname> = MaybeUninit::uninit();
        if unsafe { libc::uname(info.as_mut_ptr()) } == 0 {
            let info = unsafe { info.assume_init() };
            let release = unsafe { CStr::from_ptr(info.release.as_ptr()) };
            if let Ok(s) = release.to_str() {
                return s.to_string();
            }
        }
        UNKNOWN.to_string()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly")))]
    {
        "unsupported platform".to_string()
    }
}