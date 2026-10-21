# Q1 D3-R observational capture — reference host

**Scope:** owning RunenUI issue 475, `UI475-D3R-QUALIFICATION-21`. This is a **draft, opt-in, read-only baseline probe**. No retirement credits, source caps, admission policy, host game permission, mutable widget behavior or M13 finality are changed. The result does **not** qualify D3-R production.

## Authority and provenance

Accepted RunenUI and Runenwerk baselines MUST be pinned to their separately verified immutable main commits at measurement time; the exact SHAs belong in the run manifest and owning GitHub issue rather than this durable guide. This branch adds diagnostic lines only to the existing **RunenUI reference Winit host**, and must not be mistaken for the existing accepted `main` behavior. Runenwerk U9-C draft #1286 is not accepted native integration. Existing `RUNENUI_REFERENCE_PROOF=1` and `RUNENUI_TRACE` cover canonical trace; the new `RUNENUI_Q1=1` captures a minimal stream without plaintext committed text/preedit. **Do not type private text** in native measurement sessions; existing optional proof Debug messages may expose keys.

## Build/run on owner-accessible real hardware

On a normal checked-out Rust toolchain with native display, after checking out the exact **Q1 draft feature SHA** (not protected main):

```sh
git rev-parse HEAD
cargo +stable fmt --all -- --check
cargo validate
RUNENUI_Q1=1 cargo +stable run --locked -p reference_winit --release 2> q1-raw-stderr.log
```

On Windows PowerShell, launch within the same checked-out worktree:

```powershell
$env:RUNENUI_Q1 = '1'
cargo +stable run --locked -p reference_winit --release 2>&1 | Tee-Object q1-raw-stderr.log
```

Optional: `RUNENUI_REFERENCE_PROOF=1` enables the previously accepted canonical redacted trace sink and other existing proof diagnostics. It is **not required** to capture the Q1 source/M13 fields.

Run the fixed scenarios **S1–S8** from owning RunenUI issue 475's Q1 qualification record with an operator script/checklist. Use a fresh folder per scenario/repetition, commit SHA, OS/display server, CPU/GPU/RAM, device IDs, input method, native/virtual injection provenance, real source loss and display-Present boundary. Recommended one capture per scenario and three physically repeated passes where actually possible; report UNAVAILABLE for missing physical Wayland/X11/Windows/device. A synthetic/headless CI run MUST be labeled as such, not physical evidence.

### Observer line schema `RUNENUI_Q1` (schema v1)

```text
RUNENUI_Q1 seq=<monotonic-u64> us=<monotonic-elapsed> stage=<stage> [typed k=v fields...]
```

- `native_keyboard`: Winit window, raw **WindowEvent** device ID, physical key, Down/Up state, repeat, synthetic; never logical/text payload.
- `native_mouse/native_touch/native_cursor`: raw Winit device and event kind, no pointer coordinates.
- `native_ime`: Enabled/Disabled/Preedit/Commit and only preedit/commit byte count.
- `native_device_removed`: raw **DeviceEvent** ID; **do not equate this ID with WindowEvent keyboard source** without separate platform proof.
- `device_resolved`: adapter input device ID mapping as currently performed, without changing `DeviceIdentityMap`.
- `keyboard_admitted/pointer_admitted`: actual accepted `WorkSequence`; `keyboard_refused*` and `pointer_refused`: existing typed rejection (no fabricated receipt). No changed `KeyboardDownDecision`.
- `composition_started`: accepted generation and input WorkSequence.
- `pump_end`: queue occupancy at pump checkpoint, number popped, observed through-sequence and revision. **Not a continuous live queue occupancy trace**, and no future-credit T exists on accepted main.
- `m13_settled`: exact runtime scope, original input WorkSequence, family, real `Committed/ProcessingRejected/Aborted` and ownership revision. `m13_ownership_changed/m13_scope_retired`: actual records from the same returned public batch. The logger does not create native settlement or obtain a fabricated second record stream.

The native-to-admission clock is single-process `Instant`, but only a genuine OS event source plus proven source mapping can establish physical provenance. `seq` is diagnostic log order, **not** WorkSequence. Lines are observational stderr from the existing Winit host; host policy and FIFO ordering are unchanged. They do not measure actual completed Present, raw-vs-window device correlation or pending user-output ownership.

### Q1 evidence validation

Preserve the **unaltered raw trace** and its SHA256, exact build refs, workload script, machine/platform manifest, and a non-private record of the operator actions. Capture optional process RSS/CPU at fixed intervals (for example through `psutil` or native OS profiler) **without** promoting debug-mode figures to production metrics. Validate: per-stream `seq` contiguous and monotonic `us`; M13 `(scope,WorkSequence)` settles at most once; order of `OwnershipChanged` and same native `InputSettled` stays as returned; release matching uses exact Winit window event source; raw Removed is *not* presumed to match keyboard source; source count includes only authentically observed eligible physical generations; queue occupancy is sampled, not full high-water; missing real native loss => **BLOCKED**, never accepted by trace inference.

For performance use **release** builds with fixed CPU/OS/display/profile, repeated identical scripted workload, capture p50/p95/p99 only with sufficient independent samples (at least 1000 observations for meaningful p99), CPU/alloc/RSS, event→M13 and event→completed-Present measured at their exact respective owning boundaries. Any native OS/hardware unavailable in hosted environments is BLOCKED. Compare an authorized D3-R candidate only in a **separate future acceptance slice**; none exists on this draft branch. Source caps, C/R changes, or performance thresholds must be owner ratified from measurements, not invented here.

