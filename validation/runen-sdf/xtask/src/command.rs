use std::process::{Command, ExitStatus};

pub fn run_validation_commands() -> Result<(), String> {
    run("cargo", &["metadata", "--format-version", "1", "--locked", "--no-deps"])?;
    run("cargo", &["tree", "-p", "runen-sdf", "--locked"])?;
    run("cargo", &["tree", "-i", "runen-sdf", "--workspace", "--locked"])?;
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run("cargo", &["test", "--workspace", "--locked"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run_rustdoc()?;
    run("cargo", &["+1.93.0", "test", "--workspace", "--locked"])?;
    run("git", &["diff", "--check"])
}

pub fn prove_clean_repository_state() -> Result<(), String> {
    let output = Command::new("git")
        .args(["status", "--short"])
        .output()
        .map_err(|error| format!("failed to execute git status --short: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "git status --short failed with {}",
            display_status(output.status)
        ));
    }

    if output.stdout.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "validation changed repository state:\n{}",
            String::from_utf8_lossy(&output.stdout)
        ))
    }
}

fn run_rustdoc() -> Result<(), String> {
    let arguments = ["doc", "--workspace", "--no-deps", "--locked"];
    let status = Command::new("cargo")
        .args(arguments)
        .env("RUSTDOCFLAGS", "-D warnings")
        .status()
        .map_err(|error| format!("failed to execute cargo doc: {error}"))?;

    require_success("cargo", &arguments, status)
}

fn run(program: &str, arguments: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(arguments)
        .status()
        .map_err(|error| format!("failed to execute {program}: {error}"))?;

    require_success(program, arguments, status)
}

fn require_success(program: &str, arguments: &[&str], status: ExitStatus) -> Result<(), String> {
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "command failed with {}: {program} {}",
            display_status(status),
            arguments.join(" ")
        ))
    }
}

fn display_status(status: ExitStatus) -> String {
    status
        .code()
        .map_or_else(|| "no exit code".to_owned(), |code| format!("exit code {code}"))
}
