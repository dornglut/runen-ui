# Dependency Policy

> **Category: Current contract**

Dependencies are added only for a current reviewed need. A proposal must identify ownership, public API impact, feature/target behavior, MSRV, maintenance health, security history, actual third-party license terms and obligations for RunenUI's current public `GPL-3.0-only` distribution, transitive cost, alternatives, and removal/upgrade implications.

For public distribution, assess each dependency and incorporated third-party work under its actual license and applicable conditions, including obligations arising from redistribution. Compatibility with RunenUI's public GPL distribution is necessary, but it does not change or replace any third-party license. See [LICENSING.md](../LICENSING.md) for RunenUI's public license and separately authorized commercial-licensing model.

For any intended separate commercial distribution involving third-party dependencies or material, independently assess whether the actual third-party terms can be satisfied and whether any additional permissions or relicensing rights needed for that distribution exist. Public GPL compatibility alone does **not** confer permission to commercially relicense third-party works. Where commercial compatibility or required rights cannot be established from accepted policy and the actual terms, require explicit legal/business review before approving that use.

Architecture-defining choices—layout algorithms, text stack, renderer, windowing/host adapters, accessibility bridges, async/runtime infrastructure, serialization/source formats—require the ADR or review gate named in the roadmap. No dependency may leak its tree, vocabulary, platform ownership, or renderer semantics into RunenUI’s public contract without an explicit decision.

Keep `Cargo.lock` committed and use `--locked` in validation. Version and feature changes must be intentional, minimal, reviewed in the lockfile, and reflected in policy/docs when they change support. Git dependencies and unreviewed forks require exceptional justification and a replacement plan.

Automated dependency, vulnerability, and license enforcement belongs to the **M15 feature-complete 0.1 release-qualification gate** in the [roadmap](roadmap.md). Until that enforcement is accepted, maintainers manually review dependency and license/security implications of every change affecting dependencies, including transitive changes, and must not claim automated supply-chain coverage.
