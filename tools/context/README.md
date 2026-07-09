# Context Export

`export_repo_context.py` exports selected repository files into one line-numbered text file for AI or manual review.

The default export is intentionally small:

```bash
python3 tools/context/export_repo_context.py
```

This uses the `ai-core` profile. It is meant for normal GPT startup and includes current authority files, not the full repository.

## Profiles

List available profiles:

```bash
python3 tools/context/export_repo_context.py --list-profiles
```

Use a profile:

```bash
python3 tools/context/export_repo_context.py --profile current-work
python3 tools/context/export_repo_context.py --profile domain-work
python3 tools/context/export_repo_context.py --profile implementation-work --include 'domain/ui/ui_controls/src/**'
python3 tools/context/export_repo_context.py --profile full-audit --warn-only
```

Profiles live in:

```text
tools/context/profiles/
```

## Profile intent

```text
ai-core
  Small current authority context for most AI work.

current-work
  Current active work context across domains.

workspace-planning
  Workspace planning, routines, and current roadmap context.

domain-work
  Domain-level authority and crate entrypoints across all domains.

implementation-work
  Generic implementation authority. Add exact crate or module paths with --include.

full-audit
  Large full-repository audit context. Use only when historical or broad review is required.
```

## Task-specific overrides

Profiles should stay generic. Add task-specific paths at the command line instead of creating hardcoded profiles for one feature area.

```bash
python3 tools/context/export_repo_context.py \
  --profile implementation-work \
  --include 'domain/ui/ui_controls/src/**' \
  --include 'domain/ui/ui_controls/tests/**'
```

Other override options:

```bash
python3 tools/context/export_repo_context.py --profile current-work --include 'apps/**'
python3 tools/context/export_repo_context.py --profile domain-work --exclude 'domain/experimental/**'
python3 tools/context/export_repo_context.py --profile implementation-work --extension json
python3 tools/context/export_repo_context.py --profile implementation-work --include-filename AGENTS.md
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

## Rule

Use the smallest profile that can answer the task.

Do not use `full-audit` as the default. It is intentionally large and may include historical docs that are not current authority.

Do not add feature-specific profiles for every roadmap item. Use `--include` and `--exclude` for task-specific scope.
