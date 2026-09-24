#!/usr/bin/env python3
"""Structural audit of fspec coverage mappings.

Walks spec/features/*.feature.coverage and validates every mapping:
- file exists
- line references within file length
- impl line lists non-empty / suspiciously huge
- test line ranges non-empty

Outputs JSON summary + per-problem records.
"""
import json
import glob
import os
import sys
from collections import Counter, defaultdict

ROOT = "/home/rquast/projects/fspec"
COV = os.path.join(ROOT, "spec", "features")

def parse_range(s):
    # "66-101" or "66" or comma-separated "55,191-227"
    parts = str(s).split(",")
    los, his = [], []
    for p in parts:
        p = p.strip()
        if not p:
            continue
        if "-" in p:
            a, b = p.split("-", 1)
            los.append(int(a))
            his.append(int(b))
        else:
            v = int(p)
            los.append(v)
            his.append(v)
    if not los:
        raise ValueError(f"bad range {s!r}")
    return min(los), max(his)

file_cache = {}
def file_lines(n):
    if n in file_cache:
        return file_cache[n]
    path = os.path.join(ROOT, n)
    if not os.path.isfile(path):
        file_cache[n] = None
        return None
    with open(path, encoding="utf-8", errors="replace") as f:
        file_cache[n] = sum(1 for _ in f)
    return file_cache[n]

problems = []
stats = Counter()
impl_by_file = Counter()
test_by_file = Counter()

for cov in sorted(glob.glob(os.path.join(COV, "*.feature.coverage"))):
    feat = os.path.basename(cov)[: -len(".feature.coverage")]
    try:
        data = json.load(open(cov))
    except Exception as e:
        problems.append({"feature": feat, "kind": "unparseable", "detail": str(e)})
        stats["features_unparseable"] += 1
        continue

    for sc in data.get("scenarios", []):
        stats["scenarios"] += 1
        name = sc.get("name", "?")
        tms = sc.get("testMappings", [])
        if not tms:
            stats["scenarios_no_test_mapping"] += 1
            continue
        for tm in tms:
            tf = tm.get("file")
            tlines = tm.get("lines")
            nlines = file_lines(tf)
            if nlines is None:
                problems.append({"feature": feat, "scenario": name, "kind": "test_file_missing", "file": tf})
                stats["test_file_missing"] += 1
            else:
                test_by_file[tf] += 1
                lo, hi = parse_range(str(tlines))
                if lo > hi or lo < 1:
                    problems.append({"feature": feat, "scenario": name, "kind": "test_range_bad", "file": tf, "lines": tlines})
                    stats["test_range_bad"] += 1
                elif hi > nlines:
                    problems.append({"feature": feat, "scenario": name, "kind": "test_lines_beyond_eof", "file": tf, "lines": tlines, "file_lines": nlines})
                    stats["test_lines_beyond_eof"] += 1

            ims = tm.get("implMappings", [])
            if not ims:
                problems.append({"feature": feat, "scenario": name, "kind": "no_impl_mapping", "testFile": tf})
                stats["no_impl_mapping"] += 1
            for im in ims:
                f = im.get("file")
                raw_lines = im.get("lines", [])
                if isinstance(raw_lines, str):
                    lo, hi = parse_range(raw_lines)
                    lines = list(range(lo, hi + 1))
                else:
                    lines = [int(x) for x in raw_lines]
                if not lines:
                    problems.append({"feature": feat, "scenario": name, "kind": "impl_lines_empty", "file": f})
                    stats["impl_lines_empty"] += 1
                    continue
                impl_by_file[f] += 1
                nlines = file_lines(f)
                if nlines is None:
                    problems.append({"feature": feat, "scenario": name, "kind": "impl_file_missing", "file": f, "n_lines": len(lines)})
                    stats["impl_file_missing"] += 1
                else:
                    mx = max(lines)
                    if mx > nlines:
                        problems.append({"feature": feat, "scenario": name, "kind": "impl_lines_beyond_eof", "file": f, "max": mx, "file_lines": nlines})
                        stats["impl_lines_beyond_eof"] += 1
                    if len(lines) > 300:
                        problems.append({"feature": feat, "scenario": name, "kind": "impl_suspiciously_large", "file": f, "n_lines": len(lines)})
                        stats["impl_suspiciously_large"] += 1
                    # line density: is it a contiguous block or scattered?
                    span = mx - min(lines) + 1
                    if span > 0 and len(lines) / span < 0.3:
                        problems.append({"feature": feat, "scenario": name, "kind": "impl_scattered", "file": f, "n_lines": len(lines), "span": span, "density": round(len(lines) / span, 2)})
                        stats["impl_scattered"] += 1

summary = {
    "features": len(glob.glob(os.path.join(COV, "*.feature.coverage"))),
    "scenarios": stats["scenarios"],
    "scenarios_no_test_mapping": stats["scenarios_no_test_mapping"],
    "distinct_impl_files": len(impl_by_file),
    "distinct_test_files": len(test_by_file),
    "problem_counts": dict(stats),
    "top_impl_files": impl_by_file.most_common(10),
    "top_test_files": test_by_file.most_common(10),
}
print(json.dumps(summary, indent=2))
with open("/tmp/fspec-audit/problems.json", "w") as f:
    json.dump({"summary": summary, "problems": problems}, f, indent=1)
print(f"\nwrote {len(problems)} problem records to /tmp/fspec-audit/problems.json", file=sys.stderr)
