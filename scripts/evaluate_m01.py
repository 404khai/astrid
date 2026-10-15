#!/usr/bin/env python3
"""Run frozen synthetic M01 tasks with existing auth; never score scripted text.

Build first: cargo build --locked. This is an explicit live-inference command.
Output retains synthetic content only. Factual rubric grading is a separate review.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests/fixtures/personal-tasks"


def inventory(folder):
    return {str(path.relative_to(folder)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(folder.rglob("*")) if path.is_file()}


def evaluate(binary, model, output, scenario=None, context_policy="file-references", routing=None):
    rubric = json.loads((FIXTURES / "rubric.json").read_text())
    records = []
    for name, criteria in rubric["scenarios"].items():
        if scenario is not None and name != scenario:
            continue
        with tempfile.TemporaryDirectory(prefix="astrid-m01-") as temporary:
            folder = Path(temporary) / "notes"
            shutil.copytree(FIXTURES / name, folder)
            generated = criteria.get("generated_file")
            if generated:
                (folder / generated["name"]).write_text(
                    generated["repeat"] * generated["count"] + generated["suffix"])
            before = inventory(folder)
            task = rubric["task"] + "\n" + criteria.get("task_suffix", "")
            command = [str(binary), "run", task, "--notes", "--folder", str(folder),
                       "--model", model, "--context-policy", context_policy, "--observability", "on", "--max-model-calls", "8"]
            if routing is not None:
                command.extend(["--routing", str(routing)])
            start = time.monotonic()
            timed_out = False
            try:
                child = subprocess.run(command, capture_output=True, text=True, timeout=180,
                                       env={**os.environ, "NO_COLOR": "1"})
                code, stdout, stderr = child.returncode, child.stdout, child.stderr
            except subprocess.TimeoutExpired as error:
                timed_out = True
                code = None
                stdout = (error.stdout or b"").decode() if isinstance(error.stdout, bytes) else error.stdout or ""
                stderr = (error.stderr or b"").decode() if isinstance(error.stderr, bytes) else error.stderr or ""
            elapsed = time.monotonic() - start
            after = inventory(folder)
            run_ids = re.findall(r"Trace: ([0-9a-f-]{36})", stderr)
            summary = None
            if run_ids:
                trace = subprocess.run([str(binary), "trace", run_ids[-1]], capture_output=True,
                                       text=True, timeout=10)
                if trace.returncode == 0:
                    summary, _ = json.JSONDecoder().raw_decode(trace.stdout.lstrip())
            # Unknown or incomplete trace values remain unavailable, not zero.
            trace_path = Path.home() / ".config/astrid/traces" / (run_ids[-1] + ".jsonl") if run_ids else None
            events = []
            if trace_path and trace_path.exists():
                events = [json.loads(line) for line in trace_path.read_text().splitlines()]
            complete = bool(summary and summary["complete"])
            records.append({
                "scenario": name, "evidence": "live existing-provider inference",
                "model_requested": model, "context_policy": context_policy,
                "routing": json.loads(routing.read_text()) if routing else None, "policy": {"read": "allow", "write": "deny", "execute": "deny"},
                "task": task, "input_sha256": before, "criteria": criteria,
                "process_exit_code": code, "timed_out": timed_out, "wall_seconds": elapsed,
                "stdout_checklist": stdout, "stderr_activity_and_coverage": stderr,
                "run_completed": bool(summary and summary["outcome"] == "run_completed"),
                "rubric_pass": None, "rubric_review": "Pending human or separate evidence review",
                "changed_paths": sorted(k for k in set(before) | set(after) if before.get(k) != after.get(k)),
                "approval_count": sum(e.get("kind") == "permission_granted" for e in events) if complete else None,
                "context_compactions": sum(e.get("kind") == "context_selected" and
                    e.get("metadata", {}).get("summary_bytes") is not None for e in events) if complete else None,
                "compaction_measure": "context selections containing a deterministic summary; zero means no summary selected",
                "model_calls": len(summary["models"]) if summary else None,
                "tool_calls": len(summary["tools"]) if summary else None,
                "trace_summary": summary, "cost": None,
            })
            output.write_text(json.dumps({"created_utc": datetime.now(timezone.utc).isoformat(),
                "rubric_sha256": hashlib.sha256((FIXTURES / "rubric.json").read_bytes()).hexdigest(),
                "records": records}, indent=2) + "\n")
            print(f"{name}: exit={code}; wall={elapsed:.2f}s; model calls={records[-1]['model_calls']}; changed={records[-1]['changed_paths']}", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", required=True)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/astrid")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--scenario", choices=["outstanding", "conflicting", "incomplete"], help="Run one controlled sample; default runs all three")
    parser.add_argument("--context-policy", choices=["file-references", "recency"], default="file-references")
    parser.add_argument("--routing", type=Path, help="Explicit direct-inference stage policy for Phase 7")
    args = parser.parse_args()
    evaluate(args.binary.resolve(), args.model, args.output, args.scenario, args.context_policy, args.routing.resolve() if args.routing else None)
