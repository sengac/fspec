#!/usr/bin/env python3
"""Build laya audit dataset: one `choice` question per (scenario, impl mapping)
where the impl file still exists. Question: does the mapped code actually
implement the scenario?

State (the ~950-token budget) carries: scenario name, test mapping, all impl
mappings (paths+line ranges), and the excerpt of the first impl mapping.
"""
import json
import glob
import os
from collections import Counter

ROOT = "/home/rquast/projects/fspec"
COV = os.path.join(ROOT, "spec", "features")
OUT = "/tmp/fspec-audit/records.jsonl"

SRC_CACHE = {}
def read_lines(path):
    if path in SRC_CACHE:
        return SRC_CACHE[path]
    p = os.path.join(ROOT, path)
    try:
        with open(p, encoding="utf-8", errors="replace") as f:
            lines = f.read().splitlines()
    except OSError:
        lines = None
    SRC_CACHE[path] = lines
    return lines

def normalize_lines(raw):
    """-> list of int line numbers (1-based)."""
    if isinstance(raw, str):
        out = []
        for part in raw.split(","):
            part = part.strip()
            if not part:
                continue
            if "-" in part:
                a, b = part.split("-", 1)
                out.extend(range(int(a), int(b) + 1))
            else:
                out.append(int(part))
        return out
    return [int(x) for x in raw]

def collapse(lines, pad=3):
    """Collapse 1-based line numbers into [(start,end)] ranges with padding,
    sorted, deduped, merged."""
    if not lines:
        return []
    s = sorted(set(int(x) for x in lines if x > 0))
    ranges = []
    for n in s:
        a, b = n - pad, n + pad
        if ranges and a <= ranges[-1][1] + 1:
            ranges[-1] = (ranges[-1][0], max(ranges[-1][1], b))
        else:
            ranges.append((a, b))
    return ranges

def excerpt(path, lines, max_chars=3600):
    """Code excerpt for the first impl mapping. Returns (text, covered)."""
    src = read_lines(path)
    if src is None:
        return "", 0
    nlines = len(src)
    valid = [x for x in lines if 1 <= x <= nlines]
    if not valid:
        return "", 0
    ranges = collapse(valid)
    # clip ranges to EOF
    clipped = [(a, min(b, nlines)) for a, b in ranges]
    parts = []
    total = 0
    included = 0
    for a, b in clipped:
        block = f"// {os.path.basename(path)}:{a}-{b}\n" + "\n".join(src[a - 1:b]) + "\n"
        if total + len(block) > max_chars and parts:
            parts.append("... (more mapped lines truncated)\n")
            break
        parts.append(block)
        total += len(block)
        included += b - a + 1
    text = "".join(parts)
    if len(text) > max_chars:
        text = text[:max_chars] + "\n... (truncated)"
    return text, len(valid)

def main():
    records = []
    skipped = Counter()
    impl_line_files = Counter()
    for cov in sorted(glob.glob(os.path.join(COV, "*.feature.coverage"))):
        feat = os.path.basename(cov)[:-len(".feature.coverage")]
        try:
            data = json.load(open(cov))
        except Exception:
            skipped["unparseable"] += 1
            continue
        for sc in data.get("scenarios", []):
            tms = sc.get("testMappings") or []
            if not tms:
                skipped["no_test_mapping"] += 1
                continue
            tm = tms[0]
            tm_info = {"file": tm.get("file"), "lines": tm.get("lines")}
            ims = tm.get("implMappings") or []
            if not ims:
                skipped["no_impl"] += 1
                continue
            usable = []
            for im in ims:
                f = im.get("file")
                raw = im.get("lines", [])
                lines = normalize_lines(raw)
                if not lines:
                    continue
                p = os.path.join(ROOT, f)
                if os.path.isfile(p):
                    usable.append((f, lines))
                    impl_line_files[f] += 1
                else:
                    skipped["impl_missing_file"] += 1
            if not usable:
                continue
            first_file, first_lines = usable[0]
            code, covered = excerpt(first_file, first_lines, max_chars=2300)
            if not code:
                skipped["impl_empty_excerpt"] += 1
                continue
            # Plain string state: laya serializes Value::String verbatim and
            # truncates the END of the sequence on overflow. Budget: the whole
            # prompt (template + state) fits ~980 tokens; keep metadata FIRST
            # so the scenario name survives any tail truncation, and bound
            # the code excerpt so metadata + code stay inside the window.
            state = (
                f'SCENARIO "{sc.get("name", "?")}" (feature {feat}): '
                f"TEST {tm_info['file']}:{tm_info['lines']}; "
                f"IMPL {first_file} ({len(first_lines)} mapped lines) among "
                + "; ".join(f"{f} ({len(l)} lines)" for f, l in usable[1:])
                + f".\nCODE UNDER REVIEW:\n{code}"
            )
            rec = {
                "qid": f"{feat}::{sc.get('name', '?').replace(':', '')[:80]}",
                "feature": feat,
                "scenario": sc.get("name", "?"),
                "state": state,
                "code": code,
            }
            records.append(rec)

    with open(OUT, "w") as f:
        for r in records:
            f.write(json.dumps(r) + "\n")

    state_size = [len((r["state"] + r["code"]).encode()) for r in records]
    import statistics
    print(json.dumps({
        "records": len(records),
        "skipped": dict(skipped),
        "impl_files_involved": len(impl_line_files),
        "avg_state_plus_code_chars": int(statistics.mean(state_size)) if state_size else 0,
        "p95_chars": sorted(state_size)[int(len(state_size) * 0.95)] if state_size else 0,
    }, indent=2))

if __name__ == "__main__":
    main()
