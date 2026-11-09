#!/usr/bin/env python3
"""merge-duplicate-coverage-mappings.py — one-off cleanup for BUG-557.

Coverage files could hold the same test mapping (same file, same lines text)
more than once in a scenario, and the same implementation mapping (same file,
same line list) more than once in a test mapping; re-pointing one copy left
the other stale. link-coverage now merges instead of duplicating and
`fspec validate` reports exact duplicates. This merges the existing ones in
spec/features/**/*.feature.coverage:

- exact duplicate test mappings of a scenario become the first of them, and
  their implementation mappings are joined (one per file, lines united,
  sorted), in order of first appearance;
- implementation mappings repeating a file within one test mapping are
  joined the same way.

Same-file test mappings with different ranges stay as they are, and nothing
else in a file changes. Only files that held a duplicate are rewritten.

Usage:
  python3 scripts/merge-duplicate-coverage-mappings.py [project-dir]   # default: current directory
"""

import json
import sys
from pathlib import Path


def join_impl(mappings):
    """One mapping per file with list lines united (sorted); a mapping whose
    lines are written as text ("10-20") joins only an identical one."""
    joined = []
    for mapping in mappings:
        lines = mapping["lines"]
        same_file = [m for m in joined if m["file"] == mapping["file"]]
        if isinstance(lines, list):
            target = next((m for m in same_file if isinstance(m["lines"], list)), None)
            if target is not None:
                target["lines"] = sorted(set(target["lines"]) | set(lines))
                continue
            joined.append({"file": mapping["file"], "lines": sorted(set(lines))})
        elif not any(m["lines"] == lines for m in same_file):
            joined.append({"file": mapping["file"], "lines": lines})
    return joined


def merge_scenario(scenario):
    merged = []
    index = {}
    changed = False
    for mapping in scenario.get("testMappings", []):
        key = (mapping.get("file"), mapping.get("lines"))
        impl = mapping.get("implMappings")
        if key in index:
            first = merged[index[key]]
            first["implMappings"] = join_impl(first.get("implMappings", []) + (impl or []))
            if not first["implMappings"]:
                del first["implMappings"]
            changed = True
            continue
        if impl and len({m["file"] for m in impl}) < len(impl):
            joined = join_impl(impl)
            if joined != impl:
                mapping["implMappings"] = joined
                changed = True
        index[key] = len(merged)
        merged.append(mapping)
    scenario["testMappings"] = merged
    return changed


def main():
    project = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    features = project / "spec" / "features"
    if not features.is_dir():
        sys.exit(f"No spec/features directory in {project}")
    files = mappings = 0
    for path in sorted(features.rglob("*.feature.coverage")):
        original = path.read_text(encoding="utf-8")
        coverage = json.loads(original)
        before = sum(len(s.get("testMappings", [])) for s in coverage.get("scenarios", []))
        changed = [merge_scenario(s) for s in coverage.get("scenarios", [])]
        if any(changed):
            after = sum(len(s["testMappings"]) for s in coverage["scenarios"])
            mappings += before - after
            files += 1
            text = json.dumps(coverage, indent=2, ensure_ascii=False)
            if original.endswith("\n"):
                text += "\n"
            path.write_text(text, encoding="utf-8")
    print(f"Merged {mappings} duplicate test mapping(s) in {files} coverage file(s)")


if __name__ == "__main__":
    main()
