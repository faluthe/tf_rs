//! Builds `tf_rs` and injects it into the running game (`tf_linux64`) by having `gdb`
//! call `dlopen` in it. `sudo` is needed for `gdb` to ptrace the game.
//!
//! ```text
//! cargo inject            # debug build
//! cargo inject --release
//! cargo inject --debug    # also opens a terminal tailing the log
//! ```
//!
//! The terminal is `$TERMINAL`, `xdg-terminal-exec` or `x-terminal-emulator`, the first
//! one found.
//!
//! Injecting by hand with `inject/so_inject.sh`:
//!
//! ```text
//! cargo build -p tf_rs
//! sudo ./inject/so_inject.sh inject $(pidof tf_linux64) $PWD/target/debug/libtf_rs.so
//! sudo ./inject/so_inject.sh uninject <pid> <handle printed by dlopen>
//! ```
//!
//! Don't uninject after using the menu's Unload button, it already unloaded the library.

use core::panic;
use std::{
    io::{BufRead, BufReader, ErrorKind, IsTerminal, Read},
    path::Path,
    process::{Command, ExitCode, Stdio},
    thread,
};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Message {
    reason: String,
    filenames: Vec<String>,
}

/// Drops ANSI escape sequences so cargo's colored status lines can be matched.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip to the end of the CSI sequence, which ends with a letter
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn get_lib_path(release: bool) -> String {
    let color = std::io::stderr().is_terminal();
    let mut args = vec![
        "build",
        "-p",
        "tf_rs",
        "--message-format=json-render-diagnostics",
    ];
    if release {
        args.push("--release");
    }
    // cargo turns color off when its stderr is a pipe
    if color {
        args.push("--color=always");
    }

    let mut child = Command::new("cargo")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute cargo build");

    /* Relay cargo's progress and diagnostics, but replace "Finished" when
     * nothing was compiled since there was nothing to finish. */
    let stderr = child.stderr.take().expect("cargo stderr not piped");
    let relay = thread::spawn(move || {
        let mut compiled = false;
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let status = strip_ansi(&line);
            let status = status.trim_start();
            if status.starts_with("Compiling") {
                compiled = true;
            } else if !compiled && status.starts_with("Finished") {
                if color {
                    eprintln!("\x1b[1;32m{:>12}\x1b[0m no rebuild needed", "Up to date,");
                } else {
                    eprintln!("{:>12} no rebuild needed", "Up to date,");
                }
                continue;
            }
            eprintln!("{line}");
        }
    });

    let mut stdout = String::new();
    child
        .stdout
        .take()
        .expect("cargo stdout not piped")
        .read_to_string(&mut stdout)
        .expect("Failed to read cargo output");
    let status = child.wait().expect("Failed to wait on cargo build");
    relay.join().expect("stderr relay thread panicked");
    if !status.success() {
        eprintln!("cargo build failed");
        std::process::exit(1);
    }

    let mut lib_path = None;
    for line in stdout.lines() {
        if let Ok(msg) = serde_json::from_str::<Message>(line) {
            if msg.reason == "compiler-artifact"
                && !msg.filenames.is_empty()
                && msg.filenames[0].ends_with(".so")
            {
                lib_path = Some(msg.filenames[0].clone());
                break;
            }
        }
    }
    lib_path.expect("Could not find compiler-artifact filename")
}

fn get_pid() -> String {
    let pid = String::from_utf8(
        Command::new("pidof")
            .arg("tf_linux64")
            .output()
            .expect("Failed to execute pidof")
            .stdout,
    )
    .expect("pidof output not UTF-8")
    .trim()
    .to_string();

    if pid.is_empty() {
        panic!("No process found with name 'tf_linux64'");
    }

    pid
}

/// Must match `LOG_FILE` in tf_rs/src/logger.rs, which writes next to the .so.
const LOG_FILE: &str = "tf_rs.log";

fn open_debug_terminal(lib_path: &str) {
    let log_path = Path::new(lib_path)
        .parent()
        .expect("Library path has no parent directory")
        .join(LOG_FILE);

    // -F follows the file across the truncate tf_rs does on each injection
    let tail = ["tail", "-n", "0", "-F"];

    /* The user's default terminal, in order of preference: $TERMINAL (which
     * by convention takes -e), the xdg-terminal-exec spec launcher, then
     * Debian's x-terminal-emulator alternative. */
    let mut candidates: Vec<(String, Option<&str>)> = Vec::new();
    if let Ok(term) = std::env::var("TERMINAL") {
        if !term.is_empty() {
            candidates.push((term, Some("-e")));
        }
    }
    candidates.push(("xdg-terminal-exec".to_string(), None));
    candidates.push(("x-terminal-emulator".to_string(), Some("-e")));

    for (term, exec_flag) in &candidates {
        let mut cmd = Command::new(term);
        cmd.args(exec_flag).args(tail).arg(&log_path);

        // Don't wait, some terminals only exit when their window closes
        match cmd.spawn() {
            Ok(_) => return,
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => eprintln!("Failed to launch {term}: {e}"),
        }
    }

    eprintln!(
        "No terminal found, set $TERMINAL or run: tail -n 0 -F {}",
        log_path.display()
    );
}

fn main() -> ExitCode {
    let debug = std::env::args().any(|arg| arg == "--debug");
    let release = std::env::args().any(|arg| arg == "--release");
    let lib_path = get_lib_path(release);
    let pid = get_pid();
    if debug {
        open_debug_terminal(&lib_path);
    }

    let status = Command::new("sudo")
        .args([
            "bash",
            if debug {
                "./inject/so_inject_debug.sh"
            } else {
                "./inject/so_inject.sh"
            },
            "inject",
            &pid,
            &lib_path,
        ])
        .status()
        .expect("Failed to execute so_inject_debug.sh");
    if !status.success() {
        return ExitCode::from(1);
    }

    ExitCode::from(0)
}
