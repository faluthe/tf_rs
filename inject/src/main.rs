use core::panic;
use std::{
    path::Path,
    process::{Command, ExitCode},
};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Message {
    reason: String,
    filenames: Vec<String>,
}

fn get_lib_path(release: bool) -> String {
    let mut args = vec!["build", "-p", "tf_rs", "--message-format=json"];
    if release {
        args.push("--release");
    }

    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute cargo build");

    let mut lib_path = None;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
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
    let status = Command::new("gnome-terminal")
        .args(["--", "tail", "-n", "0", "-F"])
        .arg(&log_path)
        .status()
        .expect("Failed to execute terminal command");
    if !status.success() {
        panic!("Failed to open terminal for {}", log_path.display());
    }
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
