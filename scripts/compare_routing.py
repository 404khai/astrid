#!/usr/bin/env python3
"""Controlled synthetic quality comparisons using existing CLI/auth. No mock quality scores.

Build first. Default two counterbalanced repetitions for each task/configuration.
Notes factual grading stays pending for a separate rubric review. Coding artifacts
are independently verified. Every episode gets a fresh copied fixture/workspace.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests/fixtures/evaluation/comparison.json"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def inventory(root):
    return {str(p.relative_to(root)): digest(p.read_bytes()) for p in sorted(root.rglob("*"))
            if p.is_file() and "target" not in p.relative_to(root).parts}


def run_process(command, folder, timeout):
    started = time.monotonic()
    try:
        result = subprocess.run(command, cwd=folder, capture_output=True, text=True,
                                timeout=timeout, env={**os.environ, "NO_COLOR": "1"})
        return {"exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr,
                "timed_out": False, "wall_seconds": time.monotonic() - started}
    except subprocess.TimeoutExpired as error:
        def decode(value):
            return value.decode(errors="replace") if isinstance(value, bytes) else value or ""
        return {"exit_code": None, "stdout": decode(error.stdout), "stderr": decode(error.stderr),
                "timed_out": True, "wall_seconds": time.monotonic() - started}


def make_fixture(folder, task):
    source = ROOT / "tests/fixtures" / ("greeting" if task == "coding" else "personal-tasks/outstanding")
    shutil.copytree(source, folder)
    if task == "coding":
        code = (folder / "src/lib.rs").read_text()
        assert "let name = name.trim();" in code
        (folder / "src/lib.rs").write_text(code.replace("let name = name.trim();", "let name = name;"))
        (folder / "GREETING.txt").write_text("wrong\n")
    return source


def coding_grade(folder, before, original_tests, verification):
    after = inventory(folder)
    try:
        return (verification["exit_code"] == 0 and "2 passed" in verification["stdout"]
                and (folder / "GREETING.txt").read_bytes() == b"Hello, Astrid!\n"
                and b"#[cfg(test)]" in (folder / "src/lib.rs").read_bytes()
                and (folder / "src/lib.rs").read_bytes().split(b"#[cfg(test)]", 1)[-1] == original_tests
                and all(after.get(p) == before[p] for p in ["Cargo.toml", "AGENTS.md"]))
    except (OSError, KeyError):
        return False


def compare(binary, output, repetitions=2):
    manifest_bytes = MANIFEST.read_bytes()
    manifest = json.loads(manifest_bytes)
    report = {"created_utc": datetime.now(timezone.utc).isoformat(), "evidence": "live existing-provider inference",
              "manifest": manifest, "manifest_sha256": digest(manifest_bytes),
              "runner_sha256": digest(Path(__file__).read_bytes()), "records": []}
    for task, definition in manifest["tasks"].items():
        for trial in range(repetitions):
            for configuration in (["fixed", "routed"] if trial % 2 == 0 else ["routed", "fixed"]):
                with tempfile.TemporaryDirectory(prefix="astrid-routing-comparison-") as temporary:
                    base = Path(temporary)
                    folder = base / "workspace"
                    make_fixture(folder, task)
                    before = inventory(folder)
                    tests = ((folder / "src/lib.rs").read_bytes().split(b"#[cfg(test)]", 1)[-1]
                             if task == "coding" else None)
                    routing = {"initial_effort": manifest["effort"], "stages": []}
                    if configuration == "routed":
                        routing["stages"] = [{"from_call": 2, "model": "gpt-5.6-terra",
                                              "class": definition["class"], "effort": manifest["effort"]}]
                    policy_file = base / "routing.json"
                    policy_file.write_text(json.dumps(routing))
                    command = [str(binary), "run", definition["task"], "--model", manifest["initial_model"],
                               "--routing", str(policy_file), "--observability", "on", "--max-model-calls", "8",
                               "--context-policy", manifest["context_policy"], "--context-tokens", str(manifest["context_tokens"]),
                               "--response-reserve", str(manifest["response_reserve"]), "--context-bytes", str(manifest["context_bytes"])]
                    command.extend(["--notes", "--folder", str(folder)] if task == "notes" else ["--mode", "unbound", "--shell-timeout", "30"])
                    result = run_process(command, folder, 240)
                    after = inventory(folder)
                    summary = None
                    ids = re.findall(r"Trace: ([0-9a-f-]{36})", result["stderr"])
                    if ids:
                        trace = run_process([str(binary), "trace", ids[-1]], folder, 15)
                        if trace["exit_code"] == 0:
                            summary, _ = json.JSONDecoder().raw_decode(trace["stdout"].lstrip())
                    verification = run_process(["cargo", "test", "--offline"], folder, 45) if task == "coding" else None
                    record = {"task": task, "trial": trial + 1, "configuration": configuration, "routing": routing,
                              "initial_fixture_sha256": before, "task_sha256": digest(definition["task"].encode()),
                              "process": result, "changed_paths": sorted(p for p in set(before) | set(after) if before.get(p) != after.get(p)),
                              "trace_summary": summary, "independent_verification": verification,
                              "rubric": definition["rubric"], "task_success": None, "quality_review": "Pending separate solo rubric review",
                              "model_verification_command_observed": bool(re.search(r"shell\s+.*cargo test", result["stderr"])) if task == "coding" else None,
                              "cost": None}
                    if task == "coding":
                        record["task_success"] = coding_grade(folder, before, tests, verification)
                        record["quality_review"] = "Independent artifact/test rubric evaluated; run completion is separate"
                    if summary:
                        calls = list(summary["models"].values())
                        record["usage"] = {k: sum(c["usage"][k] for c in calls)
                                           if summary["complete"] and all(c.get("usage") and not c["usage"]["invalid"] and c["usage"][k] is not None for c in calls)
                                           else None for k in ["input_tokens", "output_tokens", "cached_tokens"]}
                        record["model_calls"] = len(calls)
                        record["tool_calls"] = len(summary["tools"])
                        record["context_compactions"] = sum(c.get("summary_bytes") is not None for c in summary["contexts"] if "budget" in c)
                    report["records"].append(record)
                    output.write_text(json.dumps(report, indent=2) + "\n")
                    print(f"{task} {trial+1} {configuration}: exit={result['exit_code']}, artifact/quality={record['task_success']}, wall={result['wall_seconds']:.2f}s", flush=True)
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/astrid")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, choices=range(1, 5), default=2)
    args = parser.parse_args()
    compare(args.binary.resolve(), args.output, args.repetitions)
