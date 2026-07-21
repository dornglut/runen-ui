# Dependency Policy

> **Category: Current contract**

Dependencies are added only for a current reviewed need. A proposal must identify ownership, public API impact, feature/target behavior, MSRV, maintenance health, security history, license compatibility with MIT distribution, transitive cost, alternatives, and removal/upgrade implications.

Architecture-defining choices—layout algorithms, text stack, renderer, windowing/host adapters, accessibility bridges, async/runtime infrastructure, serialization/source formats—require the ADR or review gate named in the roadmap. No dependency may leak its tree, vocabulary, platform ownership, or renderer semantics into RunenUI’s public contract without an explicit decision.

Keep `Cargo.lock` committed and use `--locked` in validation. Version and feature changes must be intentional, minimal, reviewed in the lockfile, and reflected in policy/docs when they change support. Git dependencies and unreviewed forks require exceptional justification and a replacement plan.

Automated dependency, vulnerability, and license enforcement is an M11 release gate. Until then, maintainers review dependencies manually in every change and do not claim automated supply-chain coverage.
