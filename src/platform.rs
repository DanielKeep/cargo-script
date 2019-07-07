/*
Copyright ⓒ 2015-2017 cargo-script contributors.

Licensed under the MIT license (see LICENSE or <http://opensource.org
/licenses/MIT>) or the Apache License, Version 2.0 (see LICENSE of
<http://www.apache.org/licenses/LICENSE-2.0>), at your option. All
files in the project carrying such notice may not be copied, modified,
or distributed except according to those terms.
*/
/*!
This module is for platform-specific stuff.
*/

pub use self::inner::{
    current_time, file_last_modified, force_cargo_color, get_cache_dir, get_config_dir,
    migrate_old_data, read_path, write_path,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MigrationKind {
    DryRun,
    ForReal,
}

impl MigrationKind {
    pub fn for_real(&self) -> bool {
        *self == MigrationKind::ForReal
    }
}

#[cfg(any(unix, windows))]
mod inner_unix_or_windows {
    /**
    Gets the current system time, in milliseconds since the UNIX epoch.
    */
    pub fn current_time() -> u64 {
        /*
        This is kinda dicey, since *ideally* both this function and `file_last_modified` would be using the same underlying APIs.  They are not, insofar as I know.

        At least, not when targetting Windows.

        That said, so long as everything is in the same units and uses the same epoch, it should be fine.
        */
        let now_1970_utc = time::now_utc().to_timespec();
        if now_1970_utc.sec < 0 || now_1970_utc.nsec < 0 {
            // Fuck it.
            return 0;
        }
        let now_ms_1970_utc =
            (now_1970_utc.sec as u64 * 1000) + (now_1970_utc.nsec as u64 / 1_000_000);
        now_ms_1970_utc
    }
}

#[cfg(unix)]
mod inner {
    pub use super::inner_unix_or_windows::current_time;

    use crate::error::{Blame, MainError};
    use super::MigrationKind;
    use log::info;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};
    use std::{cmp, env, fs, io};

    /**
    Gets the last-modified time of a file, in milliseconds since the UNIX epoch.
    */
    pub fn file_last_modified(file: &fs::File) -> u64 {
        let mtime_s_1970_utc = file.metadata().map(|md| md.mtime()).unwrap_or(0);

        let mtime_s_1970_utc = cmp::max(0, mtime_s_1970_utc);
        mtime_s_1970_utc as u64 * 1000
    }

    /**
    Get a directory suitable for storing user- and machine-specific data which may or may not be persisted across sessions.

    This is chosen to match the location where Cargo places its cache data.
    */
    pub fn get_cache_dir() -> Result<PathBuf, MainError> {
        // try $CARGO_HOME then fall back to $HOME
        if let Some(home) = env::var_os("CARGO_HOME") {
            let home = Path::new(&home);
            let old_home = home.join(".cargo");
            if old_home.exists() {
                // Keep using the old directory in preference to the new one, but only if it still contains `script-cache` and/or `binary-cache`.
                if old_home.join("script-cache").exists() || old_home.join("binary-cache").exists()
                {
                    // Yup; use this one.
                    return Ok(old_home);
                }
            }

            // Just use `$CARGO_HOME` directly.
            return Ok(home.into());
        }

        if let Some(home) = env::var_os("HOME") {
            return Ok(Path::new(&home).join(".cargo"));
        }

        Err((Blame::Human, "neither $CARGO_HOME nor $HOME is defined").into())
    }

    /**
    Get a directory suitable for storing user-specific configuration data.

    This is chosen to match the location where Cargo places its configuration data.
    */
    pub fn get_config_dir() -> Result<PathBuf, MainError> {
        // Currently, this appears to be the same as the cache directory.
        get_cache_dir()
    }

    pub fn migrate_old_data(kind: MigrationKind) -> (Vec<String>, Result<(), MainError>) {
        let mut log = vec![];
        match migrate_0_2_0(kind, &mut log) {
            Ok(()) => (),
            Err(e) => return (log, Err(e)),
        }
        (log, Ok(()))
    }

    fn migrate_0_2_0(kind: MigrationKind, log: &mut Vec<String>) -> Result<(), MainError> {
        /*
        Previously, when `CARGO_HOME` was defined on !Windows, the cache would be at `$CARGO_HOME/.cargo`.  If it exists, its contents (`script-cache` and `binary-cache`) need to moved into `$CARGO_HOME` directly.
        */
        if let Some(home) = env::var_os("CARGO_HOME") {
            let home = Path::new(&home);
            let old_base = home.join(".cargo");
            if old_base.exists() {
                info!(
                    "<0.2.0 cache directory ({:?}) exists; attempting migration",
                    old_base
                );

                /*
                Why both `info!` and `log`?  One for *before* we try (to help debug any issues) that only appears in the "real" log, and one for the user to let them know what we did/didn't do.
                */

                let old_script_cache = old_base.join("script-cache");
                let new_script_cache = home.join("script-cache");
                match (old_script_cache.exists(), new_script_cache.exists()) {
                    (true, true) => {
                        info!(
                            "not migrating {:?}; already exists at new location",
                            old_script_cache
                        );
                        log.push(format!(
                            "Did not move {:?}: new location {:?} already exists.",
                            old_script_cache, new_script_cache
                        ));
                    }
                    (true, false) => {
                        info!("migrating {:?} -> {:?}", old_script_cache, new_script_cache);
                        if kind.for_real() {
                            fs::rename(&old_script_cache, &new_script_cache)?;
                        }
                        log.push(format!(
                            "Moved {:?} to {:?}.",
                            old_script_cache, new_script_cache
                        ));
                    }
                    (false, _) => {
                        info!("not migrating {:?}; does not exist", old_script_cache);
                    }
                }

                let old_binary_cache = old_base.join("binary-cache");
                let new_binary_cache = home.join("binary-cache");
                match (old_binary_cache.exists(), new_binary_cache.exists()) {
                    (true, true) => {
                        info!(
                            "not migrating {:?}; already exists at new location",
                            old_binary_cache
                        );
                        log.push(format!(
                            "Did not move {:?}: new location {:?} already exists.",
                            old_binary_cache, new_binary_cache
                        ));
                    }
                    (true, false) => {
                        info!("migrating {:?} -> {:?}", old_binary_cache, new_binary_cache);
                        if kind.for_real() {
                            fs::rename(&old_binary_cache, &new_binary_cache)?;
                        }
                        log.push(format!(
                            "Moved {:?} to {:?}.",
                            old_script_cache, new_script_cache
                        ));
                    }
                    (false, _) => {
                        info!("not migrating {:?}; does not exist", old_binary_cache);
                    }
                }

                // If `$CARGO_HOME/.cargo` is empty, remove it.
                if fs::read_dir(&old_base)?.next().is_none() {
                    info!("{:?} is empty; removing", old_base);
                    if kind.for_real() {
                        fs::remove_dir(&old_base)?;
                    }
                    log.push(format!("Removed empty directory {:?}", old_base));
                } else {
                    info!("not removing {:?}; not empty", old_base);
                    log.push(format!("Not removing {:?}: not empty.", old_base));
                }

                info!("done with migration");
            }
        }

        Ok(())
    }

    pub fn write_path<W>(w: &mut W, path: &Path) -> io::Result<()>
    where
        W: io::Write,
    {
        w.write_all(path.as_os_str().as_bytes())
    }

    pub fn read_path<R>(r: &mut R) -> io::Result<PathBuf>
    where
        R: io::Read,
    {
        use std::ffi::OsStr;
        let mut buf = vec![];
        r.read_to_end(&mut buf)?;
        Ok(OsStr::from_bytes(&buf).into())
    }

    /**
    Returns `true` if `cargo-script` should force Cargo to use coloured output.

    This depends on whether `cargo-script`'s STDERR is connected to a TTY or not.
    */
    pub fn force_cargo_color() -> bool {
        atty::is(atty::Stream::Stderr)
    }
}

