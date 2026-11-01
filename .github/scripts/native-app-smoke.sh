#!/usr/bin/env bash
# Native X11/Xvfb smoke: two independent public-API programs, real windows,
# real renderer startup and WM_DELETE_WINDOW shutdown. Not an IME proof.
set -euo pipefail

wm_pid=""
app_pid=""
cleanup() {
  if [[ -n "$app_pid" ]]; then
    kill "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  if [[ -n "$wm_pid" ]]; then
    kill "$wm_pid" 2>/dev/null || true
    wait "$wm_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

openbox --sm-disable >"${RUNNER_TEMP:-/tmp}/runenui-openbox.log" 2>&1 &
wm_pid="$!"
sleep 1
kill -0 "$wm_pid"

for entry in "fluent_counter:Counter" "fluent_status:Status"; do
  executable="${entry%%:*}"
  window_title="${entry#*:}"
  log="${RUNNER_TEMP:-/tmp}/${executable}.log"
  window_id=""
  echo "Starting ${executable} as a real native window"
  "./target/debug/${executable}" >"$log" 2>&1 &
  app_pid="$!"

  for _ in $(seq 1 100); do
    if ! kill -0 "$app_pid" 2>/dev/null; then
      echo "Native process terminated before window became visible: ${executable}" >&2
      cat "$log" >&2
      exit 1
    fi
    window_id="$(xdotool search --onlyvisible --name "^${window_title}$" 2>/dev/null | head -n 1 || true)"
    if [[ -n "$window_id" ]]; then
      break
    fi
    sleep 0.2
  done

  if [[ -z "$window_id" ]]; then
    echo "Native window did not appear: ${executable}" >&2
    cat "$log" >&2
    exit 1
  fi

  echo "Found visible native window: ${executable} id=${window_id}"
  xdotool windowactivate --sync "$window_id"
  xdotool windowsize "$window_id" 850 530
  # The second consumer has a one-shot initial timer (1500 ms).
  # Require an exact callback observation, not just an idle process.
  if [[ "$executable" == "fluent_status" ]]; then
    timer_observed=""
    for _ in $(seq 1 100); do
      if grep -Fq "RUNENUI_FLUENT_STATUS_TIMER_FIRED" "$log"; then
        timer_observed=yes
        break
      fi
      if ! kill -0 "$app_pid" 2>/dev/null; then
        break
      fi
      sleep 0.2
    done
    if [[ -z "$timer_observed" ]]; then
      echo "Canonical initial timer did not reach application action" >&2
      cat "$log" >&2
      exit 1
    fi
    echo "Canonical native timer action PASS"
    # A genuine XTest keyboard event should activate the explicitly authored
    # and runtime-focused button. Do not submit an internal semantic action.
    xdotool windowactivate --sync "$window_id"
    xdotool key --clearmodifiers space
    key_observed=""
    for _ in $(seq 1 50); do
      if grep -Fq "RUNENUI_FLUENT_STATUS_TOGGLED" "$log"; then
        key_observed=yes
        break
      fi
      if ! kill -0 "$app_pid" 2>/dev/null; then
        break
      fi
      sleep 0.2
    done
    if [[ -z "$key_observed" ]]; then
      echo "Native X11 Space activation did not reach authored button action" >&2
      cat "$log" >&2
      exit 1
    fi
    echo "Focused native keyboard action PASS"
  fi
  if ! kill -0 "$app_pid" 2>/dev/null; then
    echo "Native process failed during resize/idle: ${executable}" >&2
    cat "$log" >&2
    exit 1
  fi
  # Request WM-managed graceful closure via _NET_CLOSE_WINDOW. Unlike
  # xdotool windowclose (XDestroyWindow), this must not destroy the X11
  # target before winit can retire its native input context.
  wmctrl -ic "$window_id"

  closed=""
  for _ in $(seq 1 100); do
    if ! kill -0 "$app_pid" 2>/dev/null; then
      closed=yes
      break
    fi
    sleep 0.2
  done
  if [[ -z "$closed" ]]; then
    echo "Window did not exit after WM close: ${executable}" >&2
    cat "$log" >&2
    exit 1
  fi
  wait "$app_pid" || { echo "Native application exited with error: ${executable}" >&2; cat "$log" >&2; exit 1; }
  app_pid=""
  echo "Native open/resize/close PASS: ${executable}"
done
