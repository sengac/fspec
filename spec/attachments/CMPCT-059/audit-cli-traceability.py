import hashlib
import json
import pathlib
import re
import shutil
import subprocess
import tempfile


ROOT = pathlib.Path(__file__).resolve().parents[3]
TARGET = re.compile(r"rust/cli/tests/cmpct(?:052|054|057|058|059|071|079|082)_")
CACHE = {}


def functions(relative):
    if relative not in CACHE:
        result = subprocess.run(
            ["ast-grep", "scan", "--inline-rules",
             "id: functions\nlanguage: Rust\nrule:\n  kind: function_item",
             "--json=compact", relative],
            cwd=ROOT, capture_output=True, text=True, check=True,
        )
        source = (ROOT / relative).read_text().splitlines()
        entries = []
        for match in json.loads(result.stdout):
            start = match["range"]["start"]["line"] + 1
            end = match["range"]["end"]["line"] + 1
            while start > 1 and source[start - 2].lstrip().startswith(("#[", "///")):
                start -= 1
            body = "\n".join(source[start - 1:end])
            entries.append({"start": start, "end": end, "body": body,
                            "name": re.search(r"\bfn (\w+)", match["text"])[1],
                            "test": bool(re.search(r"#\[(?:tokio::)?test(?:\(|\])", body))})
        CACHE[relative] = entries
    return CACHE[relative]


def run(command, directory):
    result = subprocess.run(command, cwd=directory, capture_output=True, text=True)
    return {"command": command, "exit": result.returncode,
            "output": result.stdout + result.stderr}


rows = []
all_tests = set()
with tempfile.TemporaryDirectory(prefix="cmpct-cli-traceability-") as directory:
    scratch = pathlib.Path(directory)
    (scratch / "spec/features").mkdir(parents=True)
    (scratch / "rust").symlink_to(ROOT / "rust", target_is_directory=True)
    for name in ["tags.json", "work-units.json", "prefixes.json"]:
        source = ROOT / "spec" / name
        if source.exists():
            shutil.copy2(source, scratch / "spec" / name)
    for feature in sorted((ROOT / "spec/features").glob("*.feature")):
        coverage_path = feature.with_suffix(".feature.coverage")
        coverage = json.loads(coverage_path.read_text()) if coverage_path.exists() else {}
        mappings = [mapping for scenario in coverage.get("scenarios", [])
                    for mapping in scenario.get("testMappings", [])]
        if not any(TARGET.match(mapping["file"]) for mapping in mappings):
            if feature.stem != "structured-terminal-overflow-classification":
                continue
        source = feature.read_text()
        scenarios = re.split(r"^\s*Scenario(?: Outline)?: ", source, flags=re.M)[1:]
        row = {"feature": feature.stem, "scenarios": len(scenarios), "tests": [],
               "errors": [], "commands": [], "sha256": {}, "implementations": []}
        expected_names = [scenario.splitlines()[0] for scenario in scenarios]
        if expected_names != [scenario["name"] for scenario in coverage.get("scenarios", [])]:
            row["errors"].append("scenario inventory mismatch")
        shutil.copy2(feature, scratch / "spec/features" / feature.name)
        if coverage_path.exists():
            shutil.copy2(coverage_path, scratch / "spec/features" / coverage_path.name)
        for path in [feature, coverage_path]:
            if path.exists():
                row["sha256"][str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
        total_lines = 0
        targets = set()
        for scenario_text, scenario in zip(scenarios, coverage.get("scenarios", [])):
            steps = re.findall(r"^\s*((?:Given|When|Then|And|But) .+)$", scenario_text, re.M)
            scenario_mappings = scenario.get("testMappings", [])
            if len(scenario_mappings) != 1:
                row["errors"].append(f"not one test: {scenario['name']}")
            for mapping in scenario_mappings:
                targets.add(mapping["file"])
                start, end = map(int, mapping["lines"].split("-"))
                matching = [entry for entry in functions(mapping["file"])
                            if entry["test"] and start <= entry["start"]
                            and entry["end"] <= end]
                if len(matching) != 1:
                    row["errors"].append(f"not a complete test: {scenario['name']}")
                else:
                    entry = matching[0]
                    key = (mapping["file"], entry["name"])
                    if key in all_tests:
                        row["errors"].append(f"duplicate test: {entry['name']}")
                    all_tests.add(key)
                    row["tests"].append(key)
                    if not all("@step " + step in entry["body"] for step in steps):
                        row["errors"].append(f"missing exact steps: {entry['name']}")
                total_lines += end - start + 1
                if not mapping.get("implMappings"):
                    row["errors"].append(f"missing implementation: {scenario['name']}")
                for implementation in mapping.get("implMappings", []):
                    path = implementation["file"]
                    numbers = implementation["lines"]
                    source_lines = (ROOT / path).read_text().splitlines()
                    if not numbers or not all(1 <= number <= len(source_lines) for number in numbers):
                        row["errors"].append(f"bad implementation bounds: {path}")
                    if not all(any(entry["start"] <= number <= entry["end"]
                                   for entry in functions(path)) for number in numbers):
                        row["errors"].append(f"implementation outside function: {path}")
                    total_lines += len(numbers)
                    if path not in row["implementations"]:
                        row["implementations"].append(path)
                row["commands"].append(run(
                    ["fspec", "link-coverage", feature.stem, "--scenario", scenario["name"],
                     "--test-file", mapping["file"], "--test-lines", mapping["lines"]], scratch))
        for target in targets:
            row["sha256"][target] = hashlib.sha256((ROOT / target).read_bytes()).hexdigest()
            unmapped = {entry["name"] for entry in functions(target) if entry["test"]} - {
                name for path, name in row["tests"] if path == target}
            if unmapped:
                row["errors"].append(f"unmapped tests: {sorted(unmapped)}")
        for path in row["implementations"]:
            row["sha256"][path] = hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
        stats = coverage.get("stats", {})
        expected = {"totalScenarios": len(scenarios), "coveredScenarios": len(scenarios),
                    "coveragePercent": 100, "testFiles": sorted(targets),
                    "implFiles": row["implementations"], "totalLinesCovered": total_lines}
        if any(stats.get(key) != value for key, value in expected.items()):
            row["errors"].append(f"stats mismatch; expected {expected}")
        for command in [["fspec", "validate", f"spec/features/{feature.name}"],
                        ["fspec", "format", f"spec/features/{feature.name}"],
                        ["fspec", "audit-coverage", feature.stem]]:
            row["commands"].append(run(command, scratch))
        if (scratch / "spec/features" / feature.name).read_text() != source:
            row["errors"].append("feature format differs")
        rows.append(row)
    tags = run(["fspec", "validate-tags"], scratch)
result = {"features": rows, "tags": tags}
pathlib.Path("/tmp/cmpct-cli-supervised-traceability.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps({"features": [{key: row[key] for key in ["feature", "scenarios", "errors"]}
                               for row in rows], "tags": tags}, indent=2))
raise SystemExit(bool(tags["exit"] or any(row["errors"] or any(
    command["exit"] for command in row["commands"]) for row in rows)))
