import json
import pathlib
import re
import subprocess


ROOT = pathlib.Path(__file__).resolve().parents[3]
TARGET = re.compile(r"rust/cli/tests/cmpct(?:052|054|057|058|059|071|079|082)_")
CACHE = {}
COMMANDS = []


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
            entries.append({"start": start, "end": end,
                            "name": re.search(r"\bfn (\w+)", match["text"])[1],
                            "test": bool(re.search(r"#\[(?:tokio::)?test(?:\(|\])", body))})
        CACHE[relative] = entries
    return CACHE[relative]


def function_lines(relative, names):
    entries = [entry for entry in functions(relative) if entry["name"] in names]
    if {entry["name"] for entry in entries} != set(names):
        raise ValueError(f"missing named implementation: {relative}: {names}")
    return sorted({number for entry in entries
                   for number in range(entry["start"], entry["end"] + 1)})


def run(arguments):
    command = ["fspec", *arguments]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    COMMANDS.append({"command": command, "exit": result.returncode,
                     "output": result.stdout + result.stderr})
    pathlib.Path("/tmp/cmpct-cli-supervised-relink.json").write_text(
        json.dumps(COMMANDS, indent=2) + "\n")
    if result.returncode:
        raise RuntimeError(COMMANDS[-1])


for feature in sorted((ROOT / "spec/features").glob("*.feature")):
    coverage_path = feature.with_suffix(".feature.coverage")
    coverage = json.loads(coverage_path.read_text()) if coverage_path.exists() else {}
    scenarios = coverage.get("scenarios", [])
    if not any(TARGET.match(mapping["file"]) for scenario in scenarios
               for mapping in scenario.get("testMappings", [])):
        if feature.stem != "structured-terminal-overflow-classification":
            continue
    scenario_names = re.findall(r"^\s*Scenario: (.+)$", feature.read_text(), re.M)
    for name in scenario_names:
        normalized = re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")
        scenario = next((scenario for scenario in scenarios if scenario["name"] == name), None)
        if scenario and scenario["testMappings"]:
            mapping, = scenario["testMappings"]
            test_file = mapping["file"]
            start, end = map(int, mapping["lines"].split("-"))
            candidates = [entry for entry in functions(test_file)
                          if entry["test"] and entry["name"] == normalized]
            if not candidates:
                candidates = [entry for entry in functions(test_file)
                              if entry["test"] and entry["start"] <= end and start <= entry["end"]]
            entry, = candidates
            implementations = {item["file"]: sorted(set(item["lines"]))
                               for item in mapping["implMappings"]}
        else:
            test_file = ("rust/cli/tests/cmpct054_structured_overflow.rs"
                         if feature.stem == "structured-terminal-overflow-classification"
                         else "rust/cli/tests/cmpct079_recovery_isolation.rs")
            entry, = [entry for entry in functions(test_file)
                      if entry["test"] and entry["name"] == normalized]
            implementation = ("rust/cli/src/interactive/overflow_classification.rs"
                              if "054_" in test_file else "rust/cli/src/session/recovery_view.rs")
            implementations = {implementation: function_lines(
                implementation, [entry["name"] for entry in functions(implementation)])}
        for relative in list(implementations):
            if relative == "rust/cli/src/compaction_operation.rs":
                implementations[relative] = function_lines(
                    relative, [entry["name"] for entry in functions(relative)])
            elif relative == "rust/cli/src/compactor_sub_agent.rs":
                implementations[relative] = function_lines(relative, ["run_compactor_sub_agent_round"])
            elif relative == "rust/cli/src/session/restore_compaction.rs":
                implementations[relative] = function_lines(relative, ["restore_compaction_boundary"])
            elif relative == "rust/cli/src/interactive/error_classifiers.rs":
                implementations[relative] = function_lines(relative, ["is_context_overflow_error"])
                classifier = "rust/cli/src/interactive/overflow_classification.rs"
                implementations[classifier] = function_lines(
                    classifier, ["is_context_overflow", "http_overflow", "has_overflow_code"])
            elif relative == "rust/cli/src/interactive/stream_loop.rs":
                if feature.stem == "pre-prompt-compaction-isolation":
                    implementations[relative] = list(range(295, 341))
                elif feature.stem == "compaction-inference-handoff":
                    implementations[relative] = list(range(1010, 1030))
        if scenario:
            run(["unlink-coverage", feature.stem, "--scenario", name, "--all"])
        base = ["link-coverage", feature.stem, "--scenario", name, "--test-file", test_file]
        run([*base, "--test-lines", f"{entry['start']}-{entry['end']}"])
        for relative, numbers in implementations.items():
            run([*base, "--impl-file", relative, "--impl-lines", ",".join(map(str, numbers))])
    print(f"{feature.stem}: {len(scenario_names)} scenarios relinked", flush=True)
