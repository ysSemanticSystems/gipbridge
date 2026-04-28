use anyhow::{anyhow, bail, Context, Result};
use std::ffi::CString;
use std::fs;
use std::io;
use std::os::fd::FromRawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn pipe_path(pipe_name: &str) -> Result<PathBuf> {
    if pipe_name.contains('/') {
        bail!("--pipe-name must not contain '/'");
    }
    if pipe_name.as_bytes().iter().any(|&b| b == 0) {
        bail!("--pipe-name must not contain NUL");
    }
    let home = std::env::var("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Dolphin")
        .join("Pipes")
        .join(pipe_name))
}

pub fn ensure_fifo(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create_dir_all({})", parent.display()))?;
    }

    if path.exists() {
        return Ok(());
    }

    let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|_| anyhow!("pipe path contains interior NUL"))?;
    let rc = unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) };
    if rc != 0 {
        bail!("mkfifo({}) failed: {}", path.display(), io::Error::last_os_error());
    }
    Ok(())
}

pub fn open_writer_nonblocking(path: &Path) -> Result<fs::File> {
    let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|_| anyhow!("pipe path contains interior NUL"))?;
    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_WRONLY | libc::O_NONBLOCK) };
    if fd < 0 {
        return Err(anyhow!(io::Error::last_os_error())).with_context(|| format!("open({})", path.display()));
    }
    Ok(unsafe { fs::File::from_raw_fd(fd) })
}

pub fn open_writer_wait(path: &Path) -> Result<fs::File> {
    loop {
        match open_writer_nonblocking(path) {
            Ok(f) => return Ok(f),
            Err(e) => {
                if let Some(ioe) = e.downcast_ref::<io::Error>() {
                    if ioe.raw_os_error() == Some(libc::ENXIO) {
                        // ENXIO is expected when the FIFO exists but no reader (Dolphin) is connected yet.
                        std::thread::sleep(Duration::from_millis(250));
                        continue;
                    }
                }
                return Err(e);
            }
        }
    }
}

