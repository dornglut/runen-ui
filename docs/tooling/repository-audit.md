# Repository Audit

`cargo xtask audit-repository` is RunenUI's deterministic, network-free repository structure and authority audit. It reads the checked-out tree only, never modifies files, and reports findings in stable order.

```text
cargo xtask audit-repository
cargo xtask audit-repository --format json
```

Fatal findings return non-zero. Diagnostics are informational. The fatal audit is also part of `cargo validate`.

The audit enforces **structural authority boundaries**. It does not claim to prove arbitrary semantic equivalence between prose documents; human review still verifies that the canonical owner contains the correct rule.

## Fatal invariants

The audit fails closed for material repository-contract violations including:

- required root/documentation entrypoints missing;
- retired duplicate-authority paths reintroduced;
- malformed, duplicate, invalid-status/schema, gate-policy, or summary-inconsistent conformance rows;
- workspace members/package names/dependency direction drifting from the documented workspace contract;
- forbidden or missing source authorities covered by the source-ownership audit;
- repository metadata/license/publication-policy drift;
- issue-template or admitted read-only workflow inventory/contract drift;
- retained context-export profile inventory/default/bounded-profile drift;
- active historical repository/default-branch identities outside explicit history/report exemptions;
- private archive references outside the migration-history owner;
- volatile live-work evidence embedded in current durable documentation or retained context-profile configuration.

The documentation audit derives volatility policy from artifact class and repository location instead of maintaining a filename allowlist:

- ordinary root, crate, example, test, policy, status, roadmap, architecture, tooling, context-guide, pull-request-template, issue-template, and retained context-profile material is **strict current authority** and must not hard-code live issue/PR/run URLs, full commit SHAs, current heads/branches/blockers, or pickup state;
- accepted ADR/design/conformance material is **frozen contract authority**: immutable acceptance provenance may remain where it explains the contract, but mutable current-head/branch/blocker/pickup markers are forbidden;
- changelog, history, and report material is **provenance authority** and may preserve point-in-time revision evidence;
- `.github/workflows/` is exempt from documentation-volatility rules because admitted workflows require immutable Action/reusable-workflow revisions and are independently enforced by the workflow-contract audit.

The retained context exporter has a separate structural guard: exactly `offline-review`, `implementation-review`, and `full-audit` profiles are permitted; `offline-review` must remain the default; every retained profile is volatility-audited as strict-current configuration; and the two bounded profiles must exclude `Cargo.lock`, `docs/history/**`, `docs/reports/**`, and `legacy/**`. `full-audit` remains the intentionally comprehensive profile. The Python profile tests remain tool-specific behavioral checks; they are not made a prerequisite for Rust validation merely because the optional exporter exists.

This protects the single-owner model without erasing useful historical evidence. GitHub owns live work and delivery state; durable documentation and active repository templates own contracts, architecture, policy, accepted capability truth, and reusable process prompts—not copied execution state.

## Conformance audit

Configured M4–M9 matrices are loaded from `docs/conformance/`. The audit fails closed if the indexed, registered, and on-disk matrix inventories disagree. It validates:

- permanent ID format and uniqueness across all configured matrices;
- exact row schema;
- allowed accepted-state vocabulary;
- milestone-specific delivery-slice/gate policy;
- declared summary counts versus parsed rows.

Matrix status describes accepted default-branch conformance state. The audit does not model in-flight GitHub issue/PR state or infer semantic correctness from structural validity.

## Workspace and source audit

Workspace checks enforce the accepted package inventory and dependency direction documented in [workspace structure](../architecture/workspace-structure.md). Source checks enforce narrowly modeled canonical runtime/source authorities and retired-path absence where the absence itself is an accepted architecture contract.

File size, public-item count, responsibility vocabulary, and similar concentration checks are diagnostics only. They identify review candidates and must not decide crate boundaries or correctness by themselves.

## CI contract

The active workflow inventory remains intentionally small and fail-closed.

- `.github/workflows/ci.yml` remains the ordinary repository baseline: read-only, triggered on pull requests and pushes to `main`, and exactly invokes the accepted immutable reusable `cargo validate` workflow without product-specific steps, secrets, or source mutation.
- When the retained real-renderer visual proof exists, `.github/workflows/visual-conformance.yml` is additionally required as the one specialized visual-evidence boundary. It is read-only, proves the exact checked-out revision, provisions only the deterministic native renderer environment, runs repository-owned real-wgpu proof tests, requires the fixed human-review evidence files, verifies tracked-state hygiene, and uploads only that fixed runner-temporary evidence directory with an immutable Action revision and bounded retention.

The specialized workflow supplements `cargo validate`; it does not redefine the repository baseline, move visual generation into CI authority, permit arbitrary extra workflows, mutate source, or create a second runtime/renderer correctness model. The executable audit validates the exact admitted workflow bodies and rejects inventory drift.

## JSON schema

Schema version `2` reports stable matrix/workspace/source/authority metrics and findings. Numeric values below illustrate shape only:

```json
{
  "schema_version": 2,
  "status": "pass",
  "metrics": {
    "matrix": {
      "total_rows": 0,
      "owner_accepted": 0,
      "implementation_complete": 0,
      "proof_complete": 0,
      "blocked": 0
    },
    "workspace": {
      "members": 0,
      "production_crates": 0
    },
    "source": {
      "production_modules": 0,
      "test_modules": 0
    },
    "authority": {
      "files": 0
    }
  },
  "findings": []
}
```

Consumers branch on `schema_version`, finding `severity`, and finding `code`; human-readable messages may become more precise without changing the schema version.
