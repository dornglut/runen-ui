# Release Policy

> **Category: Current contract**

## Current state

All workspace packages are pre-1.0 and publication remains disabled. No package may be published until a reviewed release-infrastructure slice deliberately changes repository/package metadata and adds the required release validation.

## Pre-release requirements

Any future 0.x release requires a scoped changelog/release note, passing `cargo validate`, reviewed package metadata/license/readmes, a clean locked dependency graph, documented public migrations, and an explicit decision about which packages are released. Tags and artifacts must be created by reviewed release automation or a documented maintainer procedure, never incidental CI source mutation.

## Stable release gate

`1.0.0` requires the production-hardening outcome in the [roadmap](roadmap.md): supported headless, standalone desktop, and embedded profiles; production controls/text/layout/accessibility; one supported conventional renderer; neutral scene protocol; deterministic public testing/replay; cross-platform CI; security/dependency/license policy enforcement; performance budgets; API/semver review; release candidates through real applications; and no unresolved release-blocking correctness defects.

## Release checklist baseline

1. Confirm the roadmap, current status, conformance state, and affected documentation are coherent with accepted default-branch behavior.
2. Confirm version, MSRV, licenses, dependencies, package readmes, and changelog/release notes.
3. Run the canonical validation baseline plus the release-specific cross-platform, package, semver, security, license, benchmark, stress, and compatibility checks required by the release gate.
4. Review generated artifacts and dry-run publication before enabling publication.
5. Create signed or otherwise deliberately authenticated release metadata through the reviewed process.
6. Publish only approved packages, verify the released artifacts, and record follow-up or rollback guidance where required.

Release process state and exact artifact/CI evidence belong in the release issue/pull request and release system, not this durable policy document.
