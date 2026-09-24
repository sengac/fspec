#!/usr/bin/env python3
"""Drive the laya semantic audit in parallel.

records.jsonl  ->  batch dirs (each = one laya `answer` invocation:
                  one model load, all questions in one forward pass)
                ->  parallel `laya answer` runs
                ->  merged laya_results.json  {qid: {choice, confidence, probabilities}}

State+code for each question is embedded in the question instructions;
state is a constant. Model loaded once per batch of B questions.
"""
import json
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

LAYA = os.environ.get("LAYA_BIN", "laya")
MODEL = os.environ.get("LAYA_MODEL_DIR", "/home/rquast/.cache/laya-rs/laya-typed-decisions")
WORK = Path("/tmp/fspec-audit")
IN = Path(sys.argv[1]) if len(sys.argv) > 1 else WORK / "records.jsonl"
OUT = WORK / "laya_results.json"
BATCH = int(os.environ.get("LAYA_BATCH", "60"))
PARALLEL = int(os.environ.get("LAYA_PARALLEL", "8"))

QUESTION_TEMPLATE = (
    "You are auditing a code-coverage mapping. The STATE first names a Gherkin "
    "SCENARIO and where the code is mapped, then shows the CODE UNDER REVIEW. "
    "Decide: does the mapped code actually implement the behavior the scenario "
    "describes? Read the code carefully — look for the scenario's core action "
    "in the mapped lines; if the behavior is clearly absent, renamed, or the "
    "mapped lines show different functionality, that is divergent.\n\n"
    "{STATE}\n\n"
    "Classify the mapping. Options:\n"
    "aligned: the mapped code visibly implements the scenario's behavior (the scenario's "
    "core action is present and functional here).\n"
    "divergent: the code exists and is related, but it does NOT do what the scenario says "
    "(missing behavior, different semantics, dead/duplicated, or wrong lines mapped).\n"
    "unverifiable: cannot decide from this excerpt alone (logic lives elsewhere, excerpt "
    "truncated, or scenario describes runtime behavior not visible in code)."
)

def build_batches(records):
    dirs = []
    for i in range(0, len(records), BATCH):
        chunk = records[i : i + BATCH]
        d = WORK / f"batch_{i // BATCH:04d}"
        d.mkdir(exist_ok=True)
        questions = {}
        for r in chunk:
            qid = r["qid"]
            questions[qid] = {
                "type": "choice",
                "instructions": QUESTION_TEMPLATE.format(STATE=r["state"]),
                "criteria": {
                    "aligned": "the mapped code implements the scenario",
                    "divergent": "the mapped code does not implement it",
                    "unverifiable": "cannot decide from this excerpt",
                },
            }
        inp = d / "in.json"
        inp.write_text(json.dumps({"state": "code-mapping audit", "questions": questions}))
        dirs.append({"dir": d, "out": d / "out.json", "count": len(chunk), "records": chunk})
    return dirs

def run_one(job):
    d, out, inp = job["dir"], job["out"], job["dir"] / "in.json"
    if out.is_file():
        try:
            json.loads(out.read_text())
            return job["records"], True
        except Exception:
            pass
    r = subprocess.run(
        [LAYA, "answer", str(inp), str(out), "--model", MODEL],
        capture_output=True, text=True, timeout=1800,
    )
    if r.returncode != 0:
        (out).write_text(r.stderr[-4000:])
        return job["records"], False
    try:
        json.loads(out.read_text())
        return job["records"], True
    except Exception:
        return job["records"], False

def main():
    records = [json.loads(l) for l in open(IN)]
    print(f"{len(records)} records", file=sys.stderr)
    jobs = build_batches(records)
    print(f"{len(jobs)} batches of {BATCH}, parallelism {PARALLEL}", file=sys.stderr)

    results = {}
    failed_jobs = []
    done = 0
    with ThreadPoolExecutor(max_workers=PARALLEL) as ex:
        for recs, ok in ex.map(run_one, jobs):
            done += 1
            if not ok:
                failed_jobs.append((done, recs[0]["qid"][:60]))
                continue
            outp = WORK / f"batch_{(done-1):04d}" / "out.json"
            try:
                res = json.loads(outp.read_text())
                for rec in recs:
                    a = res.get(rec["qid"])
                    if not a:
                        continue
                    results[rec["qid"]] = {
                        "choice": a.get("choice"),
                        "confidence": a.get("confidence"),
                        "probabilities": a.get("probabilities"),
                        "act_probability": a.get("act_probability"),
                    }
            except Exception:
                failed_jobs.append((done, "parse-fail"))
            if done % 10 == 0 or done == len(jobs):
                print(f"progress {done}/{len(jobs)} batches, {len(results)} answers, failed={len(failed_jobs)}", file=sys.stderr)

    OUT.write_text(json.dumps(results, indent=1))
    print(f"\nwrote {len(results)} answers to {OUT}", file=sys.stderr)
    if failed_jobs:
        print(f"FAILED BATCHES: {failed_jobs[:20]}", file=sys.stderr)

if __name__ == "__main__":
    main()
