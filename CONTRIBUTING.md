# Contributing to RunenUI

> **Category: Guide**

RunenUI is an experimental pre-1.0 framework undergoing foundational architecture work. Contributions should follow the current [status](docs/status-map.md), [support matrix](docs/feature-support-matrix.md), [roadmap](docs/roadmap.md), [architecture](docs/architecture.md), and [agent contract](AGENTS.md).

## Workflow

1. Discuss changes that select architecture, dependencies, public protocol shape, host/backend technology, or release policy before implementation. Required ADR topics are listed in the architecture.
2. Start a focused branch from current `master` with a clean worktree.
3. Read all affected code, tests, and authority documents; verify current behavior rather than trusting older summaries.
4. Implement one coherent slice completely, remove superseded paths, and add behavioral/conformance tests.
5. Update status, support, roadmap, architecture/ADR, public guides, and retention records when their facts change.
6. Run `cargo validate`, slice-specific checks, and `git diff --check`.
7. Open a draft pull request with the problem, target contract, scope, decisions, migrations/removals, tests, validation, non-goals, documentation changes, and next task.

## Toolchains

The repository pins the MSRV for reproducible local commands and also validates the latest stable Rust. Install/update both with `rustup`; see the [toolchain policy](docs/toolchain-policy.md).

## Change quality

- Prefer maintainable ownership and contract fixes over surface patches.
- Do not weaken production requirements to finish a pull request.
- Do not add empty crates, speculative public types, silent no-op APIs, or renderer/platform assumptions in neutral layers.
- Preserve deterministic headless behavior and strict linting.
- Treat current public APIs as breakable during 0.x, but document migrations intentionally.

By participating, contributors agree to the [Code of Conduct](CODE_OF_CONDUCT.md). Report vulnerabilities through the private process in [SECURITY.md](SECURITY.md), not a public issue.
