# Q1 D3-R observational capture — reference host

**Scope:** [#475](https://github.com/dornglut/runen-ui/issues/475), `UI475-D3R-QUALIFICATION-21`. This is a **draft, opt-in, read-only baseline probe**. No retirement credits, source caps, admission policy, host game permission, mutable widget behavior or M13 finality are changed. The result does **not** qualify D3-R production.

## Authority and provenance

Accepted RunenUI baseline `c5c8a9f01f22202e9b3443e0edec373eec71d01e`, accepted Runenwerk baseline `c341628646e0f936d26cbe459529529af3e893b4`. This branch adds diagnostic lines only to the existing **RunenUI reference Winit host**, and must not be mistaken for the existing accepted `main` behavior. Runenwerk [U9-C draft #1286](https://github.com/dornglut/runenwerk/pull/1286) is not accepted native integration. Existing `RUNENUI_REFERENCE_PROOF=1` and `RUNENUI_TRACE` cover canonical trace; the new `RUNENUI_Q1=1` captures a minimal stream without plaintext committed text/preedit. **Do not type private text** in native measurement sessions; existing optional proof Debug messages may expose keys.

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

Run the fixed scenarios **S1–S8** from [#475 Q1 qualification](https://github.com/dornglut/runen-ui/issues/475#issuecomment-6304742569) with an operator script/checklist. Use a fresh folder per scenario/repetition, commit SHA, OS/display server, CPU/GPU/RAM, device IDs, input method, native/virtual injection provenance, real source loss and display-Present boundary. Recommended one capture per scenario and three physically repeated passes where actually possible; report UNAVAILABLE for missing physical Wayland/X11/Windows/device. A synthetic/headless CI run MUST be labeled as such, not physical evidence.

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
