import hashlib
import json
import pathlib
import re
import shutil
import subprocess
import tempfile


ROOT = pathlib.Path(__file__).resolve().parents[3]
FEATURES = sorted(
    path.stem for path in (ROOT / "spec/features").glob("*.feature")
    if re.search(r"^@CMPCT-083\s*$", path.read_text(), re.M)
)
CACHE = {}
SOURCE_HASHES = {}


def functions(relative):
    if relative not in CACHE:
        before = hashlib.sha256((ROOT / relative).read_bytes()).hexdigest()
        result = subprocess.run(
            [
                "ast-grep", "scan", "--inline-rules",
                "id: functions\nlanguage: Rust\nrule:\n  kind: function_item",
                "--json=compact", relative,
            ], cwd=ROOT, capture_output=True, text=True, check=True,
        )
        assert hashlib.sha256((ROOT / relative).read_bytes()).hexdigest() == before
        SOURCE_HASHES[relative] = before
        CACHE[relative] = json.loads(result.stdout)
    return CACHE[relative]


def run(command, directory):
    result = subprocess.run(command, cwd=directory, capture_output=True, text=True)
    print(json.dumps({"command": command, "exit": result.returncode,
                      "output": result.stdout + result.stderr}))
    assert result.returncode == 0, command


with tempfile.TemporaryDirectory(prefix="cmpct083-traceability-") as directory:
    scratch = pathlib.Path(directory)
    (scratch / "spec/features").mkdir(parents=True)
    (scratch / "rust").symlink_to(ROOT / "rust", target_is_directory=True)
    for name in ["tags.json", "work-units.json", "prefixes.json"]:
        source = ROOT / "spec" / name
        if source.exists():
            shutil.copy2(source, scratch / "spec" / name)
    total_tests = 0
    total_scenarios = 0
    all_tests = set()
    all_targets = set()
    for feature in FEATURES:
        relative = f"spec/features/{feature}.feature"
        source = ROOT / relative
        text = source.read_text()
        coverage_path = source.with_suffix(".feature.coverage")
        for path in [source, coverage_path]:
            SOURCE_HASHES[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
        coverage = json.loads(coverage_path.read_text())
        shutil.copy2(source, scratch / relative)
        shutil.copy2(coverage_path, (scratch / relative).with_suffix(".feature.coverage"))
        scenarios = re.split(r"^\s*Scenario(?: Outline)?: ", text, flags=re.M)[1:]
        assert [scenario.splitlines()[0] for scenario in scenarios] == [
            scenario["name"] for scenario in coverage["scenarios"]
        ]
        targets = set()
        mapped_tests = set()
        line_count = 0
        implementation_files = []
        for scenario_text, scenario in zip(scenarios, coverage["scenarios"]):
            steps = re.findall(r"^\s*((?:Given|When|Then|And|But) .+)$",
                               scenario_text, re.M)
            examples = re.findall(r"^\s*\|.+\|\s*$", scenario_text, re.M)
            expected_rows = len(examples) - 1 if examples else 1
            assert len(scenario["testMappings"]) == expected_rows
            example_rows = [example.strip().strip("|").strip() for example in examples[1:]]
            mapped_examples = set()
            for mapping in scenario["testMappings"]:
                targets.add(mapping["file"])
                start, end = map(int, mapping["lines"].split("-"))
                lines = (ROOT / mapping["file"]).read_text().splitlines()
                assert 1 <= start <= end <= len(lines)
                selected = [entry for entry in functions(mapping["file"])
                            if start <= entry["range"]["start"]["line"] + 1
                            and entry["range"]["end"]["line"] + 1 <= end]
                assert len(selected) == 1, mapping
                assert end == selected[0]["range"]["end"]["line"] + 1, mapping
                assert start == selected[0]["range"]["start"]["line"] - 1, mapping
                assert lines[start - 1].startswith("#[tokio::test"), mapping
                assert lines[start] == "#[serial_test::serial]", mapping
                if examples:
                    matches = [example for example in example_rows
                               if json.dumps(example) in selected[0]["text"]]
                    assert len(matches) == 1
                    assert matches[0] not in mapped_examples
                    mapped_examples.add(matches[0])
                name = re.search(r"\bfn (\w+)", selected[0]["text"])[1]
                assert name not in mapped_tests
                assert (mapping["file"], name) not in all_tests
                all_tests.add((mapping["file"], name))
                mapped_tests.add(name)
                local = "\n".join(lines[start - 1:end])
                attributes = "\n".join(lines[max(0, start - 4):end])
                assert re.search(r"#\[tokio::test(?:\(|\])", attributes)
                documented = re.findall(r"@step (.+)", selected[0]["text"])
                assert steps == documented, {"test": name, "steps": documented}
                line_count += end - start + 1
                assert mapping["implMappings"], mapping
                for implementation in mapping["implMappings"]:
                    path = implementation["file"]
                    source_lines = (ROOT / path).read_text().splitlines()
                    assert all(1 <= number <= len(source_lines)
                               for number in implementation["lines"])
                    assert all(any(entry["range"]["start"]["line"] + 1 <= number
                                   <= entry["range"]["end"]["line"] + 1
                                   for entry in functions(path))
                               for number in implementation["lines"]), implementation
                    line_count += len(implementation["lines"])
                    if path not in implementation_files:
                        implementation_files.append(path)
                run(["fspec", "link-coverage", feature, "--scenario", scenario["name"],
                     "--test-file", mapping["file"], "--test-lines", mapping["lines"]],
                    scratch)
            assert mapped_examples == set(example_rows)
        assert len(targets) == 1
        actual_tests = {re.search(r"\bfn (\w+)", entry["text"])[1]
                        for entry in functions(next(iter(targets)))}
        assert mapped_tests == actual_tests
        stats = coverage["stats"]
        assert stats["totalScenarios"] == stats["coveredScenarios"] == len(scenarios)
        assert stats["coveragePercent"] == 100
        assert stats["testFiles"] == sorted(targets)
        assert stats["implFiles"] == implementation_files
        assert stats["totalLinesCovered"] == line_count, line_count
        run(["fspec", "audit-coverage", feature], scratch)
        run(["fspec", "validate", relative], scratch)
        run(["fspec", "format", relative], scratch)
        assert (scratch / relative).read_text() == text
        print(json.dumps({"feature": feature, "scenarios": len(scenarios),
                          "concrete_tests": len(mapped_tests), "mapped_lines": line_count}))
        total_tests += len(mapped_tests)
        total_scenarios += len(scenarios)
        all_targets.update(targets)
    run(["fspec", "validate-tags"], scratch)
    assert len(FEATURES) == 3
    assert total_scenarios == 6
    assert total_tests == 21
    assert len(all_targets) == 3
    for relative, digest in SOURCE_HASHES.items():
        assert hashlib.sha256((ROOT / relative).read_bytes()).hexdigest() == digest, relative
    print(json.dumps({"verified_source_sha256": SOURCE_HASHES}, sort_keys=True))
    print(f"PASS: {len(FEATURES)} features, {total_scenarios} scenarios, "
          f"{total_tests} unique concrete tests, {len(all_targets)} targets; no bypasses")
