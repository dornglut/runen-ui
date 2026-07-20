# Development and Contributions

> **Category: Current contract**

RunenUI is developed in public but remains owner-maintained during its foundational pre-1.0 work. Architecture, roadmap, implementation, release, and compatibility decisions remain under repository-owner control.

External code contributions and unsolicited pull requests are not accepted at this stage. Public visibility exists for transparency, evaluation, issue reporting, and reuse under the repository license; it does not establish an open implementation queue or a commitment to review proposed code.

## Accepted external input

The public issue tracker may be used for:

- reproducible defects in behavior that currently exists;
- documentation defects tied to current repository authority;
- narrowly scoped compatibility or build failures with sufficient evidence.

Do not use the issue tracker for vulnerability details. Follow the private process in [SECURITY.md](SECURITY.md).

Feature requests, architecture proposals, roadmap changes, implementation offers, and unsolicited pull requests may be closed without evaluation while owner-only development remains active.

## Owner development workflow

1. Start a focused branch from current `master` with a clean worktree.
2. Read all affected code, tests, and authority documents; verify current behavior rather than trusting older summaries.
3. Implement one coherent slice completely, remove superseded paths, and add behavioral or conformance tests.
4. Update status, support, roadmap, architecture or ADR, public guides, and retention records when their facts change.
5. Format intentional Rust changes with `cargo +stable fmt --all`, then run `cargo validate`, slice-specific checks, and `git diff --check`.
6. Open a draft pull request recording the problem, target contract, scope, decisions, migrations or removals, tests, validation, non-goals, documentation changes, and next task.
7. Review the exact final head before owner acceptance and squash merge.

## Toolchains

The repository pins the MSRV for reproducible default commands and also validates the latest stable Rust. Install or update both with `rustup`. Always use `cargo +stable fmt --all` for intentional formatting so local output matches the stable rustfmt enforced by validation; see the [toolchain policy](docs/toolchain-policy.md).

## Change quality

- Prefer maintainable ownership and contract fixes over surface patches.
- Do not weaken production requirements to finish a pull request.
- Do not add empty crates, speculative public types, silent no-op APIs, or renderer or platform assumptions in neutral layers.
- Preserve deterministic headless behavior and strict linting.
- Treat current public APIs as breakable during 0.x, but document migrations intentionally.

All repository interaction remains subject to the [Code of Conduct](CODE_OF_CONDUCT.md).
