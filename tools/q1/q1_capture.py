#!/usr/bin/env python3
"""Q1 native observer process wrapper. No input injection or runtime policy changes."""
import argparse
import datetime
import hashlib
import json
import pathlib
import platform
import queue
import shlex
import subprocess
import sys
import threading
import time

try:
    import psutil
except ImportError:
    psutil = None


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(131072), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--scenario", required=True, choices=[f"S{i}" for i in range(1, 9)])
    parser.add_argument("--out", required=True, help="Fresh directory; existing paths are rejected")
    parser.add_argument("--source", required=True, choices=["physical", "virtual", "headless"])
    parser.add_argument("--max-seconds", type=int, default=900)
    parser.add_argument("--max-records", type=int, default=1000000)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or min(args.max_seconds, args.max_records) <= 0:
        parser.error("A command and positive capture limits are required")
    target = pathlib.Path(args.out)
    target.mkdir(parents=True, exist_ok=False)
    repo = pathlib.Path(args.repo).resolve()
    try:
        head = subprocess.check_output(
            ["git", "-C", str(repo), "rev-parse", "HEAD"],
            text=True, stderr=subprocess.DEVNULL
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        head = None
    executable = pathlib.Path(command[0])
    if not executable.is_absolute():
        executable = repo / executable
    executable = executable.resolve()
    if not executable.is_file():
        parser.error("Q1 must measure a previously built executable file, not a PATH command")
    executable_digest = sha256(executable)
    manifest = {
        "schema": "runenui-q1-run-v1", "sha": head,
        "scenario": args.scenario, "source": args.source,
        "command": command, "repo": str(repo),
        "executable": str(executable), "executable_sha256": executable_digest,
        "os": platform.platform(), "cpu": platform.processor(),
        "started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "warning": "physical is operator attestation, NOT inferred from runner OS"
    }
    (target / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    env = dict(__import__("os").environ)
    env["RUNENUI_Q1"] = "1"
    env.setdefault("RUNENUI_REFERENCE_PROOF", "0")
    received = queue.Queue(maxsize=8192)
    warnings = []
    process = subprocess.Popen(
        command, cwd=repo, env=env, text=True, errors="replace",
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=1
    )

    def drain(stream, name):
        for line in stream:
            if not line.startswith("RUNENUI_Q1 "):
                continue  # Other Debug/proof logs may contain sensitive text.
            try:
                parts = shlex.split(line[len("RUNENUI_Q1 "):])
                fields = dict(part.split("=", 1) for part in parts if "=" in part)
                received.put_nowait({"fields": fields, "stream": name})
            except (ValueError, queue.Full):
                warnings.append("invalid_or_overfull_observation_buffer")
        received.put({"eof": True, "stream": name})

    for stream, name in [(process.stdout, "stdout"), (process.stderr, "stderr")]:
        threading.Thread(target=drain, args=(stream, name), daemon=True).start()
    memory_proc = psutil.Process(process.pid) if psutil else None
    t0 = time.monotonic()
    count = 0
    eof = 0
    with (target / "events.jsonl").open("w") as events, (target / "memory.jsonl").open("w") as memory:
        try:
            while eof < 2 or process.poll() is None:
                if time.monotonic() - t0 > args.max_seconds:
                    warnings.append("timeout")
                    process.terminate()
                    break
                if memory_proc:
                    try:
                        stats = memory_proc.memory_info()
                        memory.write(json.dumps({
                            "elapsed_s": time.monotonic() - t0,
                            "rss_bytes": stats.rss, "vms_bytes": stats.vms
                        }) + "\n")
                    except psutil.Error:
                        memory_proc = None
                try:
                    item = received.get(timeout=0.05)
                except queue.Empty:
                    continue
                if item.get("eof"):
                    eof += 1
                else:
                    if count >= args.max_records:
                        warnings.append("max_records")
                        process.terminate()
                        break
                    events.write(json.dumps(item, separators=(",", ":")) + "\n")
                    count += 1
            while not received.empty() and count < args.max_records:
                item = received.get_nowait()
                if not item.get("eof"):
                    events.write(json.dumps(item, separators=(",", ":")) + "\n")
                    count += 1
        except KeyboardInterrupt:
            warnings.append("operator_interrupt")
            process.terminate()
    try:
        result = process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        result = process.wait()
    manifest.update({
        "ended_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "elapsed_s": time.monotonic() - t0,
        "exit_code": result, "observations": count,
        "warnings": sorted(set(warnings)), "psutil": psutil is not None,
        "memory_samples": len((target / "memory.jsonl").read_text().splitlines()),
        "sha256": {
            name: sha256(target / name)
            for name in ("events.jsonl", "memory.jsonl")
        },
    })
    (target / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"dir": str(target), "observations": count, "status": result, "warnings": warnings}))
    return 2 if warnings or result else 0


if __name__ == "__main__":
    sys.exit(main())
