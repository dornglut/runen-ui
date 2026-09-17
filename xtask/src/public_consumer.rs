//! Cargo-level public-consumer feature-isolation proof.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{self, Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use super::repository_audit::{PublicConsumerPolicy, public_consumer_policy};

const PROBED_PRIVATE_FEATURE: &str = "internal-test-seams";
const PRIVATE_METHOD: &str = "__seed_next_work_sequence_for_test";
const PROBE_MANIFEST: &str = "[package]\nname = \"runenui-public-feature-probe\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[workspace]\nresolver = \"3\"\n\n[dependencies]\nrunenui_runtime = { path = \"../../crates/runenui_runtime\" }\nrunenui_core = { path = \"../../crates/runenui_core\" }\n\n[features]\nseam-enabled = [\"runenui_runtime/internal-test-seams\"]\n";
const PROBE_SOURCE: &str = "use runenui_core::UiApp;\nuse runenui_runtime::AppRuntime;\n\npub fn probe<App: UiApp>(runtime: &mut AppRuntime<App>) {\n    runtime.__seed_next_work_sequence_for_test(1);\n}\n";
static NEXT_PROBE: AtomicUsize = AtomicUsize::new(0);

pub fn validate(root: &Path) -> Result<(), String> {
    let policy = public_consumer_policy(root)?;
    if !policy
        .private_features
        .iter()
        .any(|feature| feature == PROBED_PRIVATE_FEATURE)
    {
        return Err(format!(
            "private seam probe feature `{PROBED_PRIVATE_FEATURE}` is missing from the workspace private-feature inventory"
        ));
    }
    let arguments = public_test_arguments(&policy.packages);
    let argument_refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    super::run_cargo_step(root, "stable", &argument_refs)?;
    validate_public_feature_graph(root, &policy)?;
    validate_private_seam_isolation(root)
}

fn public_test_arguments(packages: &[String]) -> Vec<String> {
    let mut arguments = vec!["test".to_owned(), "--locked".to_owned()];
    for package in packages {
        arguments.extend(["--package".to_owned(), package.clone()]);
    }
    arguments
}

fn validate_public_feature_graph(root: &Path, policy: &PublicConsumerPolicy) -> Result<(), String> {
    let mut arguments = vec!["tree", "--locked", "--offline", "--edges", "features"];
    for package in &policy.packages {
        arguments.extend(["--package", package.as_str()]);
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
    for feature in &policy.private_features {
        if graph.contains(feature) {
            return Err(format!(
                "public-consumer Cargo feature graph activates private feature `{feature}`:\n{graph}"
            ));
        }
    }
    eprintln!(
        "> public-consumer Cargo feature graph excludes private features: {}",
        policy.private_features.join(", ")
    );
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
        &["metadata", "--offline", "--format-version", "1"],
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
        let packages = [
            "runenui_external_host_conformance".to_owned(),
            "runenui_testing".to_owned(),
        ];
        assert_eq!(
            public_test_arguments(&packages),
            [
                "test",
                "--locked",
                "--package",
                "runenui_external_host_conformance",
                "--package",
                "runenui_testing",
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
