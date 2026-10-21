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
    manifest = json.loads((root / "manifest.json").read_text())
    rows = [json.loads(line) for line in (root / "events.jsonl").read_text().splitlines() if line]
    rows = [row["fields"] for row in rows if "fields" in row]
    issues = []
    warnings = []
    for filename, expected in manifest.get("sha256", {}).items():
        actual = hashlib.sha256((root / filename).read_bytes()).hexdigest()
        if actual != expected:
            issues.append("SHA256_MISMATCH:" + filename)
    if not rows:
        issues.append("NO_Q1_OBSERVATIONS")
    if args.require_physical and manifest["source"] != "physical":
        issues.append("NOT_PHYSICAL")
    elif manifest["source"] != "physical":
        warnings.append("NOT_PHYSICAL_NATIVE_EVIDENCE")
    if manifest.get("warnings"):
        issues.extend(manifest["warnings"])
    sequence = [int(row["seq"]) for row in rows]
    elapsed = [int(row["us"]) for row in rows]
    if sequence and sequence != list(range(sequence[0], sequence[0] + len(sequence))):
        issues.append("OBSERVER_SEQUENCE_GAP_OR_REORDER")
    if elapsed != sorted(elapsed):
        issues.append("OBSERVER_CLOCK_REORDER")
    stages = collections.Counter(row.get("stage", "UNKNOWN") for row in rows)
    settlements = set()
    admitted = {}
    durations = []
    finality = collections.Counter()
    for row in rows:
        stage = row.get("stage")
        if stage in ("keyboard_admitted", "pointer_admitted"):
            admitted[row["work"]] = int(row["us"])
        if stage == "m13_settled":
            key = (row["scope"], row["work"])
            if key in settlements:
                issues.append("DUPLICATE_SCOPE_SEQUENCE_M13_SETTLEMENT")
            settlements.add(key)
            finality[(row["family"], row["finality"])] += 1
            if row["work"] in admitted:
                durations.append(int(row["us"]) - admitted[row["work"]])
    held = collections.Counter()
    raw_id_pairs = set()
    max_held = 0
    for row in rows:
        if row.get("stage") != "native_keyboard":
            continue
        key = (row.get("window"), row.get("native_device"), row.get("key"))
        raw_id_pairs.add(key[:2])
        if row.get("state") == "Pressed" and row.get("repeat") == "false":
            held[key] += 1
        elif row.get("state") == "Released" and held[key] > 0:
            held[key] -= 1
        max_held = max(max_held, sum(held.values()))
    if not stages.get("native_device_removed"):
        warnings.append("NATIVE_DEVICE_REMOVAL_NOT_OBSERVED")
    memory = [json.loads(line) for line in (root / "memory.jsonl").read_text().splitlines() if line]
    output = {
        "schema": "runenui-q1-validation-v1",
        "baseline_sha": manifest["sha"],
        "scenario": manifest["scenario"],
        "source": manifest["source"],
        "stage_counts": dict(sorted(stages.items())),
        "native_winit_window_device_pairs": len(raw_id_pairs),
        "max_observed_native_held_keys": max_held,
        "still_held_at_close": sum(held.values()),
        "m13_unique_settlements": len(settlements),
        "m13_finality": {
            f"{family}:{status}": count for (family, status), count in sorted(finality.items())
        },
        "admission_to_m13_us": {
            "n": len(durations), "p50": percentile(durations, .5),
            "p95": percentile(durations, .95),
            "p99": percentile(durations, .99) if len(durations) >= 1000 else None,
        },
        "rss_peak_bytes": max((item["rss_bytes"] for item in memory), default=None),
        "blocked": [
            "physical device mapping and unobserved source teardown",
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
