/* File logger.
 *
 * The game runs inside the Steam Linux Runtime (pressure-vessel) container,
 * where stdout/stderr are pipes owned by Steam, so we log to a file next to
 * our own .so instead. That path is visible from both the container and the
 * host, since the container had to be able to dlopen the library.
 *
 * This deliberately avoids thread-locals with destructors (env_logger keeps a
 * thread-local buffer). glibc won't unmap a library while a live thread has a
 * pending TLS destructor from it, which kept dlclose from unloading us. */

use std::{
    ffi::{CString, OsStr},
    fmt::Write,
    mem,
    os::unix::ffi::OsStrExt,
    path::Path,
    sync::atomic::{AtomicI32, Ordering},
};

use log::{LevelFilter, Log, Metadata, Record};

use crate::helpers;

/// Must match the name the injector tails.
const LOG_FILE: &str = "tf_rs.log";

static LOGGER: FileLogger = FileLogger {
    fd: AtomicI32::new(-1),
};

struct FileLogger {
    fd: AtomicI32,
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let fd = self.fd.load(Ordering::Acquire);
        if fd < 0 {
            return;
        }

        let target = record.target();
        let target = target.strip_prefix("tf_rs::").unwrap_or(target);

        let mut line = String::with_capacity(128);
        write_timestamp(&mut line);
        let _ = writeln!(line, " {:<5} {target}: {}", record.level(), record.args());

        write_all(fd, line.as_bytes());
    }

    fn flush(&self) {}
}

pub fn init() {
    let fd = open_log_file().unwrap_or_else(|| unsafe { libc::dup(libc::STDERR_FILENO) });
    LOGGER.fd.store(fd, Ordering::Release);

    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(if cfg!(debug_assertions) {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        });
    }

    // Panics in hooks abort the game at the extern "C" boundary, get the reason into the log first
    std::panic::set_hook(Box::new(|info| log::error!("{info}")));
}

/// Close the log file. Later log calls are dropped.
pub fn shutdown() {
    let fd = LOGGER.fd.swap(-1, Ordering::AcqRel);
    if fd >= 0 {
        unsafe { libc::close(fd) };
    }
}

fn open_log_file() -> Option<i32> {
    let lib = helpers::lib_path()?;
    let dir = Path::new(OsStr::from_bytes(lib.as_bytes())).parent()?;
    let path = CString::new(dir.join(LOG_FILE).as_os_str().as_bytes()).ok()?;

    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC | libc::O_APPEND | libc::O_CLOEXEC,
            0o644,
        )
    };

    (fd >= 0).then_some(fd)
}

fn write_timestamp(out: &mut String) {
    unsafe {
        let mut ts: libc::timespec = mem::zeroed();
        let mut tm: libc::tm = mem::zeroed();
        libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
        libc::localtime_r(&ts.tv_sec, &mut tm);
        let _ = write!(
            out,
            "{:02}:{:02}:{:02}.{:03}",
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec,
            ts.tv_nsec / 1_000_000
        );
    }
}

fn write_all(fd: i32, mut buf: &[u8]) {
    while !buf.is_empty() {
        let n = unsafe { libc::write(fd, buf.as_ptr().cast(), buf.len()) };
        if n < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return;
        }
        buf = &buf[n as usize..];
    }
}
