//! File logger.
//!
//! The game runs inside the Steam Linux Runtime (pressure-vessel) container,
//! where stdout/stderr are pipes owned by Steam, so we log to a file next to
//! our own .so instead. That path is visible from both the container and the
//! host, since the container had to be able to dlopen the library.
//!
//! This deliberately avoids thread-locals with destructors glibc won't unmap
//! a library while a live thread has a pending TLS destructor from it, which
//! kept dlclose from unloading us.

use std::{
    collections::VecDeque,
    ffi::{CString, OsStr},
    fmt::Write,
    mem,
    os::unix::ffi::OsStrExt,
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicI32, AtomicUsize, Ordering},
    },
};

use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::helpers;

/// Must match the name the injector tails.
const LOG_FILE: &str = "tf_rs.log";

/// Most recent lines kept in memory for the menu's log view.
const MAX_LINES: usize = 1000;

/// Every line logged at any level, oldest first, and the total number ever logged.
static LINES: Mutex<(VecDeque<(Level, String)>, u64)> = Mutex::new((VecDeque::new(), 0));

/// Most verbose level written to the log file and shown in the menu, as a `LevelFilter`.
/// Every level is still kept in `LINES`, so lowering this later shows the earlier lines.
static LEVEL: AtomicUsize = AtomicUsize::new(if cfg!(debug_assertions) {
    LevelFilter::Debug as usize
} else {
    LevelFilter::Info as usize
});

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
        let fd = self.fd.load(Ordering::Acquire);
        if fd < 0 {
            return;
        }

        let target = record.target();
        let target = target.strip_prefix("tf_rs::").unwrap_or(target);

        let mut line = String::with_capacity(128);
        write_timestamp(&mut line);
        let _ = writeln!(line, " {:<5} {target}: {}", record.level(), record.args());

        if record.level() <= level() {
            write_all(fd, line.as_bytes());
        }

        line.pop();
        let mut lines = LINES.lock().unwrap_or_else(|e| e.into_inner());
        if lines.0.len() == MAX_LINES {
            lines.0.pop_front();
        }
        lines.0.push_back((record.level(), line));
        lines.1 += 1;
    }

    fn flush(&self) {}
}

pub fn init() {
    let file_fd = open_log_file();
    let fd = file_fd.unwrap_or_else(|| unsafe { libc::dup(libc::STDERR_FILENO) });
    LOGGER.fd.store(fd, Ordering::Release);

    if log::set_logger(&LOGGER).is_ok() {
        // Record everything, `LEVEL` filters what is written and shown
        log::set_max_level(LevelFilter::Trace);
    }

    if file_fd.is_none() {
        log::warn!("Could not open {LOG_FILE} next to the library, logging to stderr");
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

    // Free the buffered lines, the heap outlives our unmapping
    LINES.lock().unwrap_or_else(|e| e.into_inner()).0 = VecDeque::new();
}

pub fn level() -> LevelFilter {
    LevelFilter::iter()
        .nth(LEVEL.load(Ordering::Relaxed))
        .unwrap_or(LevelFilter::Info)
}

pub fn set_level(level: LevelFilter) {
    LEVEL.store(level as usize, Ordering::Relaxed);
}

/// Copy the buffered lines out if more than `seen` lines have been logged since.
/// Returns the new total line count alongside the lines.
pub fn lines_since(seen: u64) -> Option<(Vec<(Level, String)>, u64)> {
    // Copy rather than hand out the guard, so nothing logs while we hold the lock
    let lines = LINES.lock().unwrap_or_else(|e| e.into_inner());
    (lines.1 != seen).then(|| (lines.0.iter().cloned().collect(), lines.1))
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
