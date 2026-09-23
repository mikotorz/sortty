use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

use crate::error::AppError;

/// Writes `contents` to `path` without ever leaving a partially-written file
/// behind: writes to a sibling `<name>.tmp` file first, then renames it onto
/// `path`. The rename is a single filesystem operation, so a crash or power
/// loss mid-write can only ever leave the old `path` (untouched) or the new
/// one — never a truncated/corrupt one.
pub fn write_atomic(path: &Path, contents: &str) -> Result<(), AppError> {
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp_path = path.with_file_name(tmp_name);

    std::fs::write(&tmp_path, contents).map_err(|e| AppError::io(tmp_path.clone(), e))?;
    std::fs::rename(&tmp_path, path).map_err(|e| AppError::io(path.to_path_buf(), e))
}

/// Moves the file at `from` to `to`, creating `to`'s parent folders as
/// needed. This is the only way apply and undo move files (ADR 0017):
///
/// - It never overwrites: an existing `to` is an error, not a replacement
///   (unlike `std::fs::rename`, which replaces on Windows).
/// - It only falls back to copy + remove for a genuine cross-volume move,
///   never for other failures like a file that's open in another program.
/// - It's all-or-nothing: if the source can't be removed after a
///   cross-volume copy, the copy is deleted again and the move fails.
pub fn move_no_clobber(from: &Path, to: &Path) -> Result<(), AppError> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent.to_path_buf(), e))?;
    }
    match rename_no_replace(from, to) {
        Ok(()) => Ok(()),
        Err(e) if is_cross_device(&e) => copy_then_remove(from, to),
        Err(e) => Err(AppError::io(from.to_path_buf(), e)),
    }
}

fn copy_then_remove(from: &Path, to: &Path) -> Result<(), AppError> {
    let copy = || -> io::Result<()> {
        let mut src = File::open(from)?;
        let modified = src.metadata()?.modified()?;
        // `create_new` so the copy itself can never overwrite either.
        let mut dst = OpenOptions::new().write(true).create_new(true).open(to)?;
        let copied = io::copy(&mut src, &mut dst).and_then(|_| dst.set_modified(modified));
        if copied.is_err() {
            drop(dst);
            let _ = std::fs::remove_file(to);
        }
        copied
    };
    copy().map_err(|e| AppError::io(from.to_path_buf(), e))?;

    if let Err(e) = std::fs::remove_file(from) {
        // Roll back rather than leave the file in two places.
        let _ = std::fs::remove_file(to);
        return Err(AppError::io(from.to_path_buf(), e));
    }
    Ok(())
}

#[cfg(windows)]
fn is_cross_device(e: &io::Error) -> bool {
    // ERROR_NOT_SAME_DEVICE
    e.raw_os_error() == Some(17)
}

#[cfg(not(windows))]
fn is_cross_device(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::CrossesDevices
}

#[cfg(windows)]
fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    let from = verbatim_wide(from)?;
    let to = verbatim_wide(to)?;
    // Flags = 0: no MOVEFILE_REPLACE_EXISTING (fail if `to` exists) and no
    // MOVEFILE_COPY_ALLOWED (cross-volume is handled by `copy_then_remove`).
    // SAFETY: both buffers are NUL-terminated UTF-16 and outlive the call.
    let ok = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// `path` as a NUL-terminated, `\\?\`-prefixed wide string, so the raw Win32
/// call handles paths past MAX_PATH the same way `std::fs` already does
/// (ADR 0010). `std::path::absolute` normalizes `/` and `..` first, which a
/// verbatim path would otherwise take literally.
#[cfg(windows)]
fn verbatim_wide(path: &Path) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;

    let absolute = std::path::absolute(path)?;
    let wide: Vec<u16> = absolute.as_os_str().encode_wide().collect();
    let bs = u16::from(b'\\');
    let prefix: Vec<u16> = if wide.starts_with(&[bs, bs, u16::from(b'?'), bs]) {
        Vec::new()
    } else if wide.starts_with(&[bs, bs]) {
        // \\server\share\x -> \\?\UNC\server\share\x
        return Ok(r"\\?\UNC"
            .encode_utf16()
            .chain(wide[1..].iter().copied())
            .chain(std::iter::once(0))
            .collect());
    } else {
        r"\\?\".encode_utf16().collect()
    };
    Ok(prefix
        .into_iter()
        .chain(wide)
        .chain(std::iter::once(0))
        .collect())
}

#[cfg(not(windows))]
fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        return Err(io::Error::from(io::ErrorKind::AlreadyExists));
    }
    std::fs::rename(from, to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn writes_and_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");

        write_atomic(&path, "first").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "first");

        write_atomic(&path, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");

        // No leftover temp file.
        assert!(!path.with_file_name("data.json.tmp").exists());
    }

    #[test]
    fn moves_a_file_and_creates_missing_parent_folders() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"a").unwrap();

        move_no_clobber(&dir.path().join("a.txt"), &dir.path().join("x/y/a.txt")).unwrap();

        assert!(!dir.path().join("a.txt").exists());
        assert_eq!(fs::read(dir.path().join("x/y/a.txt")).unwrap(), b"a");
    }

    #[test]
    fn refuses_to_overwrite_an_existing_destination() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"incoming").unwrap();
        fs::write(dir.path().join("b.txt"), b"existing").unwrap();

        let result = move_no_clobber(&dir.path().join("a.txt"), &dir.path().join("b.txt"));

        assert!(result.is_err());
        assert_eq!(fs::read(dir.path().join("a.txt")).unwrap(), b"incoming");
        assert_eq!(fs::read(dir.path().join("b.txt")).unwrap(), b"existing");
    }

    /// Regression: a file open in another program used to fail the rename,
    /// then get *copied* to the destination anyway, leaving an orphan copy
    /// no run record knew about.
    #[cfg(windows)]
    #[test]
    fn a_locked_file_fails_without_leaving_a_copy_behind() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("locked.txt");
        let to = dir.path().join("Docs/locked.txt");
        fs::write(&from, b"x").unwrap();
        // Share read only: renaming or deleting it is refused while open.
        let _handle = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&from)
            .unwrap();

        assert!(move_no_clobber(&from, &to).is_err());
        assert!(!to.exists());
        assert!(from.exists());
    }

    #[cfg(windows)]
    #[test]
    fn cross_volume_copy_rolls_back_when_the_source_cannot_be_removed() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("locked.txt");
        let to = dir.path().join("copy.txt");
        fs::write(&from, b"x").unwrap();
        let _handle = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&from)
            .unwrap();

        assert!(copy_then_remove(&from, &to).is_err());
        assert!(!to.exists());
        assert!(from.exists());
    }

    #[test]
    fn cross_volume_copy_keeps_the_modified_time() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("a.txt");
        let to = dir.path().join("b.txt");
        fs::write(&from, b"a").unwrap();
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        File::options()
            .write(true)
            .open(&from)
            .unwrap()
            .set_modified(old)
            .unwrap();

        copy_then_remove(&from, &to).unwrap();

        assert!(!from.exists());
        assert_eq!(fs::metadata(&to).unwrap().modified().unwrap(), old);
    }
}
