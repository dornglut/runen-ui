# Context Export

> **Category: Guide**

`export_repo_context.py` is optional convenience tooling for offline/manual review or environments that cannot inspect the repository directly. It is **not** repository authority and is not the normal contributor/agent startup path. Normal work starts from `AGENTS.md`, the current repository, and the owning GitHub issue.

The exporter writes selected repository files into one line-numbered generated text file. Generated exports are ignored by Git and must not be committed.

```bash
python3 tools/context/export_repo_context.py
```

The default `offline-review` profile is deliberately bounded. Default output is written below `context/` using the repository folder and selected profile name.

## Profiles

```bash
python3 tools/context/export_repo_context.py --list-profiles
```

Three profiles are retained because they represent materially different offline/manual review breadth:

- `offline-review` — durable authority, repository procedures, package manifests/readmes, and intake templates without implementation source;
- `implementation-review` — current authority plus Rust source, examples, tests, and repository validation tooling;
- `full-audit` — comprehensive active-repository material, including the lockfile and governance/license files.

Use the smallest profile that supports the review. Add exact task-specific paths with `--include` instead of adding domain-, milestone-, issue-, or branch-specific profiles.

Examples:

```bash
python3 tools/context/export_repo_context.py
python3 tools/context/export_repo_context.py --profile implementation-review --include 'crates/runenui_runtime/src/**'
python3 tools/context/export_repo_context.py --profile full-audit --warn-only
```

`full-audit` is intentionally large and is never the default. The bounded profiles exclude generated/build directories, historical legacy content, and `Cargo.lock`.

## Budgets and output

Budget limits prevent accidental oversized exports:

```bash
python3 tools/context/export_repo_context.py --max-files 120 --max-bytes 1500000
```

Use `--output` only when a task requires a specific generated file. Relative paths are resolved from the repository root. Every export begins with a manifest describing the profile, root, included file count/bytes, include/exclude rules, and warnings.

## Rules

- Generated exports are snapshots, never current-work, architecture, or startup authority.
- Do not use an export when direct current repository inspection is available.
- Do not infer GitHub issue, pull-request, blocker, priority, or CI state from an export; those remain live GitHub authority.
- Do not commit generated context files.
- Do not add a profile merely to encode one roadmap slice, domain, current issue, or branch.
- Historical material remains available through Git/history rather than bounded review profiles.

Profile inclusion/exclusion behavior is covered by:

```bash
python3 -m unittest discover -s tools/context/tests -v
```
