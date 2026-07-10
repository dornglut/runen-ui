#![forbid(unsafe_code)]

use std::{
    env,
    ffi::{OsStr, OsString},
    process::{Command, ExitCode},
};

const VALIDATE_STEPS: &[&[&str]] = &[
    &["fmt", "--all", "--check"],
    &["test", "--workspace"],
    &[
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--",
        "-D",
        "warnings",
    ],
];

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);

    match arguments.next().as_deref() {
        Some("validate") => validate(),
        Some("help" | "--help" | "-h") | None => {
            print_usage();
            ExitCode::SUCCESS
        }
        Some(command) => {
            eprintln!("unknown xtask command: {command}");
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn validate() -> ExitCode {
    let cargo = cargo_program();

    for step in VALIDATE_STEPS {
        if let Err(error) = run_cargo_step(cargo.as_os_str(), step) {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn run_cargo_step(cargo: &OsStr, arguments: &[&str]) -> Result<(), String> {
    let command = arguments.join(" ");
    eprintln!("> cargo {command}");

    let status = Command::new(cargo)
        .args(arguments)
        .status()
        .map_err(|error| format!("failed to run cargo {command}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {command} failed with status {status}"))
    }
}

fn cargo_program() -> OsString {
    env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

fn print_usage() {
    eprintln!("usage: cargo validate");
    eprintln!("       cargo xtask validate");
}
