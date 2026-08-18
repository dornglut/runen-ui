# Repository Audit

> **Category: Guide**

`cargo xtask audit-repository` is the deterministic, network-free repository
structure and authority audit. It reads the checked-out tree only, never modifies
files, and reports findings in stable code/path/message order.

Run the human-readable report:

```powershell
cargo xtask audit-repository
```

Run the machine-readable report:

```powershell
cargo xtask audit-repository --format json
```

A fatal finding returns a non-zero exit status. Diagnostics never change the exit
status. `cargo validate` invokes the same fatal invariant implementation after the
locked stable/MSRV baseline and relative Markdown-link validation.

The audit enforces explicit structural and modeled-authority contracts. It does
**not** infer arbitrary semantic equivalence between prose in different retained
documents. Migration/closure work therefore still requires the authority-impact
cross-document review defined by `AGENTS.md` and `docs/work-tracking.md`.

## Fatal invariants

The audit fails for:

- duplicate, malformed, invalid-status, invalid-schema, gate-policy, or
  summary-inconsistent rows in the configured M4/M5 conformance matrices,
  including permanent-ID duplication across the configured matrix set;
- workspace members missing manifests or package names, duplicate package names,
  undocumented members, documented missing members, forbidden workspace
  dependency direction, or reviewed dependency-section drift;
- public issue links outside the issue set owned by `docs/work-tracking.md`;
- active obsolete-default-branch authority outside the documented historical
  exemptions;
- private-archive URLs outside
  `docs/history/public-repository-migration.md`;
- repository/governance inventory drift, including required entry points, issue
  templates, and the accepted read-only CI workflow contract;
- MIT notice, workspace license, repository metadata, or `publish = false`
  policy drift;
- missing, relocated, or duplicate definitions of canonical runtime/source
  authorities modeled by the source audit;
- narrowly modeled retired M5 public/source authorities that would recreate a
  forbidden compatibility or parallel semantic/testing path.

The workspace dependency projection is the executable form of
`docs/architecture/workspace-structure.md`: `runenui_core` and `xtask` have no
workspace-package dependencies, `runenui_runtime` depends only on
`runenui_core`, `runenui_testing` is downstream of core/runtime, and the
Counter/downstream conformance packages follow their documented production/dev
dependency directions. A new production package is fatal until its reviewed
ownership and dependency direction are added to both the architecture contract
and audit.

Source-absence checks are deliberately structural. They are appropriate when the
absence of a retired public authority is itself the accepted contract; they are
not a substitute for behavioral/conformance tests and must not ban incidental
method names used legitimately by diagnostics, trace, or unrelated APIs.

## Diagnostics

Diagnostics identify review candidates without asserting architecture failure:

- production source modules with at least 900 lines;
- modules with at least 40 public or crate-visible item declarations;
- modules with at least 10 `pub use` statements;
- item declarations spanning at least five responsibility vocabularies;
- composite god-file candidates combining size with public-surface or
  responsibility concentration;
- test modules with at least 800 lines or 20 `#[test]` cases;
- architecture documents containing volatile branch, PR, or current-head markers.

These thresholds are triage signals only. Line count or vocabulary coincidence
must never determine correctness, extraction, or crate boundaries.

## JSON schema

Schema version `1` is a single JSON object with stable key order. The numeric
values below are illustrative schema values, **not current repository metrics**;
current counts come from the checked-out matrices/workspace at execution time.

```json
{
  "schema_version": 1,
  "status": "pass",
  "metrics": {
    "matrix": {
      "total_rows": 290,
      "owner_accepted": 285,
      "implementation_complete": 0,
      "proof_complete": 3,
      "blocked": 2
    },
    "workspace": {
      "members": 6,
      "production_crates": 2
    },
    "source": {
      "production_modules": 0,
      "test_modules": 0
    },
    "authority": {
      "files": 0,
      "modeled_public_issues": 0
    }
  },
  "findings": [
    {
      "severity": "diagnostic",
      "code": "diagnostic.large_source_module",
      "path": "crates/example/src/lib.rs",
      "message": "..."
    }
  ]
}
```

`status` is `pass` when no fatal finding exists and `fail` otherwise. `path` is a
repository-relative slash-normalized string or `null`. Consumers must branch on
`schema_version`, `severity`, and `code`; human messages may become more precise
without a schema-version change.
