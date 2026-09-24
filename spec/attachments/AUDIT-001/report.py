#!/usr/bin/env python3
"""Merge laya results + structural findings into the final audit report.

v2: rebuilds the qid -> (feature, scenario, impl file) join from each batch's
in.json (source of truth), because the same feature::scenario name can appear
in two different coverage files.
"""
import json
import glob
from collections import Counter, defaultdict

A = "/tmp/fspec-audit"

res = json.load(open(f"{A}/laya_results.json"))
problems = json.load(open(f"{A}/problems.json"))

qmap = {}
for inp in sorted(glob.glob(f"{A}/batch_*/in.json")):
    d = json.load(open(inp))
    for qid, q in d["questions"].items():
        st = q["instructions"]
        feat = st.split("(feature ", 1)[1].split("): ", 1)[0] if "(feature " in st else None
        sc = st.split('SCENARIO "', 1)[1].split('"', 1)[0] if 'SCENARIO "' in st else "?"
        impl = st.split("IMPL ", 1)[1].split(" (", 1)[0] if "IMPL " in st else "?"
        qmap[qid] = {"feature": feat, "scenario": sc, "impl": impl}

feat_stats = defaultdict(Counter)
file_stats = defaultdict(Counter)
for qid, a in res.items():
    q = qmap.get(qid)
    if not q:
        continue
    feat_stats[q["feature"]][a["choice"]] += 1
    file_stats[q["impl"]][a["choice"]] += 1

total = len(res)
aligned = sum(1 for a in res.values() if a["choice"] == "aligned")
div = sum(1 for a in res.values() if a["choice"] == "divergent")
unv = sum(1 for a in res.values() if a["choice"] == "unverifiable")
print(f"LAYA AUDIT: {total} mappings judged")
print(f"  aligned {aligned} ({100*aligned/total:.1f}%)  divergent {div} ({100*div/total:.1f}%)  unverifiable {unv} ({100*unv/total:.1f}%)")

feat_rows = [(f, t, c["divergent"], c["aligned"], c["unverifiable"]) for f, c in feat_stats.items() if (t := sum(c.values())) >= 3 and c["divergent"] >= 2]
feat_rows.sort(key=lambda x: -x[2])
frows = sorted(((c["divergent"], f, sum(c.values()), c) for f, c in file_stats.items() if sum(c.values()) >= 3 and c["divergent"] >= 2), key=lambda x: -x[0])

json.dump({
    "totals": {"judged": total, "aligned": aligned, "divergent": div, "unverifiable": unv},
    "top_features": [{"feature": f, "judged": t, "divergent": dv, "aligned": al, "unverifiable": uv} for f, t, dv, al, uv in feat_rows],
    "top_files": [{"file": f, "judged": t, "divergent": dv, "aligned": c["aligned"], "unverifiable": c["unverifiable"]} for dv, f, t, c in frows],
    "structural": problems["summary"]["problem_counts"],
}, open(f"{A}/report.json", "w"), indent=2)
json.dump({qid: {"choice": a["choice"], "confidence": a["confidence"], "probabilities": a["probabilities"], **qmap[qid]} for qid, a in res.items() if qid in qmap}, open(f"{A}/per_qid.json", "w"), indent=1)
print(f"wrote {A}/report.json and {A}/per_qid.json")
