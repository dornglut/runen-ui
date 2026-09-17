//! Cargo-level public-consumer feature-isolation proof.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{self, Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

const PUBLIC_PACKAGES: &[&str] = &[
    "runenui_testing",
    "runenui_external_widget_conformance",
    "runenui_external_renderer_conformance",
    "runenui_external_host_conformance",
];
const PRIVATE_FEATURE: &str = "internal-test-seams";
const PRIVATE_METHOD: &str = "__seed_next_work_sequence_for_test";
const PROBE_MANIFEST: &str = "[package]\nname = \"runenui-public-feature-probe\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[workspace]\nresolver = \"3\"\n\n[dependencies]\nrunenui_runtime = { path = \"../../crates/runenui_runtime\" }\nrunenui_core = { path = \"../../crates/runenui_core\" }\n\n[features]\nseam-enabled = [\"runenui_runtime/internal-test-seams\"]\n";
const PROBE_SOURCE: &str = "use runenui_core::UiApp;\nuse runenui_runtime::AppRuntime;\n\npub fn probe<App: UiApp>(runtime: &mut AppRuntime<App>) {\n    runtime.__seed_next_work_sequence_for_test(1);\n}\n";
static NEXT_PROBE: AtomicUsize = AtomicUsize::new(0);

pub fn validate(root: &Path) -> Result<(), String> {
    let arguments = public_test_arguments();
    super::run_cargo_step(root, "stable", &arguments)?;
    validate_public_feature_graph(root)?;
    validate_private_seam_isolation(root)
}

fn public_test_arguments() -> Vec<&'static str> {
    let mut arguments = vec!["test", "--locked"];
    for package in PUBLIC_PACKAGES {
        arguments.extend(["--package", package]);
    }
    arguments
}

fn validate_public_feature_graph(root: &Path) -> Result<(), String> {
    let mut arguments = vec!["tree", "--locked", "--offline", "--edges", "features"];
    for package in PUBLIC_PACKAGES {
        arguments.extend(["--package", package]);
    }
    eprintln!("> cargo +stable {}", arguments.join(" "));
    let output = Command::new("rustup")
        .args(["run", "stable", "cargo"])
        .args(&arguments)
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to inspect public-consumer Cargo features: {error}"))?;
    require_success("inspect public-consumer Cargo feature graph", &output)?;
    let graph = String::from_utf8_lossy(&output.stdout);
    if graph.contains(PRIVATE_FEATURE) {
        return Err(format!(
            "public-consumer Cargo feature graph activates `{PRIVATE_FEATURE}`:\n{graph}"
        ));
    }
    eprintln!("> public-consumer Cargo feature graph excludes `{PRIVATE_FEATURE}`");
    Ok(())
}

struct ProbeDirectory(PathBuf);

impl ProbeDirectory {
    fn new(root: &Path) -> Result<Self, String> {
        let directory = root.join("target").join(format!(
            "runenui-public-feature-probe-{}-{}",
            process::id(),
            NEXT_PROBE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory)
            .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;
        Ok(Self(directory))
    }

    fn manifest(&self) -> PathBuf {
        self.0.join("Cargo.toml")
    }
}

impl Drop for ProbeDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn validate_private_seam_isolation(root: &Path) -> Result<(), String> {
    let probe = ProbeDirectory::new(root)?;
    fs::create_dir(probe.0.join("src"))
        .map_err(|error| format!("failed to create probe source directory: {error}"))?;
    fs::write(probe.manifest(), PROBE_MANIFEST)
        .map_err(|error| format!("failed to write probe manifest: {error}"))?;
    fs::write(probe.0.join("src/lib.rs"), PROBE_SOURCE)
        .map_err(|error| format!("failed to write probe source: {error}"))?;
    fs::copy(root.join("Cargo.lock"), probe.0.join("Cargo.lock"))
        .map_err(|error| format!("failed to copy probe lockfile: {error}"))?;

    // Update only the disposable copy of the workspace lockfile for its probe package.
    // Dependency resolution is offline; both actual compiler checks are locked.
    let prepared = run_probe_cargo(
        root,
        &probe.manifest(),
        &[
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--no-deps",
        ],
    )?;
    require_success("prepare offline probe lockfile", &prepared)?;

    let enabled = run_probe_cargo(
        root,
        &probe.manifest(),
        &[
            "check",
            "--lib",
            "--offline",
            "--locked",
            "--features",
            "seam-enabled",
        ],
    )?;
    require_success("compile positive feature-enabled seam control", &enabled)?;

    let disabled = run_probe_cargo(
        root,
        &probe.manifest(),
        &["check", "--lib", "--offline", "--locked"],
    )?;
    let diagnostics = String::from_utf8_lossy(&disabled.stderr);
    if disabled.status.success() || !is_expected_private_seam_rejection(&diagnostics) {
        return Err(format!(
            "public/default-feature seam probe did not reject the private method as expected (status {}):\n{diagnostics}",
            disabled.status
        ));
    }
    eprintln!("> public/default-feature probe rejected `{PRIVATE_METHOD}` as expected");
    Ok(())
}

fn run_probe_cargo(root: &Path, manifest: &Path, arguments: &[&str]) -> Result<Output, String> {
    eprintln!(
        "> cargo +stable {} --manifest-path {}",
        arguments.join(" "),
        manifest.display()
    );
    Command::new("rustup")
        .args(["run", "stable", "cargo"])
        .args(arguments)
        .arg("--manifest-path")
        .arg(manifest)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to execute public feature probe: {error}"))
}

fn require_success(label: &str, output: &Output) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{label} failed (status {}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn is_expected_private_seam_rejection(diagnostics: &str) -> bool {
    diagnostics.contains("no method named") && diagnostics.contains(PRIVATE_METHOD)
}

#[cfg(test)]
mod tests {
    use super::{is_expected_private_seam_rejection, public_test_arguments};

    #[test]
    fn public_lane_selects_only_downstream_packages_without_all_features() {
        assert_eq!(
            public_test_arguments(),
            [
                "test",
                "--locked",
                "--package",
                "runenui_testing",
                "--package",
                "runenui_external_widget_conformance",
                "--package",
                "runenui_external_renderer_conformance",
                "--package",
                "runenui_external_host_conformance",
            ]
        );
    }

    #[test]
    fn negative_probe_rejects_only_the_known_private_method_diagnostic() {
        assert!(is_expected_private_seam_rejection(
            "error[E0599]: no method named `__seed_next_work_sequence_for_test` found"
        ));
        assert!(!is_expected_private_seam_rejection(
            "error: dependency unavailable"
        ));
        assert!(!is_expected_private_seam_rejection(
            "error[E0599]: no method named `other_method` found"
        ));
    }
}