#[cfg(windows)]
pub mod inner {
    #![allow(non_snake_case)]

    pub use super::inner_unix_or_windows::current_time;

    use super::MigrationKind;
    use crate::error::MainError;
    use std::ffi::OsString;
    use std::fmt;
    use std::fs;
    use std::io;
    use std::mem;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};

    /**
    Gets the last-modified time of a file, in milliseconds since the UNIX epoch.
    */
    pub fn file_last_modified(file: &fs::File) -> u64 {
        use std::os::windows::fs::MetadataExt;

        const MS_BETWEEN_1601_1970: u64 = 11_644_473_600_000;

        let mtime_100ns_1601_utc = file.metadata().map(|md| md.last_write_time()).unwrap_or(0);
        let mtime_ms_1601_utc = mtime_100ns_1601_utc / (1000 * 10);

        // This can obviously underflow... but since files created prior to 1970 are going to be *somewhat rare*, I'm just going to saturate to zero.
        let mtime_ms_1970_utc = mtime_ms_1601_utc.saturating_sub(MS_BETWEEN_1601_1970);
        mtime_ms_1970_utc
    }

    /**
    Get a directory suitable for storing user- and machine-specific data which may or may not be persisted across sessions.

    This is *not* chosen to match the location where Cargo places its cache data, because Cargo is *wrong*.  This is at least *less wrong*.

    On Windows, LocalAppData is where user- and machine- specific data should go, but it *might* be more appropriate to use whatever the official name for "Program Data" is, though.
    */
    pub fn get_cache_dir() -> Result<PathBuf, MainError> {
        let rfid = &winapi::um::knownfolders::FOLDERID_LocalAppData;
        let dir =
            SHGetKnownFolderPath(rfid, 0, ::std::ptr::null_mut()).map_err(|e| e.to_string())?;
        Ok(Path::new(&dir).to_path_buf().join("Cargo"))
    }

    /**
    Get a directory suitable for storing user-specific configuration data.

    This is *not* chosen to match the location where Cargo places its cache data, because Cargo is *wrong*.  This is at least *less wrong*.
    */
    pub fn get_config_dir() -> Result<PathBuf, MainError> {
        let rfid = &winapi::um::knownfolders::FOLDERID_RoamingAppData;
        let dir =
            SHGetKnownFolderPath(rfid, 0, ::std::ptr::null_mut()).map_err(|e| e.to_string())?;
        Ok(Path::new(&dir).to_path_buf().join("Cargo"))
    }

    type WinResult<T> = Result<T, WinError>;

    struct WinError(winapi::shared::winerror::HRESULT);

    impl fmt::Display for WinError {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            write!(fmt, "HRESULT({})", self.0)
        }
    }

    fn SHGetKnownFolderPath(
        rfid: &winapi::um::shtypes::KNOWNFOLDERID,
        dwFlags: winapi::shared::minwindef::DWORD,
        hToken: winapi::shared::ntdef::HANDLE,
    ) -> WinResult<OsString> {
        use winapi::shared::ntdef::PWSTR;
        let mut psz_path: PWSTR = unsafe { mem::uninitialized() };
        let hresult = unsafe {
            winapi::um::shlobj::SHGetKnownFolderPath(
                rfid,
                dwFlags,
                hToken,
                mem::transmute(&mut psz_path as &mut PWSTR as *mut PWSTR),
            )
        };

        if hresult == winapi::shared::winerror::S_OK {
            let r = unsafe { pwstr_to_os_string(psz_path) };
            unsafe { winapi::um::combaseapi::CoTaskMemFree(psz_path as *mut _) };
            Ok(r)
        } else {
            Err(WinError(hresult))
        }
    }

    unsafe fn pwstr_to_os_string(ptr: winapi::shared::ntdef::PWSTR) -> OsString {
        OsStringExt::from_wide(::std::slice::from_raw_parts(ptr, pwstr_len(ptr)))
    }

    unsafe fn pwstr_len(mut ptr: winapi::shared::ntdef::PWSTR) -> usize {
        let mut len = 0;
        while *ptr != 0 {
            len += 1;
            ptr = ptr.offset(1);
        }
        len
    }

    pub fn migrate_old_data(kind: MigrationKind) -> (Vec<String>, Result<(), MainError>) {
        // Avoid unused code/variable warnings.
        let _ = kind.for_real();
        (vec![], Ok(()))
    }

    pub fn write_path<W>(w: &mut W, path: &Path) -> io::Result<()>
    where
        W: io::Write,
    {
        for word in path.as_os_str().encode_wide() {
            let lo = (word & 0xff) as u8;
            let hi = (word >> 8) as u8;
            w.write_all(&[lo, hi])?;
        }
        Ok(())
    }

    pub fn read_path<R>(r: &mut R) -> io::Result<PathBuf>
    where
        R: io::Read,
    {
        let mut buf = vec![];
        r.read_to_end(&mut buf)?;

        let mut words = Vec::with_capacity(buf.len() / 2);
        let mut it = buf.iter().cloned();
        while let Some(lo) = it.next() {
            let hi = it.next().unwrap();
            words.push(lo as u16 | ((hi as u16) << 8));
        }

        return Ok(OsString::from_wide(&words).into());
    }

    /**
    Returns `true` if `cargo-script` should force Cargo to use coloured output.

    Always returns `false` on Windows because colour is communicated over a side-channel.
    */
    pub fn force_cargo_color() -> bool {
        false
    }
}
