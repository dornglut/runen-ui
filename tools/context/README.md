# Context Export

> **Category: Guide**

`export_repo_context.py` exports selected repository files into one line-numbered text file for AI or manual review.

The default export writes into the repository-local `context/` folder:

```bash
python3 tools/context/export_repo_context.py
```

Default output shape:

```text
context/<repo-folder-name>-<profile>-context.txt
```

For this repository and the default profile, that is normally:

```text
context/RunenUI-ai-core-context.txt
```

The `context/` folder is for generated exports only. Its generated files are ignored by Git and should not be committed.

## Profiles

List available profiles:

```bash
python3 tools/context/export_repo_context.py --list-profiles
```

Use a profile:

```bash
python3 tools/context/export_repo_context.py --profile current-work
python3 tools/context/export_repo_context.py --profile domain-work
python3 tools/context/export_repo_context.py --profile implementation-work --include 'crates/ui_core/src/**'
python3 tools/context/export_repo_context.py --profile full-audit --warn-only
```

Profiles live in:

```text
tools/context/profiles/
```

## Profile intent

```text
ai-core
  Small RunenUI authority context for normal AI startup.

current-work
  Current RunenUI work context, including root authority, docs, tools, source, crates, examples, and tests.

workspace-planning
  Planning and documentation context without implementation-heavy legacy audit.

domain-work
  Domain and crate-level context for architecture or boundary review.

implementation-work
  Generic implementation context. Add exact crate, module, test, or example paths with --include.

full-audit
  Comprehensive active-repository audit context. Includes CI YAML, Cargo.lock,
  license files, and other repository metadata; excludes historical legacy content.
```

## Task-specific overrides

Profiles should stay generic. Add task-specific paths at the command line instead of creating hardcoded profiles for one feature area.

```bash
python3 tools/context/export_repo_context.py \
  --profile implementation-work \
  --include 'crates/ui_core/src/**' \
  --include 'crates/ui_core/tests/**'
```

Other override options:

```bash
python3 tools/context/export_repo_context.py --profile current-work --include 'apps/**'
python3 tools/context/export_repo_context.py --profile implementation-work --extension json
python3 tools/context/export_repo_context.py --profile implementation-work --include-filename AGENTS.md
```

## Explicit output path

Use `--output` only when a task needs a named export:

```bash
python3 tools/context/export_repo_context.py \
  --profile current-work \
  --output context/current-work-context.txt
```

Relative output paths are resolved from the repository root. Absolute paths are used as-is.

## Windows PowerShell examples

From the repository root:

```powershell
py tools/context/export_repo_context.py
py tools/context/export_repo_context.py --profile current-work
py tools/context/export_repo_context.py --profile implementation-work --include 'crates/ui_core/src/**'
```

Copy the generated context for a new AI thread:

```powershell
Get-Content .\context\RunenUI-ai-core-context.txt -Raw | Set-Clipboard
```

## Budgets

Budget options prevent accidentally creating a huge context file:

```bash
python3 tools/context/export_repo_context.py --profile current-work --max-files 120 --max-bytes 1500000
```

By default, a budget breach fails the export. To write anyway:

```bash
python3 tools/context/export_repo_context.py --profile full-audit --max-bytes 3000000 --warn-only
```

## Manifest

Every generated context file starts with a manifest that records:

```text
profile name
description
root path
included file count
total source bytes
include globs
exclude globs
extensions
include filenames
warnings
```

This makes it clear whether a new AI thread is seeing a small authority context, current work context, implementation context, or full audit dump.

## Rules

Use the smallest profile that can answer the task.

Do not use `full-audit` as the default. It is intentionally large. Historical
material removed from the active branch remains available through the archival
tag documented in `docs/history/legacy-archive.md`.

Do not add feature-specific profiles for every roadmap item. Use `--include` and `--exclude` for task-specific scope.
