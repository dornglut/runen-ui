#!/usr/bin/env python3
"""Synthetic Q1 tooling regression fixtures. Never physical input evidence."""
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

VALIDATOR = pathlib.Path(__file__).with_name("q1_validate.py")


def record(stage, **fields):
    return {"stage": stage, **{key: str(value) for key, value in fields.items()}}


def s2_records():
    return [
        record("native_keyboard", window="W", native_device="D", key="KeyA",
               state="Pressed", repeat="false", synthetic="false"),
        record("keyboard_admitted", work=12),
        record("native_keyboard", window="W", native_device="D", key="KeyA",
               state="Pressed", repeat="true", synthetic="false"),
        record("m13_settled", scope="Scope(1)", work=12, family="Keyboard",
               finality="Committed"),
        record("native_keyboard", window="W", native_device="D", key="KeyA",
               state="Released", repeat="false", synthetic="false"),
    ]


class Q1ValidatorTest(unittest.TestCase):
    def run_validation(self, fields, *, scenario="S2", source="headless",
                       exit_code=0, override=None, require_physical=False,
                       mutate_capture=None):
        with tempfile.TemporaryDirectory(prefix="q1-synthetic-") as directory:
            root = pathlib.Path(directory)
            events = [
                {"stream": "stderr", "fields": {"seq": str(i), "us": str(1000 * i), **row}}
                for i, row in enumerate(fields)
            ]
            if override is not None:
                override(events)
            (root / "events.jsonl").write_text(
                "".join(json.dumps(row) + "\n" for row in events)
            )
            (root / "memory.jsonl").write_text("")
            digests = {
                name: hashlib.sha256((root / name).read_bytes()).hexdigest()
                for name in ("events.jsonl", "memory.jsonl")
            }
            manifest = {
                "schema": "runenui-q1-run-v1", "sha": "synthetic",
                "scenario": scenario, "source": source, "warnings": [],
                "exit_code": exit_code, "observations": len(events),
                "psutil": False, "elapsed_s": 1.0, "sha256": digests,
            }
            (root / "manifest.json").write_text(json.dumps(manifest))
            if mutate_capture:
                mutate_capture(root)
            command = [sys.executable, str(VALIDATOR), str(root)]
            if require_physical:
                command.append("--require-physical")
            result = subprocess.run(command, capture_output=True, text=True, check=False)
            report_path = root / "validation.json"
            report = json.loads(report_path.read_text() if report_path.exists() else result.stdout)
            return result.returncode, report

    def test_valid_headless_s2_is_structurally_valid_but_not_physical(self):
        status, report = self.run_validation(s2_records())
        self.assertEqual(status, 0, report["issues"])
        self.assertIn("NOT_PHYSICAL_NATIVE_EVIDENCE", report["warnings"])
        self.assertEqual(report["observed_admitted_to_m13_matches"], 1)
        self.assertEqual(report["max_observed_native_held_keys"], 1)
        self.assertTrue(any("full S1-S8 campaign" in gap for gap in report["blocked"]))

    def test_duplicate_m13_settlement_rejected(self):
        fields = s2_records()
        fields.append(dict(fields[3]))
        status, report = self.run_validation(fields)
        self.assertNotEqual(status, 0)
        self.assertIn("DUPLICATE_SCOPE_SEQUENCE_M13_SETTLEMENT", report["issues"])

    def test_unsettled_admitted_work_rejected(self):
        fields = [field for field in s2_records() if field["stage"] != "m13_settled"]
        status, report = self.run_validation(fields)
        self.assertNotEqual(status, 0)
        self.assertIn("ADMITTED_INPUT_WITHOUT_OBSERVED_M13_SETTLEMENT", report["issues"])

    def test_nonzero_process_exit_rejected(self):
        status, report = self.run_validation(s2_records(), exit_code=7)
        self.assertNotEqual(status, 0)
        self.assertIn("NONZERO_OR_UNKNOWN_PROCESS_EXIT", report["issues"])

    def test_missing_required_field_rejected(self):
        def corrupt(events):
            del events[2]["fields"]["stage"]
        status, report = self.run_validation(s2_records(), override=corrupt)
        self.assertNotEqual(status, 0)
        self.assertIn("MISSING_Q1_REQUIRED_FIELDS", report["issues"])

    def test_observer_gap_rejected(self):
        def corrupt(events):
            events[2]["fields"]["seq"] = "44"
        status, report = self.run_validation(s2_records(), override=corrupt)
        self.assertNotEqual(status, 0)
        self.assertIn("OBSERVER_SEQUENCE_GAP_OR_REORDER", report["issues"])

    def test_stdout_observer_rejected(self):
        def corrupt(events):
            events[1]["stream"] = "stdout"
        status, report = self.run_validation(s2_records(), override=corrupt)
        self.assertNotEqual(status, 0)
        self.assertIn("INVALID_Q1_EVENT_ENVELOPE", report["issues"])

    def test_missing_repeat_rejected(self):
        fields = [field for field in s2_records() if field.get("repeat") != "true"]
        status, report = self.run_validation(fields)
        self.assertNotEqual(status, 0)
        self.assertIn("S2_MISSING_REPEAT", report["issues"])

    def test_headless_rejected_by_physical_gate(self):
        status, report = self.run_validation(s2_records(), require_physical=True)
        self.assertNotEqual(status, 0)
        self.assertIn("NOT_PHYSICAL", report["issues"])

    def test_checksum_mismatch_rejected(self):
        def mutate(root):
            with (root / "events.jsonl").open("a") as stream:
                stream.write("\n")
        status, report = self.run_validation(s2_records(), mutate_capture=mutate)
        self.assertNotEqual(status, 0)
        self.assertIn("SHA256_MISMATCH_OR_MISSING:events.jsonl", report["issues"])

    def test_missing_checksum_rejected(self):
        def mutate(root):
            path = root / "manifest.json"
            value = json.loads(path.read_text())
            del value["sha256"]["memory.jsonl"]
            path.write_text(json.dumps(value))
        status, report = self.run_validation(s2_records(), mutate_capture=mutate)
        self.assertNotEqual(status, 0)
        self.assertIn("MISSING_OR_UNEXPECTED_ARTIFACT_DIGEST", report["issues"])

    def test_collector_captures_built_process_stderr(self):
        collector = pathlib.Path(__file__).with_name("q1_capture.py")
        with tempfile.TemporaryDirectory(prefix="q1-process-") as directory:
            root = pathlib.Path(directory)
            output = root / "run"
            emitter = 'import sys; sys.stderr.write("RUNENUI_Q1 seq=0 us=0 stage=native_focus window=W focused=true\\n")'
            args = [
                sys.executable, str(collector), "--repo", str(root),
                "--scenario", "S1", "--source", "headless",
                "--out", str(output), "--",
                sys.executable, "-c", emitter,
            ]
            result = subprocess.run(args, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
            manifest = json.loads((output / "manifest.json").read_text())
            self.assertEqual(manifest["observations"], 1)
            self.assertEqual(manifest["executable_sha256"],
                             hashlib.sha256(pathlib.Path(sys.executable).read_bytes()).hexdigest())
            result = subprocess.run(
                [sys.executable, str(VALIDATOR), str(output)],
                capture_output=True, text=True, check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr + result.stdout)

    def test_collector_record_cap_fails_closed(self):
        collector = pathlib.Path(__file__).with_name("q1_capture.py")
        with tempfile.TemporaryDirectory(prefix="q1-cap-") as directory:
            root = pathlib.Path(directory)
            output = root / "run"
            emitter = ('import sys; '
                       'sys.stderr.write("RUNENUI_Q1 seq=0 us=0 stage=native_focus window=W focused=true\\n'
                       'RUNENUI_Q1 seq=1 us=1 stage=native_focus window=W focused=false\\n")')
            result = subprocess.run([
                sys.executable, str(collector), "--repo", str(root),
                "--scenario", "S1", "--source", "headless",
                "--max-records", "1", "--out", str(output), "--",
                sys.executable, "-c", emitter,
            ], capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            manifest = json.loads((output / "manifest.json").read_text())
            self.assertIn("max_records", manifest["warnings"])

    def test_unattested_physical_label_rejected(self):
        # This is a deliberately false label in a synthetic negative fixture.
        status, report = self.run_validation(s2_records(), source="physical")
        self.assertNotEqual(status, 0)
        self.assertIn("MISSING_OR_INVALID_OPERATOR_PROTOCOL", report["issues"])
        self.assertIn("NO_NATIVE_RSS_SAMPLING", report["issues"])

    def test_collector_rejects_missing_physical_protocol(self):
        collector = pathlib.Path(__file__).with_name("q1_capture.py")
        with tempfile.TemporaryDirectory(prefix="q1-unattested-") as directory:
            root = pathlib.Path(directory)
            result = subprocess.run([
                sys.executable, str(collector), "--repo", str(root),
                "--scenario", "S2", "--source", "physical",
                "--out", str(root / "run"), "--", sys.executable, "-c", "pass",
            ], capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("--operator-protocol", result.stderr)
            self.assertFalse((root / "run").exists())

    def test_incomplete_s5_ime_rejected(self):
        fields = [
            record("native_ime", window="W", kind="preedit", bytes=4),
            record("composition_started", generation=1, work=13),
            record("composition_ended", generation=1),
        ]
        status, report = self.run_validation(fields, scenario="S5")
        self.assertNotEqual(status, 0)
        self.assertIn("S5_MISSING_PREEDIT_COMMIT_OR_DISABLED", report["issues"])


if __name__ == "__main__":
    unittest.main()
