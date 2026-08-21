use std::path::Path;

use super::{VolatilityPolicy, audit_volatility, volatility_policy};

#[test]
fn volatility_policy_is_derived_from_artifact_class() {
    for path in [
        "docs/adr/0007-renderer-neutral-paint-hit-scene-protocol.md",
        "docs/design/example.md",
        "docs/conformance/m6-conformance-matrix.md",
    ] {
        assert_eq!(
            volatility_policy(Path::new(path)),
            VolatilityPolicy::FrozenContract
        );
    }

    for path in [
        "CHANGELOG.md",
        "docs/history/public-repository-migration.md",
        "docs/reports/example.md",
        ".github/workflows/ci.yml",
    ] {
        assert_eq!(
            volatility_policy(Path::new(path)),
            VolatilityPolicy::Provenance
        );
    }

    for path in [
        "README.md",
        "docs/architecture/public-api.md",
        "docs/tooling/validation.md",
        "tools/context/README.md",
        "crates/runenui_core/README.md",
        ".github/pull_request_template.md",
        ".github/ISSUE_TEMPLATE/milestone-slice.yml",
    ] {
        assert_eq!(
            volatility_policy(Path::new(path)),
            VolatilityPolicy::StrictCurrent
        );
    }
}

#[test]
fn current_process_template_allows_empty_review_evidence_prompt() {
    let path = ".github/pull_request_template.md";
    let mut findings = Vec::new();
    audit_volatility(
        path,
        "- Reviewed exact head:\n",
        volatility_policy(Path::new(path)),
        &mut findings,
    );
    assert!(findings.is_empty());
}

#[test]
fn current_process_template_rejects_hard_coded_live_state() {
    let path = ".github/pull_request_template.md";
    let mut findings = Vec::new();
    audit_volatility(
        path,
        "https://github.com/dornglut/runen-ui/pull/79\ncurrent head: 0123456789abcdef0123456789abcdef01234567\n",
        volatility_policy(Path::new(path)),
        &mut findings,
    );

    assert!(findings.iter().any(|finding| {
        finding.code == "authority.volatile_github_state"
            && finding.severity == super::Severity::Fatal
    }));
    assert!(findings.iter().any(|finding| {
        finding.code == "authority.volatile_commit_sha"
            && finding.severity == super::Severity::Fatal
    }));
    assert!(findings.iter().any(|finding| {
        finding.code == "authority.volatile_execution_marker"
            && finding.severity == super::Severity::Fatal
    }));
}
