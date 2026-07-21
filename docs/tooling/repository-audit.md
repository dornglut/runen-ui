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

## Fatal invariants

The audit fails for:

- duplicate, malformed, invalid-status, or summary-inconsistent M4 conformance
  matrix rows;
- workspace members missing manifests or package names, duplicate package names,
  undocumented members, documented missing members, or forbidden workspace
  dependency direction;
- public issue links outside the issue set owned by `docs/work-tracking.md`;
- active obsolete-default-branch authority outside `CHANGELOG.md` and `docs/history/`;
- private-archive URLs outside
  `docs/history/public-repository-migration.md`;
- MIT notice, workspace license, or `publish = false` policy drift;
- missing, relocated, or duplicate definitions of the canonical `WorkQueue`,
  `Trace`, or runtime `SurfacePublicationState` authorities.

The workspace dependency projection is the executable form of
`docs/architecture/workspace-structure.md`: `runenui_core` and `xtask` have no
workspace-package dependencies, `runenui_runtime` depends only on
`runenui_core`, and the Counter/downstream conformance packages may depend on
core and runtime. A new production package is fatal until its reviewed ownership
and dependency direction are added to both the architecture contract and audit.

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

Schema version `1` is a single JSON object with stable key order:

```json
{
  "schema_version": 1,
  "status": "pass",
  "metrics": {
    "matrix": {
      "total_rows": 237,
      "owner_accepted": 132,
      "implementation_complete": 0,
      "proof_complete": 0,
      "blocked": 105
    },
    "workspace": {
      "members": 5,
      "production_crates": 2
    },
    "source": {
      "production_modules": 0,
      "test_modules": 0
    },
    "authority": {
      "files": 0,
      "modeled_public_issues": 11
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
