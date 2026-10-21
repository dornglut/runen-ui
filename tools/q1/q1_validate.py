#!/usr/bin/env python3
"""Q1 schema/finality validator. Does not certify native source mapping or D3-R."""
import argparse
import collections
import hashlib
import json
import math
import pathlib
import sys


def percentile(values, proportion):
    values = sorted(values)
    return values[max(0, math.ceil(len(values) * proportion) - 1)] if values else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory")
    parser.add_argument("--require-physical", action="store_true")
    args = parser.parse_args()
    root = pathlib.Path(args.directory)
    try:
        manifest = json.loads((root / "manifest.json").read_text())
        raw_rows = [json.loads(line) for line in (root / "events.jsonl").read_text().splitlines() if line]
        memory = [json.loads(line) for line in (root / "memory.jsonl").read_text().splitlines() if line]
    except (OSError, ValueError) as error:
        print(json.dumps({"issues": ["INCOMPLETE_OR_MALFORMED_CAPTURE"], "detail": str(error)}))
        return 2
    if not isinstance(manifest, dict):
        print(json.dumps({"issues": ["INVALID_MANIFEST_OBJECT"]}))
        return 2
    issues = []
    warnings = []
    if not isinstance(manifest.get("sha256"), dict):
        manifest["sha256"] = {}
        issues.append("INVALID_ARTIFACT_DIGEST_MANIFEST")
    if manifest.get("schema") != "runenui-q1-run-v1":
        issues.append("UNKNOWN_CAPTURE_SCHEMA")
    if manifest.get("scenario") not in {f"S{i}" for i in range(1, 9)}:
        issues.append("UNKNOWN_SCENARIO")
    if manifest.get("source") not in {"physical", "virtual", "headless"}:
        issues.append("UNKNOWN_SOURCE_CLASS")
    if manifest.get("exit_code") != 0:
        issues.append("NONZERO_OR_UNKNOWN_PROCESS_EXIT")
    if manifest.get("observations") != len(raw_rows):
        issues.append("OBSERVATION_COUNT_MISMATCH")
    required_artifacts = {"events.jsonl", "memory.jsonl"}
    if manifest.get("source") == "physical":
        required_artifacts.add("operator_protocol.json")
    if set(manifest.get("sha256", {})) != required_artifacts:
        issues.append("MISSING_OR_UNEXPECTED_ARTIFACT_DIGEST")
    rows = []
    required_by_stage = {
        "native_keyboard": {"window", "native_device", "key", "state", "repeat"},
        "native_mouse": {"window", "native_device", "button", "state"},
        "native_touch": {"window", "native_device", "touch", "phase"},
        "composition_started": {"generation"},
        "composition_ended": {"generation"},
        "composition_cancelled": {"generation"},
        "pump_end": {"q"},
        "m13_settled": {"scope", "work", "family", "finality"},
        "keyboard_admitted": {"work"},
        "pointer_admitted": {"work"},
    }
    for raw in raw_rows:
        if not isinstance(raw, dict) or raw.get("stream") != "stderr" or not isinstance(raw.get("fields"), dict):
            issues.append("INVALID_Q1_EVENT_ENVELOPE")
            continue
        fields = raw["fields"]
        if not {"seq", "us", "stage"} <= fields.keys():
            issues.append("MISSING_Q1_REQUIRED_FIELDS")
            continue
        try:
            if int(fields["seq"]) < 0 or int(fields["us"]) < 0:
                raise ValueError("negative observer index or clock")
        except (ValueError, TypeError):
            issues.append("INVALID_Q1_SEQUENCE_OR_CLOCK")
            continue
        missing = required_by_stage.get(fields["stage"], set()) - fields.keys()
        if missing:
            issues.append("MISSING_STAGE_FIELDS:" + fields["stage"])
            continue
        if fields["stage"] == "pump_end":
            try:
                if int(fields["q"]) < 0:
                    raise ValueError("negative Q")
            except (ValueError, TypeError):
                issues.append("INVALID_PUMP_QUEUE_SAMPLE")
                continue
        rows.append(fields)
    if len(rows) != len(raw_rows):
        issues.append("INCOMPLETE_OBSERVATION_STREAM")
    for filename in sorted(required_artifacts):
        expected = manifest.get("sha256", {}).get(filename)
        try:
            actual = hashlib.sha256((root / filename).read_bytes()).hexdigest()
        except OSError:
            actual = None
        if not isinstance(expected, str) or len(expected) != 64 or actual != expected:
            issues.append("SHA256_MISMATCH_OR_MISSING:" + filename)
    if manifest.get("source") == "physical":
        try:
            protocol = json.loads((root / "operator_protocol.json").read_text())
        except (OSError, ValueError):
            protocol = None
        required = {"schema", "scenario", "operator_alias", "platform",
                    "hardware", "display_backend", "input_method",
                    "device_inventory", "planned_actions", "source_mapping_evidence"}
        if (not isinstance(protocol, dict)
                or protocol.get("schema") != "runenui-q1-operator-v1"
                or protocol.get("scenario") != manifest.get("scenario")
                or required - protocol.keys()
                or any(not isinstance(protocol[key], str) or not protocol[key].strip()
                       for key in required - {"device_inventory"})
                or not isinstance(protocol.get("device_inventory"), list)
                or not protocol["device_inventory"]
                or any(not isinstance(device, str) or not device.strip()
                       for device in protocol["device_inventory"])):
            issues.append("MISSING_OR_INVALID_OPERATOR_PROTOCOL")
    if not rows:
        issues.append("NO_Q1_OBSERVATIONS")
    if args.require_physical and manifest.get("source") != "physical":
        issues.append("NOT_PHYSICAL")
    elif manifest.get("source") != "physical":
        warnings.append("NOT_PHYSICAL_NATIVE_EVIDENCE")
    if manifest.get("warnings"):
        issues.extend(manifest["warnings"])
    sequence = [int(row["seq"]) for row in rows]
    elapsed = [int(row["us"]) for row in rows]
    if sequence and sequence != list(range(len(sequence))):
        issues.append("OBSERVER_SEQUENCE_GAP_OR_REORDER")
    if elapsed != sorted(elapsed):
        issues.append("OBSERVER_CLOCK_REORDER")
    stages = collections.Counter(row.get("stage", "UNKNOWN") for row in rows)
    settlements = set()
    admitted = {}
    durations = []
    matched_admissions = set()
    finality = collections.Counter()
    for row in rows:
        stage = row.get("stage")
        if stage in ("keyboard_admitted", "pointer_admitted"):
            if "work" not in row:
                issues.append("ADMISSION_MISSING_WORK_SEQUENCE")
                continue
            if row["work"] in admitted:
                issues.append("DUPLICATE_ADMITTED_WORK_SEQUENCE")
            admitted[row["work"]] = int(row["us"])
        if stage == "m13_settled":
            if not {"scope", "work", "family", "finality"} <= row.keys():
                issues.append("M13_SETTLEMENT_MISSING_FIELDS")
                continue
            key = (row["scope"], row["work"])
            if key in settlements:
                issues.append("DUPLICATE_SCOPE_SEQUENCE_M13_SETTLEMENT")
            settlements.add(key)
            finality[(row["family"], row["finality"])] += 1
            if row["work"] in admitted:
                elapsed_us = int(row["us"]) - admitted[row["work"]]
                if elapsed_us < 0:
                    issues.append("NEGATIVE_ADMISSION_SETTLEMENT_LATENCY")
                elif row["work"] in matched_admissions:
                    issues.append("MULTIPLE_MATCHES_FOR_ADMISSION")
                else:
                    durations.append(elapsed_us)
                    matched_admissions.add(row["work"])
    missing_settlements = set(admitted) - matched_admissions
    if missing_settlements:
        issues.append("ADMITTED_INPUT_WITHOUT_OBSERVED_M13_SETTLEMENT")
    held = collections.Counter()
    raw_id_pairs = set()
    max_held = 0
    for row in rows:
        if row.get("stage") != "native_keyboard":
            continue
        key = (row.get("window"), row.get("native_device"), row.get("key"))
        raw_id_pairs.add(key[:2])
        if row.get("state") == "Pressed" and row.get("repeat") == "false":
            # A duplicated raw Down is not another physical held-key lifetime.
            held[key] = 1
        elif row.get("state") == "Released" and held[key] > 0:
            held[key] -= 1
        max_held = max(max_held, sum(held.values()))
    # These are Winit observation peaks, NOT admitted UI lifetime/credit counts.
    mouse_held = set()
    touch_held = set()
    composition_held = set()
    max_mouse = max_touch = max_composition = 0
    for row in rows:
        stage = row.get("stage")
        if stage == "native_mouse":
            key = (row.get("window"), row.get("native_device"), row.get("button"))
            if row.get("state") == "Pressed":
                mouse_held.add(key)
            elif row.get("state") == "Released":
                mouse_held.discard(key)
        elif stage == "native_touch":
            key = (row.get("window"), row.get("native_device"), row.get("touch"))
            if row.get("phase") == "Started":
                touch_held.add(key)
            elif row.get("phase") in ("Ended", "Cancelled"):
                touch_held.discard(key)
        elif stage == "composition_started":
            composition_held.add(row["generation"])
        elif stage in ("composition_ended", "composition_cancelled"):
            composition_held.discard(row["generation"])
        elif stage == "composition_retired":
            # Stale generation is host-owned; its identity is no longer valid.
            composition_held.clear()
        max_mouse = max(max_mouse, len(mouse_held))
        max_touch = max(max_touch, len(touch_held))
        max_composition = max(max_composition, len(composition_held))
    if not stages.get("native_device_removed"):
        warnings.append("NATIVE_DEVICE_REMOVAL_NOT_OBSERVED")
    required_all = {
        "S2": ("native_keyboard", "keyboard_admitted", "m13_settled"),
        "S3": ("native_keyboard", "native_device_removed"),
        "S4": ("native_mouse", "native_touch", "pointer_admitted"),
        "S5": ("native_ime", "composition_started"),
    }
    scenario = manifest.get("scenario")
    missing = [name for name in required_all.get(scenario, ()) if not stages.get(name)]
    if missing:
        issues.append("SCENARIO_MISSING_REQUIRED_STAGES:" + ",".join(missing))
    if scenario == "S2":
        keyboard_rows = [row for row in rows if row.get("stage") == "native_keyboard"]
        if not any(row.get("state") == "Pressed" and row.get("repeat") == "false" for row in keyboard_rows):
            issues.append("S2_MISSING_INITIAL_DOWN")
        if not any(row.get("state") == "Pressed" and row.get("repeat") == "true" for row in keyboard_rows):
            issues.append("S2_MISSING_REPEAT")
        if not any(row.get("state") == "Released" for row in keyboard_rows):
            issues.append("S2_MISSING_RELEASE")
    if scenario == "S5":
        ime_kinds = {row.get("kind") for row in rows if row.get("stage") == "native_ime"}
        if not {"preedit", "commit", "disabled"} <= ime_kinds:
            issues.append("S5_MISSING_PREEDIT_COMMIT_OR_DISABLED")
    if scenario == "S5" and not (stages.get("composition_ended") or stages.get("composition_cancelled")):
        issues.append("SCENARIO_MISSING_COMPOSITION_TERMINATION")
    if scenario == "S7" and not (stages.get("native_focus") or stages.get("native_device_removed")):
        issues.append("SCENARIO_MISSING_SOURCE_LOSS")
    if args.require_physical and not manifest.get("sha"):
        issues.append("MISSING_CHECKED_OUT_BUILD_SHA")
    if any(int(row["q"]) > 4096 for row in rows if row.get("stage") == "pump_end"):
        issues.append("INVALID_QUEUE_CHECKPOINT")
    if manifest.get("source") == "physical" and (not manifest.get("psutil") or not memory):
        issues.append("NO_NATIVE_RSS_SAMPLING")
    valid_memory = []
    for item in memory:
        if not isinstance(item, dict) or not isinstance(item.get("rss_bytes"), int) or item["rss_bytes"] < 0:
            issues.append("INVALID_MEMORY_SAMPLE")
        else:
            valid_memory.append(item)
    memory = valid_memory
    queue_samples = [int(row["q"]) for row in rows if row.get("stage") == "pump_end"]
    elapsed_seconds = manifest.get("elapsed_s") or 0
    native_events = sum(
        stages.get(name, 0)
        for name in ("native_keyboard", "native_mouse", "native_touch", "native_cursor", "native_ime")
    )
    output = {
        "schema": "runenui-q1-validation-v1",
        "baseline_sha": manifest.get("sha"),
        "scenario": manifest.get("scenario"),
        "source": manifest.get("source"),
        "operator_attestation_sha256": manifest.get("sha256", {}).get("operator_protocol.json"),
        "operator_attestation_qualification": "operator assertion only; physical device and source mapping remain independently unverified",
        "stage_counts": dict(sorted(stages.items())),
        "native_winit_window_device_pairs": len(raw_id_pairs),
        "max_observed_native_held_keys": max_held,
        "still_held_at_close": sum(held.values()),
        "max_observed_native_mouse_buttons": max_mouse,
        "max_observed_native_touch_contacts": max_touch,
        "max_host_observed_composition_generations": max_composition,
        "mouse_buttons_held_at_close": len(mouse_held),
        "touch_contacts_held_at_close": len(touch_held),
        "host_compositions_held_at_close": len(composition_held),
        "lifetime_qualification": "raw Winit/host observations only, NOT accepted source credits or authenticated physical devices",
        "m13_unique_settlements": len(settlements),
        "observed_admitted_inputs": len(admitted),
        "observed_admitted_to_m13_matches": len(matched_admissions),
        "unmatched_admitted_inputs": len(missing_settlements),
        "m13_finality": {
            f"{family}:{status}": count for (family, status), count in sorted(finality.items())
        },
        "admission_to_m13_us": {
            "n": len(durations), "p50": percentile(durations, .5),
            "p95": percentile(durations, .95),
            "p99": percentile(durations, .99) if len(durations) >= 1000 else None,
        },
        "rss_peak_bytes": max((item["rss_bytes"] for item in memory), default=None),
        "native_observed_events_per_second": (
            native_events / elapsed_seconds if elapsed_seconds > 0 else None
        ),
        "pump_checkpoint_max_waiting_Q": max(queue_samples, default=None),
        "pump_checkpoint_min_free_C_minus_Q": (
            4096 - max(queue_samples) if queue_samples else None
        ),
        "queue_headroom_qualification": "checkpoint only; no D3-R future T/G credits or continuous high-water on accepted main",
        "admission_latency_qualification": "correlation by observed WorkSequence within reference host, not verified physical OS-to-framework source identity or descendant finality",
        "blocked": [
            "physical device mapping and unobserved source teardown",
            "full S1-S8 campaign not exercisable by reference_winit single-editor DemoApp; see tooling README coverage matrix",
            "native event to completed Present boundary",
            "accepted main contains no D3-R credits",
            "no ratified source caps or performance thresholds",
        ],
        "issues": sorted(set(issues)),
        "warnings": sorted(set(warnings)),
    }
    (root / "validation.json").write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps(output, indent=2))
    return int(bool(issues))


if __name__ == "__main__":
    sys.exit(main())
