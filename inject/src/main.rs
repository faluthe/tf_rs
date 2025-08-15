use core::panic;
use std::process::{Command, ExitCode};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Message {
    reason: String,
    filenames: Vec<String>,
}

fn get_lib_path() -> String {
    let output = Command::new("cargo")
        .args(["build", "-p", "tf_rs", "--message-format=json"])
        .output()
        .expect("Failed to execute cargo build");

    let lib_path = {
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
    };

    lib_path
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

fn open_debug_terminal() {
    let status = Command::new("gnome-terminal")
        .args(["--", "bash", "-lc", "cat /proc/$(pidof tf_linux64)/fd/1"])
        .status()
        .expect("Failed to execute terminal command");
    if !status.success() {
        panic!("Failed to open terminal for stdout output");
    }
    let status = Command::new("gnome-terminal")
        .args(["--", "bash", "-lc", "cat /proc/$(pidof tf_linux64)/fd/2"])
        .status()
        .expect("Failed to execute terminal command");
    if !status.success() {
        panic!("Failed to open terminal for stderr output");
    }
}

fn main() -> ExitCode {
    let debug = std::env::args().any(|arg| arg == "--debug");
    let lib_path = get_lib_path();
    let pid = get_pid();
    if debug {
        open_debug_terminal();
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
