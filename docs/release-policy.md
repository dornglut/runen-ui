# Release Policy

> **Category: Current contract**

## Current state

All workspace packages are pre-1.0 and publication remains disabled. No package may be published until a reviewed release-infrastructure slice deliberately changes repository/package metadata and adds the required release validation.

Repository package versions used during unpublished development are not release claims by themselves.

## First public release gate

The first public RunenUI package release is reserved for `0.1.0` after the feature-complete qualification outcome in the [roadmap](roadmap.md).

`0.1.0` requires the declared supported headless, standalone desktop, embedded application, and real-time/game profiles to be feature-complete at the framework level. Required foundations include production layout/style/text/editing/interaction, animation, accessibility, standard controls, virtualization, renderer/resource integration, supported platform hosts, deterministic testing/replay, inspectability, performance budgets, and representative real applications. Specialist products and optional platform families that are explicitly outside the roadmap's 0.1 gate do not block release.

No earlier public 0.x package release is used as an unfinished-foundation preview. Internal/unpublished development remains free to use ordinary repository metadata and clean pre-1.0 cutovers.

## 0.x evolution after 0.1

After `0.1.0`, subsequent `0.x` releases may make reviewed breaking changes when they materially improve correctness, ownership, extensibility, ergonomics, performance, or long-term API quality. Such releases require explicit migrations where downstream users are affected.

Each public 0.x release requires a scoped changelog/release note, passing `cargo validate`, reviewed package metadata/license/readmes, a clean locked dependency graph, documented public migrations, and an explicit decision about which packages are released. Tags and artifacts must be created by reviewed release automation or a documented maintainer procedure, never incidental CI source mutation.

## Stable release gate

`1.0.0` is a compatibility and support commitment over an already feature-complete product, not the milestone where missing foundational UI systems first become available.

Stable release requires successful real-world 0.x use, supported production profiles, a reviewed stable public API/semver and deprecation strategy, compatibility and release automation, documented platform/MSRV/support policy, security/dependency/license enforcement, sustained performance budgets, migration policy, real downstream application evidence, and no unresolved stable-release correctness or compatibility blockers. See the [roadmap](roadmap.md) and [API stability policy](api-stability.md).

## Release checklist baseline

1. Confirm the roadmap, current status, conformance state, and affected documentation are coherent with accepted default-branch behavior.
2. Confirm the target release satisfies its roadmap gate: feature-complete profile qualification for `0.1.0`, or the additional compatibility/support gate for `1.0.0`.
3. Confirm version, MSRV, licenses, dependencies, package readmes, changelog/release notes, and documented migrations where applicable.
4. Run the canonical validation baseline plus the release-specific cross-platform, package, semver, security, license, benchmark, stress, compatibility, and real-application checks required by the target release gate.
5. Review generated artifacts and dry-run publication before enabling publication.
6. Create signed or otherwise deliberately authenticated release metadata through the reviewed process.
7. Publish only approved packages, verify the released artifacts, and record follow-up or rollback guidance where required.

Release process state and exact artifact/CI evidence belong in the release issue/pull request and release system, not this durable policy document.