**Safety and scope:** do not merge this instrumentation as a D3-R product behavior change, amend accepted ADR 0005/0006/0019, or repin Runenwerk. This Q1 draft remains independent of #476 and expired/closed #499–#501.



### Accepted Runenwerk Editor/native workload comparison (separate owner)

In a **separate** checked-out `dornglut/runenwerk` worktree at verified accepted Runenwerk main commit, recorded in the run manifest, with its normal toolchain and supported native GPU/OS:

```sh
git rev-parse HEAD
cargo +stable build --release --locked -p runenwerk_editor -p runenwerk_render_lab
./target/release/runenwerk_editor
./target/release/runenwerk-render-lab
```

The Render Lab executable name is `runenwerk-render-lab`; on Windows use the corresponding `.exe`. Collect the same **operator-defined S1–S8 workload/rate, OS device inventory, native RSS and event-to-display evidence when available**, but **do not** submit this Editor process to the RunenUI Q1 validator and call it a RunenUI/M13 result: accepted Runenwerk main does not contain the U9-C native RunenUI source bridge, and its persisted normalized recordings do not record raw Winit identities, text/IME and final native teardown. Real Editor original M13 correlation needs a separately authorized source-qualified observational integration, not draft #1286 inference. Status until observed: **BLOCKED**.

## Tooling decision and tested boundaries

**Decision for Q1:** retain the limited Python 3 standard-library capture and validation
scripts rather than create a persistent Rust `xtask` acquisition subsystem. This
workflow is an opt-in, operator-driven, temporary physical-measurement instrument,
not a production RunenUI component or new canonical validation authority. `psutil`
is the only optional nonstandard Python dependency, and is **required for a
physically qualified RSS sample**. If the workflow becomes permanent or is
adopted as a mandatory repository gate, reassess Rust `xtask` integration
and the cost of keeping Python. Language uniformity alone is not a reason to
rewrite the collector.

Run the checked-in **synthetic tooling regressions** before any physical capture,
on both the desired host OS and in the operator checkout:

```sh
python3 -m unittest discover -s tools/q1 -p 'test_q1.py' -v
```

These focused Python tests are **not** covered by `cargo validate` or the
existing Rust CI partitions; canonical CI success alone does not verify
the Python validator. Negative cases must exit nonzero, including malformed
events, missing required scenario steps, interrupted/nonzero native process
exit, missing M13 correlation and observer sequence gaps. Tests are explicitly
synthetic; they do not qualify physical Windows/Linux behavior.

A successful `q1_validate.py` result certifies only local structured capture
integrity. In particular, it requires a process exit code of zero, matching
record count and expected artifact SHA256 digests, contiguous observer indices
starting at zero, actual stderr records, and any observed keyboard/pointer
admission to have a matching observed M13 settlement. It rejects observed
stage omissions and missing required S2 repeat/release and S5
Preedit/Commit/Disabled classes. This is **not** proof that the original native
source identity was authenticated or that M13 describes descendant/gameplay
completion. Tool-observed WorkSequence matches cannot establish that semantic
linkage on their own. An exit with no missing records still cannot certify
the completeness of source types this host never emitted.

## Owner-executable collector and validator

Build the exact branch **before** capture so RSS represents the native host, **not the Cargo build process**:

```sh
cargo +stable build --release --locked -p reference_winit
python3 -m pip install psutil
python3 tools/q1/q1_capture.py --repo . --scenario S1 --source physical --out ./q1-results/S1-run1 -- ./target/release/reference_winit
python3 tools/q1/q1_validate.py ./q1-results/S1-run1 --require-physical
```

Windows executable: `./target/release/reference_winit.exe` and `python` instead of `python3`; launch from a PowerShell session with the exact feature checkout and a real interactive window. Repeat S1–S8 (see `scenarios.json`) on the actual available Windows/Linux X11/Wayland systems. The `--source physical` label is **operator attestation**, not a platform auto-detection; never apply it to Xvfb, VNC simulation, CI or a scripted headless event. Output directories must be fresh. The collector preserves only structured Q1 lines and SHA256s of `events.jsonl`/`memory.jsonl`, plus `manifest.json`; it does not retain other stderr or Debug text. The wrapper records the **exact launched executable path and its SHA256** as a reproducibility aid; this does not by itself prove that the binary was built from the manifest source SHA. It rejects a nonexistent/implicit PATH target; pass the previously built binary explicitly. `manifest.json` is not signed: custody of the directory and its independent external checksum/attachment still matters. `psutil` is needed to report RSS; its absence leaves that metric unqualified. Copy results **outside version control**, add the actual hardware/input-device/display manifest and real operator run sheet, and keep private key/text input out of the workload.

The validator checks raw Winit keyboard, mouse, touch and host-composition lifetime peaks (observed, not UI-admitted), explicit S2–S5 input/protocol stage presence, observer order, duplicate `(scope,WorkSequence)` M13 outcomes, missing admitted keyboard/pointer settlement, and observed Winit IDs/held-key high-water. Physical runs also require an actual RSS sample and an actual repository SHA in the manifest, but remain **operator-attested** rather than automatically hardware-authenticated. It **cannot** authenticate raw DeviceEvent-to-WindowEvent physical device correlation, infer genuine source retirement or derive a D3-R source cap from a single run. Native loss, exact input→completed Present, and downstream Editor U9-C native proof remain **BLOCKED** without separate physical source evidence. A locally generated fixture only tests the parser, not RunenUI behavior.

