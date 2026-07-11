#![forbid(unsafe_code)]

use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const VALIDATE_STEPS: &[(&str, &[&str])] = &[
    ("stable", &["fmt", "--all", "--check"]),
    ("stable", &["test", "--workspace", "--locked"]),
    (
        "stable",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    ),
    ("1.93.0", &["test", "--workspace", "--locked"]),
];

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);

    match arguments.next().as_deref() {
        Some("validate") => validate(),
        Some("check-links") => check_links(),
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
    for (toolchain, arguments) in VALIDATE_STEPS {
        if let Err(error) = run_cargo_step(toolchain, arguments) {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    }

    check_links()
}

fn check_links() -> ExitCode {
    let root = match env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("failed to determine repository root: {error}");
            return ExitCode::FAILURE;
        }
    };

    match validate_markdown_links(&root) {
        Ok(file_count) => {
            eprintln!("> checked relative Markdown links in {file_count} files");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_cargo_step(toolchain: &str, arguments: &[&str]) -> Result<(), String> {
    let command = arguments.join(" ");
    eprintln!("> cargo +{toolchain} {command}");

    let status = Command::new("rustup")
        .args(["run", toolchain, "cargo"])
        .args(arguments)
        .status()
        .map_err(|error| format!("failed to run cargo +{toolchain} {command}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "cargo +{toolchain} {command} failed with status {status}"
        ))
    }
}

fn validate_markdown_links(root: &Path) -> Result<usize, String> {
    let mut files = Vec::new();
    collect_markdown_files(root, root, &mut files)?;
    files.sort();

    let mut failures = Vec::new();

    for relative in &files {
        let absolute = root.join(relative);
        let contents = fs::read_to_string(&absolute)
            .map_err(|error| format!("failed to read {}: {error}", relative.display()))?;

        for target in markdown_targets(&contents) {
            let Some(path) = local_link_path(target) else {
                continue;
            };

            let destination = absolute.parent().unwrap_or(root).join(path);

            if !destination.exists() {
                failures.push(format!("{} -> {target}", relative.display()));
            }
        }
    }

    if failures.is_empty() {
        Ok(files.len())
    } else {
        Err(format!(
            "broken relative Markdown links:\n{}",
            failures.join("\n")
        ))
    }
}

fn collect_markdown_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect an entry in {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();

        if path.is_dir() {
            if !is_ignored_directory(&path) {
                collect_markdown_files(root, &path, files)?;
            }
        } else if path.extension() == Some(OsStr::new("md")) {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("failed to relativize {}: {error}", path.display()))?;
            files.push(relative.to_path_buf());
        }
    }

    Ok(())
}

fn is_ignored_directory(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(OsStr::to_str),
        Some(".git" | "target" | "context" | ".context" | "context-exports")
    )
}

fn markdown_targets(contents: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut remaining = contents;

    while let Some(start) = remaining.find("](") {
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find(')') else {
            break;
        };
        targets.push(after_start[..end].trim());
        remaining = &after_start[end + 1..];
    }

    targets
}

fn local_link_path(target: &str) -> Option<&str> {
    if target.is_empty()
        || target.starts_with('#')
        || target.starts_with("https://")
        || target.starts_with("http://")
        || target.starts_with("mailto:")
    {
        return None;
    }

    let target = target.strip_prefix('<').unwrap_or(target);
    let target = target.strip_suffix('>').unwrap_or(target);
    let target = target.split_whitespace().next().unwrap_or(target);
    let path = target.split('#').next().unwrap_or(target);

    (!path.is_empty()).then_some(path)
}

fn print_usage() {
    eprintln!("usage: cargo validate");
    eprintln!("       cargo xtask check-links");
}

#[cfg(test)]
mod tests {
    use super::{local_link_path, markdown_targets};

    #[test]
    fn markdown_target_parser_finds_inline_links() {
        assert_eq!(
            markdown_targets("[one](docs/one.md) and [two](docs/two.md#part)"),
            ["docs/one.md", "docs/two.md#part"]
        );
    }

    #[test]
    fn local_link_parser_ignores_external_and_anchor_links() {
        assert_eq!(local_link_path("https://example.com"), None);
        assert_eq!(local_link_path("#section"), None);
        assert_eq!(
            local_link_path("docs/status-map.md#current"),
            Some("docs/status-map.md")
        );
    }
}
