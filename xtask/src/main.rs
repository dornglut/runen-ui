#![forbid(unsafe_code)]

mod repository_audit;

use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const EXPECTED_LICENSE_EXPRESSION: &str = "license = \"GPL-3.0-only\"";
const EXPECTED_GPL_MARKERS: &[&str] = &[
    "GNU GENERAL PUBLIC LICENSE",
    "Version 3, 29 June 2007",
    "END OF TERMS AND CONDITIONS",
];
const EXPECTED_POLICY_MARKERS: &[&str] = &[
    "GPL-3.0-only",
    "separate commercial agreement",
    "copyright holder(s) with sufficient rights",
    "previously released under MIT",
    "Third-party dependencies",
    "external pull requests contributing tracked repository content",
    "Issue reports, design discussion, reviews, and reproducible cases",
];
const EXPECTED_WORKSPACE_PACKAGE_MANIFESTS: &[&str] = &[
    "crates/runenui_core/Cargo.toml",
    "crates/runenui_runtime/Cargo.toml",
    "crates/runenui_testing/Cargo.toml",
    "examples/counter/Cargo.toml",
    "tests/external_widget/Cargo.toml",
    "tests/external_renderer/Cargo.toml",
    "xtask/Cargo.toml",
];
const VALIDATE_STEPS: &[(&str, &[&str])] = &[
    ("stable", &["metadata", "--locked", "--no-deps"]),
    ("stable", &["fmt", "--all", "--check"]),
    (
        "stable",
        &["test", "--workspace", "--all-features", "--locked"],
    ),
    (
        "stable",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    ),
    (
        "1.93.0",
        &["test", "--workspace", "--all-features", "--locked"],
    ),
];

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);

    match arguments.next().as_deref() {
        Some("validate") => validate(),
        Some("check-links") => check_links(),
        Some("audit-repository") => audit_repository(arguments),
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
    let root = match workspace_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    for (toolchain, arguments) in VALIDATE_STEPS {
        if let Err(error) = run_cargo_step(&root, toolchain, arguments) {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    }

    if let Err(error) = validate_current_licensing(&root) {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }

    if let Err(error) = check_repository_links(&root) {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }

    if let Err(error) = repository_audit::validate_fatal(&root) {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn check_links() -> ExitCode {
    let root = match workspace_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    match check_repository_links(&root) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn audit_repository(arguments: impl Iterator<Item = String>) -> ExitCode {
    let format = match repository_audit::parse_output_format(arguments) {
        Ok(format) => format,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let root = match workspace_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    match repository_audit::run(&root, format) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn workspace_root() -> Result<PathBuf, String> {
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_directory.parent().ok_or_else(|| {
        format!(
            "xtask manifest directory has no workspace parent: {}",
            manifest_directory.display()
        )
    })?;

    if root.join("Cargo.toml").is_file() {
        Ok(root.to_path_buf())
    } else {
        Err(format!(
            "resolved workspace root does not contain Cargo.toml: {}",
            root.display()
        ))
    }
}

fn run_cargo_step(root: &Path, toolchain: &str, arguments: &[&str]) -> Result<(), String> {
    let command = arguments.join(" ");
    eprintln!("> cargo +{toolchain} {command}");

    let status = Command::new("rustup")
        .args(["run", toolchain, "cargo"])
        .args(arguments)
        .current_dir(root)
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

fn check_repository_links(root: &Path) -> Result<(), String> {
    let files = validate_markdown_links(root)?;
    eprintln!(
        "> checked relative Markdown links in {} files from {}",
        files.len(),
        root.display()
    );
    Ok(())
}

fn validate_markdown_links(root: &Path) -> Result<Vec<PathBuf>, String> {
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
        Ok(files)
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
        Some(
            ".git"
                | "target"
                | "node_modules"
                | "dist"
                | "build"
                | ".astro"
                | "context"
                | ".context"
                | "context-exports"
        )
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

fn validate_repository_metadata(root: &Path) -> Result<(), String> {
    if root.join("LICENSING.md").is_file() {
        validate_current_licensing(root)
    } else {
        Ok(())
    }
}

fn validate_current_licensing(root: &Path) -> Result<(), String> {
    let license_path = root.join("LICENSE");
    let license = fs::read_to_string(&license_path)
        .map_err(|error| format!("failed to read {}: {error}", license_path.display()))?;

    if license.contains("MIT License")
        || license.contains("Permission is hereby granted, free of charge")
        || license.contains("THE SOFTWARE IS PROVIDED \"AS IS\"")
    {
        return Err("LICENSE contains the stale MIT representation".into());
    }

    for marker in EXPECTED_GPL_MARKERS {
        if !license.contains(marker) {
            return Err(format!(
                "LICENSE does not contain the canonical GPLv3 marker: {marker}"
            ));
        }
    }

    let licensing_path = root.join("LICENSING.md");
    let licensing = fs::read_to_string(&licensing_path)
        .map_err(|error| format!("failed to read {}: {error}", licensing_path.display()))?;
    for marker in EXPECTED_POLICY_MARKERS {
        if !licensing.contains(marker) {
            return Err(format!(
                "LICENSING.md does not contain the required policy statement: {marker}"
            ));
        }
    }

    let manifest_path = root.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;

    if !manifest.contains(EXPECTED_LICENSE_EXPRESSION) {
        return Err(format!(
            "workspace metadata must contain {EXPECTED_LICENSE_EXPRESSION}"
        ));
    }

    if !manifest.contains("publish = false") {
        return Err("workspace package publication must remain disabled".into());
    }

    for relative in EXPECTED_WORKSPACE_PACKAGE_MANIFESTS {
        let package_manifest_path = root.join(relative);
        let package_manifest = fs::read_to_string(&package_manifest_path).map_err(|error| {
            format!(
                "failed to read workspace package manifest {}: {error}",
                package_manifest_path.display()
            )
        })?;
        if !package_manifest.contains("license.workspace = true") {
            return Err(format!(
                "workspace package manifest {relative} must retain license.workspace = true"
            ));
        }
    }

    Ok(())
}

fn print_usage() {
    eprintln!("usage: cargo validate");
    eprintln!("       cargo xtask check-links");
    eprintln!("       cargo xtask audit-repository [--format json]");
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        process::{self, Command},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::{
        local_link_path, markdown_targets, validate_current_licensing, validate_markdown_links,
        workspace_root,
    };

    static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(label: &str) -> Result<Self, String> {
            let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "runenui-xtask-{label}-{}-{sequence}",
                process::id()
            ));
            fs::create_dir_all(&path)
                .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
            Ok(Self { path })
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write(&self, relative: &str, contents: &str) -> Result<(), String> {
            let path = self.path.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
            fs::write(&path, contents)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn resolved_workspace_root_contains_root_manifest() -> Result<(), String> {
        let root = workspace_root()?;
        assert!(root.join("Cargo.toml").is_file());
        Ok(())
    }

    #[test]
    fn repository_scan_includes_root_level_documents() -> Result<(), String> {
        let root = workspace_root()?;
        let files = validate_markdown_links(&root)?;
        assert!(files.contains(&PathBuf::from("README.md")));
        assert!(files.contains(&PathBuf::from("docs/status.md")));
        Ok(())
    }

    #[test]
    fn nested_invocation_scans_the_repository_root() -> Result<(), String> {
        let root = workspace_root()?;
        if std::env::var_os("RUNENUI_NESTED_INVOCATION_TEST").is_some() {
            assert_eq!(
                std::env::current_dir().map_err(|error| error.to_string())?,
                root.join("crates/runenui_core")
            );
            let files = validate_markdown_links(&workspace_root()?)?;
            assert!(files.contains(&PathBuf::from("README.md")));
            assert!(files.contains(&PathBuf::from("docs/status.md")));
            return Ok(());
        }

        let output = Command::new(std::env::current_exe().map_err(|error| error.to_string())?)
            .args([
                "--exact",
                "tests::nested_invocation_scans_the_repository_root",
                "--nocapture",
            ])
            .env("RUNENUI_NESTED_INVOCATION_TEST", "1")
            .current_dir(root.join("crates/runenui_core"))
            .output()
            .map_err(|error| format!("failed to run nested validation test: {error}"))?;

        if !output.status.success() {
            return Err(format!(
                "nested validation test failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    #[test]
    fn generated_and_build_directories_are_excluded() -> Result<(), String> {
        let directory = TestDirectory::new("ignored-directories")?;
        directory.write("README.md", "# Valid\n")?;
        for ignored in ["target", "build", "context", "node_modules", "dist"] {
            directory.write(&format!("{ignored}/broken.md"), "[broken](missing.md)\n")?;
        }

        let files = validate_markdown_links(directory.path())?;
        assert_eq!(files, [PathBuf::from("README.md")]);
        Ok(())
    }

    #[test]
    fn broken_repository_relative_link_fails_validation() -> Result<(), String> {
        let directory = TestDirectory::new("broken-link")?;
        directory.write("README.md", "[broken](missing.md)\n")?;

        let error = validate_markdown_links(directory.path());
        assert!(error.is_err());
        Ok(())
    }

    #[test]
    fn nested_document_links_resolve_from_the_document_directory() -> Result<(), String> {
        let directory = TestDirectory::new("nested-link")?;
        directory.write("README.md", "# Root\n")?;
        directory.write("docs/guide.md", "[root](../README.md)\n")?;

        let files = validate_markdown_links(directory.path())?;
        assert_eq!(
            files,
            [PathBuf::from("README.md"), PathBuf::from("docs/guide.md")]
        );
        Ok(())
    }

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
            local_link_path("docs/status.md#current"),
            Some("docs/status.md")
        );
    }

    #[test]
    fn repository_metadata_matches_owner_approved_gpl_license() -> Result<(), String> {
        validate_current_licensing(&workspace_root()?)
    }

    #[test]
    fn repository_metadata_rejects_stale_mit_representation() -> Result<(), String> {
        let directory = TestDirectory::new("stale-mit-license")?;
        directory.write(
            "LICENSE",
            "MIT License\n\nPermission is hereby granted, free of charge\n",
        )?;
        directory.write("LICENSING.md", "GPL-3.0-only\n")?;
        directory.write(
            "Cargo.toml",
            "[workspace.package]\nlicense = \"GPL-3.0-only\"\npublish = false\n",
        )?;

        let error = validate_current_licensing(directory.path());
        assert!(error.is_err());
        if let Err(error) = error {
            assert!(error.contains("stale MIT"));
        }
        Ok(())
    }
}
