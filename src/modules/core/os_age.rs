use crate::cache;

const UNKNOWN: &str = "unknown";

// Get the OS installation age by checking the root filesystem birth time.
// Uses persistent cache for the birth timestamp, then calculates age from that.
pub fn os_age() -> String {
    use std::time::SystemTime;

    // Check cache first for the birth timestamp
    if let Some(birth_time) = cache::get_cached_os_birth() {
        if let Ok(now) = SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            let age_secs = now.as_secs().saturating_sub(birth_time);
            return format_age(age_secs);
        }
    }

    // No cache hit, fetch fresh value
    let birth_time = os_birth_fresh();

    if let Some(birth) = birth_time {
        // Cache the birth timestamp for next time
        cache::cache_os_birth(birth);

        if let Ok(now) = SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            let age_secs = now.as_secs().saturating_sub(birth);
            return format_age(age_secs);
        }
    }

    UNKNOWN.to_string()
}

// Fetch OS birth timestamp fresh (no cache)
fn os_birth_fresh() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        use std::ffi::CString;
        use std::mem::MaybeUninit;

        #[repr(C)]
        struct StatxTimestamp {
            tv_sec: i64,
            tv_nsec: u32,
            _pad: i32,
        }

        #[repr(C)]
        struct Statx {
            stx_mask: u32,
            stx_blksize: u32,
            stx_attributes: u64,
            stx_nlink: u32,
            stx_uid: u32,
            stx_gid: u32,
            stx_mode: u16,
            _pad1: u16,
            stx_ino: u64,
            stx_size: u64,
            stx_blocks: u64,
            stx_attributes_mask: u64,
            stx_atime: StatxTimestamp,
            stx_btime: StatxTimestamp,
            stx_ctime: StatxTimestamp,
            stx_mtime: StatxTimestamp,
            stx_rdev_major: u32,
            stx_rdev_minor: u32,
            stx_dev_major: u32,
            stx_dev_minor: u32,
            stx_mnt_id: u64,
            _spare: [u64; 9],
        }

        const AT_FDCWD: i32 = -100;
        const AT_NO_AUTOMOUNT: i32 = 0x800;
        const AT_SYMLINK_NOFOLLOW: i32 = 0x100;
        const STATX_BTIME: u32 = 0x800;

        #[cfg(target_arch = "x86_64")]
        const SYS_STATX: i64 = 332;
        #[cfg(target_arch = "aarch64")]
        const SYS_STATX: i64 = 291;
        #[cfg(target_arch = "arm")]
        const SYS_STATX: i64 = 397;
        #[cfg(target_arch = "riscv64")]
        const SYS_STATX: i64 = 291;
        #[cfg(target_arch = "loongarch64")]
        const SYS_STATX: i64 = 291;
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "arm",
            target_arch = "riscv64",
            target_arch = "loongarch64",
        )))]
        const SYS_STATX: i64 = 332;

        let path = CString::new("/").ok()?;
        let mut statx_buf: MaybeUninit<Statx> = MaybeUninit::uninit();

        let ret = unsafe {
            libc::syscall(
                SYS_STATX,
                AT_FDCWD,
                path.as_ptr(),
                AT_NO_AUTOMOUNT | AT_SYMLINK_NOFOLLOW,
                STATX_BTIME,
                statx_buf.as_mut_ptr(),
            )
        };

        if ret != 0 {
            return None;
        }

        let statx = unsafe { statx_buf.assume_init() };

        if statx.stx_mask & STATX_BTIME == 0 {
            return None;
        }

        let birth_secs = statx.stx_btime.tv_sec;
        if birth_secs < 0 {
            return None;
        }

        return Some(birth_secs as u64);
    }

    #[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly"))]
    {
        use std::ffi::CString;
        use std::mem::MaybeUninit;

        let path = CString::new("/").ok()?;
        let mut stat_buf: MaybeUninit<libc::stat> = MaybeUninit::uninit();

        let ret = unsafe { libc::stat(path.as_ptr(), stat_buf.as_mut_ptr()) };
        if ret != 0 {
            return None;
        }

        let stat = unsafe { stat_buf.assume_init() };
        let birth = stat.st_birthtime;
        if birth <= 0 {
            return None;
        }

        return Some(birth as u64);
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly")))]
    {
        None
    }
}

// Format age in seconds to human-readable string
fn format_age(seconds: u64) -> String {
    let days = seconds / 86400;
    let years = days / 365;
    let remaining_days = days % 365;
    let months = remaining_days / 30;

    if years > 0 {
        if months > 0 {
            format!("{}y {}mo", years, months)
        } else {
            format!("{}y", years)
        }
    } else if months > 0 {
        format!("{}mo", months)
    } else if days > 0 {
        format!("{}d", days)
    } else {
        "< 1d".to_string()
    }
}