# Legacy Runenwerk UI Archive

> **Category: Historical record**

The former `legacy/` tree was removed from active repository content during M0B. It contained 671 historical files and a broad proof-oriented Runenwerk UI crate graph whose compiler/program/artifact, route-string/schema action, host, ECS, and early ownership assumptions are not the RunenUI production foundation.

The exact pre-removal tree is preserved by Git history and the annotated tag:

```text
legacy-runenwerk-ui-archive-2026-07-11
```

The tag points to `master` commit `141f005289e1ac2fe5d66c6cc35622f5db8d3406`, the audited baseline immediately before M0 work.

To inspect a historical file without restoring it to the active branch:

```powershell
git show legacy-runenwerk-ui-archive-2026-07-11:legacy/<path>
```

To inspect the complete archive, create a separate temporary worktree or checkout at the tag. Do not add legacy crates to the active workspace.

Potentially reusable ideas include layout edge cases, focus and hit-test tests, trace/report structure, accessibility/source mapping, renderer-neutral primitive concepts, deterministic stories, and later document/compiler concepts. Salvage one contract, fixture, or test idea only when an active design requires it; reimplement it under current RunenUI ownership and vocabulary rather than porting the legacy graph wholesale.
