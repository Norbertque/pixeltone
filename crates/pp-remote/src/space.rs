//! Free disk space, so a jam's phone tracks can't fill the PC's disk.

use std::io;
use std::path::Path;

/// Bytes this user may still write on the disk holding `dir` (which must exist).
pub fn available(dir: &Path) -> io::Result<u64> {
    imp::available(dir)
}

#[cfg(unix)]
mod imp {
    use std::ffi::CString;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    pub fn available(dir: &Path) -> io::Result<u64> {
        let path = CString::new(dir.as_os_str().as_bytes())?;
        let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        // SAFETY: `path` is a NUL-terminated string and `stats` has room for the answer.
        if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: statvfs succeeded, so it filled `stats` in.
        let stats = unsafe { stats.assume_init() };
        #[allow(clippy::unnecessary_cast)] // the field types differ between systems
        Ok(stats.f_bavail as u64 * stats.f_frsize as u64)
    }
}

#[cfg(windows)]
mod imp {
    use std::io;
    use std::path::Path;

    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows::core::HSTRING;

    pub fn available(dir: &Path) -> io::Result<u64> {
        let mut free = 0u64;
        // SAFETY: a valid path string and a place for the one number asked for.
        unsafe { GetDiskFreeSpaceExW(&HSTRING::from(dir), Some(&raw mut free), None, None) }
            .map_err(io::Error::other)?;
        Ok(free)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_disk_has_some_space() {
        let free = super::available(&std::env::temp_dir()).unwrap();
        assert!(free > 0);
    }
}
