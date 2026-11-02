#!/usr/bin/env bash
# Exercise the real native host's typed, non-recoverable presentation exit.
# This intentionally injects a host-visible status, not a real driver fault.
set -euo pipefail

wm_pid=""
cleanup() {
  if [[ -n "$wm_pid" ]]; then
    kill "$wm_pid" 2>/dev/null || true
    wait "$wm_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

openbox --sm-disable >"${RUNNER_TEMP:-/tmp}/runenui-openbox-fatal.log" 2>&1 &
wm_pid="$!"
sleep 1
kill -0 "$wm_pid"

log="${RUNNER_TEMP:-/tmp}/fluent_counter_fatal.log"
set +e
"./target/debug/fluent_counter" >"$log" 2>&1
status="$?"
set -e

if [[ "$status" -ne 1 ]]; then
  echo "Native fatal test must exit with typed Rust failure status 1, got $status" >&2
  cat "$log" >&2
  exit 1
fi
if ! grep -Fq "RUNENUI_NATIVE_TEST_RENDER_FAULT=SurfaceValidation" "$log"; then
  echo "Fatal surface validation fault did not reach native host" >&2
  cat "$log" >&2
  exit 1
fi
if ! grep -Fq 'Fatal("native presentation failed: native surface acquisition failed validation")' "$log"; then
  echo "Native host did not preserve its typed presentation failure on exit" >&2
  cat "$log" >&2
  exit 1
fi
echo "Native fatal presentation and nonzero typed exit PASS"
