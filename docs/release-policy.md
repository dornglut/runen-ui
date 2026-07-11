# Release Policy

> **Category: Current contract**

## Current state

All workspace packages are version `0.1.0` and set `publish = false`. No package may be published until a reviewed release-infrastructure slice deliberately changes that metadata.

## Pre-release requirements

Any future 0.x release must have a scoped changelog entry, passing `cargo validate`, reviewed package metadata/license/readmes, a clean locked dependency graph, documented API migrations, and an explicit decision about which packages are released. Tags and artifacts must be created by reviewed release automation or a documented maintainer procedure, never incidental CI source mutation.

## Stable release gate

`1.0.0` requires every M11 exit criterion: headless, standalone Windows/macOS/Linux, and embedded profiles; production controls/text/layout/accessibility; one supported conventional renderer; neutral scene protocol; deterministic public testing/replay; cross-platform CI; security/dependency/license policy enforcement; performance budgets; API/semver review; release candidates through real applications; and no unresolved P0/P1 correctness defects.

## Release checklist baseline

1. Confirm roadmap/support/status and documentation are current.
2. Confirm version, MSRV, licenses, dependencies, readmes, and changelog.
3. Run the full local/CI baseline and release-specific cross-platform, package, semver, security, license, benchmark, and stress checks required by the milestone.
4. Review generated artifacts and dry-run publication before enabling publication.
5. Create signed/annotated release metadata through the reviewed process.
6. Publish only approved packages, verify artifacts, and record follow-up or rollback guidance.

M0 establishes this policy; it does not enable or perform a release.
