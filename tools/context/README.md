# Context Export

> **Category: Guide**

`export_repo_context.py` is optional convenience tooling for offline/manual review or environments that cannot inspect the repository directly. It is **not** repository authority and is not the normal contributor/agent startup path. Normal work starts from `AGENTS.md`, the current repository, and the owning GitHub issue.

The exporter writes selected repository files into one line-numbered generated text file. Generated exports are ignored by Git and must not be committed.

```bash
python3 tools/context/export_repo_context.py
```

Default output is written below `context/` using the repository folder and selected profile name.

## Profiles

```bash
python3 tools/context/export_repo_context.py --list-profiles
```

Available profiles provide bounded convenience views such as authority-oriented, current-work, workspace-planning, domain, implementation, and full-audit exports. Use the smallest profile that can support the offline/manual task. Add task-specific paths with `--include` instead of creating feature-specific profiles.

Examples:

```bash
python3 tools/context/export_repo_context.py --profile current-work
python3 tools/context/export_repo_context.py --profile implementation-work --include 'crates/runenui_runtime/src/**'
python3 tools/context/export_repo_context.py --profile full-audit --warn-only
```

`full-audit` is intentionally large and should not be the default. Normal profiles exclude generated/build directories and historical legacy content; `Cargo.lock` is included only where the profile explicitly requires it.

## Budgets and output

Budget limits prevent accidental oversized exports:

```bash
python3 tools/context/export_repo_context.py --profile current-work --max-files 120 --max-bytes 1500000
```

Use `--output` only when a task requires a specific generated file. Relative paths are resolved from the repository root. Every export begins with a manifest describing the profile, root, included file count/bytes, include/exclude rules, and warnings.

## Rules

- Generated exports are snapshots, never current-work or architecture authority.
- Do not use an export when direct current repository inspection is available.
- Do not commit generated context files.
- Do not add a profile merely to encode one roadmap slice or current issue state.
- Historical material remains available through Git/history rather than normal context profiles.

Profile inclusion/exclusion behavior is covered by:

```bash
python3 -m unittest discover -s tools/context/tests -v
```
