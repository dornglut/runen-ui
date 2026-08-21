use std::path::Path;

use super::{VolatilityPolicy, volatility_policy};

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
    ] {
        assert_eq!(
            volatility_policy(Path::new(path)),
            VolatilityPolicy::StrictCurrent
        );
    }
}
