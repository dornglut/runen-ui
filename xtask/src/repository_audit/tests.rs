use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::{
    AuditReport, OutputFormat, PRIVATE_ARCHIVE_URL, Severity, VolatilityPolicy, audit_volatility,
    build_report, contains_full_sha, expected_ci_workflow, json_escape, parse_output_format,
};

static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Result<Self, String> {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "runenui-repository-audit-{label}-{}-{sequence}",
            process::id()
        ));
        fs::create_dir_all(&path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
        let fixture = Self { path };
        fixture.write_baseline()?;
        Ok(fixture)
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

    fn remove(&self, relative: &str) -> Result<(), String> {
        fs::remove_file(self.path.join(relative))
            .map_err(|error| format!("failed to remove {relative}: {error}"))
    }

    fn write_baseline(&self) -> Result<(), String> {
        self.write_repository_baseline()?;
        self.write_documentation_baseline()?;
        self.write_github_baseline()?;
        self.write_context_baseline()?;
        self.write_workspace_baseline()?;
        self.write_conformance_baseline()
    }

    fn write_repository_baseline(&self) -> Result<(), String> {
        self.write(
            "Cargo.toml",
            "[workspace]\nmembers = [\n  \"crates/runenui_core\",\n  \"crates/runenui_runtime\",\n  \"xtask\",\n]\n\n[workspace.package]\nrepository = \"https://github.com/dornglut/runen-ui\"\nlicense = \"MIT\"\npublish = false\n",
        )?;
        self.write(
            "LICENSE",
            "MIT License\n\nCopyright (c) 2026 Crystonix\n\nPermission is hereby granted, free of charge\nTHE SOFTWARE IS PROVIDED \"AS IS\"\n",
        )
    }

    fn write_documentation_baseline(&self) -> Result<(), String> {
        for (path, contents) in [
            ("README.md", "# RunenUI\n"),
            ("AGENTS.md", "Start from accepted `main`.\n"),
            ("ARCHITECTURE.md", "# Architecture\n"),
            ("TESTING.md", "# Testing\n"),
            ("docs/README.md", "# Docs\n"),
            (
                "docs/documentation-architecture.md",
                "# Documentation architecture\n",
            ),
            ("docs/roadmap.md", "# Roadmap\n"),
            ("docs/status.md", "# Status\n"),
            ("docs/architecture/README.md", "# Architecture\n"),
            ("docs/architecture/public-api.md", "# Public API\n"),
            ("docs/conformance/README.md", "# Conformance\n"),
        ] {
            self.write(path, contents)?;
        }
        self.write(
            "docs/architecture/workspace-structure.md",
            "| Package | Current ownership | Must not own |\n|---|---|---|\n| `runenui_core` | Core | Runtime |\n| `runenui_runtime` | Runtime | Platform |\n| `xtask` | Tooling | Runtime |\n",
        )?;
        self.write(
            "docs/history/public-repository-migration.md",
            &format!("https://{PRIVATE_ARCHIVE_URL}\n"),
        )
    }

    fn write_github_baseline(&self) -> Result<(), String> {
        self.write(
            ".github/ISSUE_TEMPLATE/config.yml",
            "blank_issues_enabled: false\ncontact_links:\n  - name: Security\n    url: https://github.com/dornglut/runen-ui/security/policy\n    about: Private security reporting.\n  - name: Engineering\n    url: https://github.com/dornglut/engineering/issues/new\n    about: Cross-repository decisions.\n",
        )?;
        for (path, contents) in [
            (".github/ISSUE_TEMPLATE/defect.yml", "name: Defect\n"),
            (
                ".github/ISSUE_TEMPLATE/milestone-slice.yml",
                "name: Milestone slice\n",
            ),
            (".github/ISSUE_TEMPLATE/proposal.yml", "name: Proposal\n"),
        ] {
            self.write(path, contents)?;
        }
        self.write(".github/workflows/ci.yml", &expected_ci_workflow())
    }

    fn write_context_baseline(&self) -> Result<(), String> {
        self.write(
            "tools/context/export_repo_context.py",
            "DEFAULT_PROFILE = \"offline-review\"\n",
        )?;
        for profile in [
            "full-audit.toml",
            "implementation-review.toml",
            "offline-review.toml",
        ] {
            self.write(
                &format!("tools/context/profiles/{profile}"),
                "description = \"offline review\"\ninclude = [\"README.md\"]\n",
            )?;
        }
        Ok(())
    }

    fn write_workspace_baseline(&self) -> Result<(), String> {
        for (path, contents) in [
            (
                "crates/runenui_core/Cargo.toml",
                "[package]\nname = \"runenui_core\"\n\n[dependencies]\n",
            ),
            (
                "crates/runenui_core/src/lib.rs",
                "#![forbid(unsafe_code)]\n",
            ),
            (
                "crates/runenui_runtime/Cargo.toml",
                "[package]\nname = \"runenui_runtime\"\n\n[dependencies]\nrunenui_core = { path = \"../runenui_core\" }\n",
            ),
            (
                "crates/runenui_runtime/src/queue.rs",
                "pub(crate) struct WorkQueue<Action> { value: Option<Action> }\n",
            ),
            (
                "crates/runenui_runtime/src/trace/store.rs",
                "pub struct Trace;\n",
            ),
            (
                "crates/runenui_runtime/src/runtime/surface_publication.rs",
                "pub(crate) struct SurfacePublicationState;\n",
            ),
            (
                "xtask/Cargo.toml",
                "[package]\nname = \"xtask\"\n\n[dependencies]\n",
            ),
            ("xtask/src/main.rs", "fn main() {}\n"),
        ] {
            self.write(path, contents)?;
        }
        Ok(())
    }

    fn write_conformance_baseline(&self) -> Result<(), String> {
        for (path, id, slice) in [
            (
                "docs/conformance/m4-conformance-matrix.md",
                "PTR-01",
                "M4C3",
            ),
            (
                "docs/conformance/m5-conformance-matrix.md",
                "SEM-ID-01",
                "M5A",
            ),
            (
                "docs/conformance/m6-conformance-matrix.md",
                "SCENE-PUB-01",
                "M6A",
            ),
        ] {
            self.write(
                path,
                &format!(
                    "```text\n1 total unique rows\n0 owner-accepted\n0 implementation-complete\n0 proof-complete\n1 blocked\n0 duplicate IDs\n0 invalid statuses\n0 invalid schemas\n```\n\n| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |\n|---|---|---|---|---|---|---|---|\n| {id} | observation | positive | negative | diagnostic | {slice} | blocked | Required |\n"
                ),
            )?;
        }
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn assert_fatal_code(report: &AuditReport, expected: &str) {
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == expected),
        "expected fatal code {expected:?}, got {:?}",
        report
            .findings
            .iter()
            .map(|finding| finding.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn output_format_contract_is_exact() {
    assert_eq!(
        parse_output_format(std::iter::empty()),
        Ok(OutputFormat::Human)
    );
    assert_eq!(
        parse_output_format(["--format".to_owned(), "json".to_owned()].into_iter()),
        Ok(OutputFormat::Json)
    );
    assert!(parse_output_format(["--json".to_owned()].into_iter()).is_err());
}

#[test]
fn sha_detector_requires_an_exact_hex_run() {
    assert!(contains_full_sha(
        "x 0123456789abcdef0123456789abcdef01234567 y"
    ));
    assert!(contains_full_sha(
        "x 0123456789ABCDEF0123456789ABCDEF01234567 y"
    ));
    assert!(!contains_full_sha(
        "0123456789abcdef0123456789abcdef0123456"
    ));
}

#[test]
fn strict_current_policy_rejects_live_issue_sha_and_run_marker() {
    let mut findings = Vec::new();
    audit_volatility(
        "docs/architecture/example.md",
        "https://github.com/dornglut/runen-ui/issues/999 accepted 0123456789abcdef0123456789abcdef01234567 CI #123",
        VolatilityPolicy::StrictCurrent,
        &mut findings,
    );
    assert_eq!(findings.len(), 3);
    assert!(
        findings
            .iter()
            .all(|finding| finding.severity == Severity::Fatal)
    );
}

#[test]
fn frozen_contract_allows_accepted_provenance_but_rejects_mutable_head_state() {
    let mut findings = Vec::new();
    audit_volatility(
        "docs/conformance/example.md",
        "accepted 0123456789abcdef0123456789abcdef01234567 via https://github.com/dornglut/runen-ui/pull/1 and CI #42",
        VolatilityPolicy::FrozenContract,
        &mut findings,
    );
    assert!(findings.is_empty());

    audit_volatility(
        "docs/conformance/example.md",
        "Current head: 0123456789abcdef0123456789abcdef01234567",
        VolatilityPolicy::FrozenContract,
        &mut findings,
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.code == "authority.volatile_execution_marker")
    );
}

#[test]
fn baseline_fixture_passes_and_json_is_deterministic() -> Result<(), String> {
    let fixture = Fixture::new("baseline")?;
    let first = build_report(fixture.path())?;
    let second = build_report(fixture.path())?;
    assert!(first.is_success(), "{}", first.render_failure());
    assert_eq!(first.render_json(), second.render_json());
    Ok(())
}

#[test]
fn missing_required_entrypoint_is_fatal() -> Result<(), String> {
    let fixture = Fixture::new("missing-entrypoint")?;
    fixture.remove("docs/status.md")?;
    let report = build_report(fixture.path())?;
    assert_fatal_code(&report, "repository.required_entrypoint_missing");
    Ok(())
}

#[test]
fn retired_authority_path_reappearance_is_fatal() -> Result<(), String> {
    let fixture = Fixture::new("retired-path")?;
    fixture.write("docs/work-tracking.md", "# retired\n")?;
    let report = build_report(fixture.path())?;
    assert_fatal_code(&report, "authority.retired_path_present");
    Ok(())
}

#[test]
fn new_current_architecture_document_cannot_bypass_volatility_audit() -> Result<(), String> {
    let fixture = Fixture::new("volatile-current")?;
    fixture.write(
        "docs/architecture/current.md",
        "Current feature head: 0123456789abcdef0123456789abcdef01234567\n",
    )?;
    let report = build_report(fixture.path())?;
    assert_fatal_code(&report, "authority.volatile_execution_marker");
    assert_fatal_code(&report, "authority.volatile_commit_sha");
    Ok(())
}

#[test]
fn crate_readme_is_strict_current_without_an_allowlist_entry() -> Result<(), String> {
    let fixture = Fixture::new("crate-readme")?;
    fixture.write(
        "crates/runenui_core/README.md",
        "https://github.com/dornglut/runen-ui/pull/999\n",
    )?;
    let report = build_report(fixture.path())?;
    assert_fatal_code(&report, "authority.volatile_github_state");
    Ok(())
}

#[test]
fn provenance_document_may_retain_historical_sha() -> Result<(), String> {
    let fixture = Fixture::new("provenance")?;
    fixture.write(
        "docs/reports/example.md",
        "accepted 0123456789abcdef0123456789abcdef01234567\n",
    )?;
    let report = build_report(fixture.path())?;
    assert!(report.is_success(), "{}", report.render_failure());
    Ok(())
}

#[test]
fn ci_contract_drift_is_fatal() -> Result<(), String> {
    let fixture = Fixture::new("ci-drift")?;
    fixture.write(".github/workflows/ci.yml", "name: Other\n")?;
    let report = build_report(fixture.path())?;
    assert_fatal_code(&report, "repository.workflow_contract");
    Ok(())
}

#[test]
fn context_default_profile_drift_is_fatal() -> Result<(), String> {
    let fixture = Fixture::new("context-default")?;
    fixture.write(
        "tools/context/export_repo_context.py",
        "DEFAULT_PROFILE = \"ai-core\"\n",
    )?;
    let report = build_report(fixture.path())?;
    assert_fatal_code(&report, "context.default_profile");
    Ok(())
}

#[test]
fn context_profile_inventory_drift_is_fatal() -> Result<(), String> {
    let fixture = Fixture::new("context-inventory")?;
    fixture.write(
        "tools/context/profiles/current-work.toml",
        "description = \"legacy current work\"\ninclude = [\"README.md\"]\n",
    )?;
    let report = build_report(fixture.path())?;
    assert_fatal_code(&report, "context.profile_inventory");
    Ok(())
}

#[test]
fn finding_order_remains_stable() {
    let mut findings = [
        super::Finding::diagnostic("z", None, "z"),
        super::Finding::fatal("a", None, "a"),
    ];
    findings.sort();
    assert_eq!(findings[0].severity, Severity::Fatal);
}

#[test]
fn json_escape_handles_control_characters() {
    assert_eq!(json_escape("a\n\"b\\c"), "a\\n\\\"b\\\\c");
}
