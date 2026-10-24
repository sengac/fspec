import collections
import hashlib
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile


ROOT = pathlib.Path(__file__).resolve().parents[3]
FEATURES = [
    "versioned-dag-generation-commit", "dag-publication-ownership",
    "durable-checkpoint-envelope-preservation",
    "durable-session-manifest-transactions", "session-manifest-lost-acknowledgement",
    "session-manifest-delete-ownership", "whole-dag-candidate-validation",
    "dag-cold-boundary-validation", "dag-promotion-revalidation",
    "durable-dag-preservation", "durable-versioned-dag-boundary",
    "durable-structured-tool-identity",
    "durable-reasoning-settlement", "core-admission-hook-cancellation",
]
CACHE = {}
HASHES = {}


def source(relative):
    path = ROOT / relative
    text = path.read_text()
    HASHES.setdefault(relative, hashlib.sha256(path.read_bytes()).hexdigest())
    return text


def functions(relative):
    if relative not in CACHE:
        source(relative)
        result = subprocess.run(
            ["ast-grep", "scan", "--inline-rules",
             "id: functions\nlanguage: Rust\nrule:\n  kind: function_item",
             "--json=compact", relative],
            cwd=ROOT, capture_output=True, text=True, check=True,
        )
        CACHE[relative] = [dict(
            start=entry["range"]["start"]["line"] + 1,
            end=entry["range"]["end"]["line"] + 1,
            name=re.search(r"\bfn\s+(\w+)", entry["text"])[1],
            text=entry["text"],
        ) for entry in json.loads(result.stdout)]
    return CACHE[relative]


def run(command, directory):
    result = subprocess.run(command, cwd=directory, capture_output=True, text=True)
    record = dict(command=command, exit=result.returncode,
                  output=result.stdout + result.stderr)
    assert result.returncode == 0, record
    return record


def audit_feature(name, temporary, unique_tests):
    relative = f"spec/features/{name}.feature"
    text = source(relative)
    coverage_relative = relative + ".coverage"
    coverage = json.loads(source(coverage_relative))
    for path in [relative, coverage_relative]:
        shutil.copy2(ROOT / path, temporary / path)
    sections = re.split(r"^\s*Scenario: ", text, flags=re.M)[1:]
    assert [section.splitlines()[0] for section in sections] == [
        entry["name"] for entry in coverage["scenarios"]
    ], name
    targets = set()
    mapped = set()
    implementation_files = []
    line_count = 0
    rows = []
    commands = []
    for section, scenario in zip(sections, coverage["scenarios"]):
        assert len(scenario["testMappings"]) == 1, scenario["name"]
        mapping = scenario["testMappings"][0]
        target = mapping["file"]
        targets.add(target)
        start, end = map(int, mapping["lines"].split("-"))
        lines = source(target).splitlines()
        assert 1 <= start <= end <= len(lines), mapping
        selected = [entry for entry in functions(target)
                    if start <= entry["start"] <= entry["end"] <= end]
        assert len(selected) == 1, mapping
        function = selected[0]
        assert end == function["end"], mapping
        assert all(not line.strip() or line.strip().startswith("#[")
                   for line in lines[start - 1:function["start"] - 1]), mapping
        attributes = "\n".join(lines[max(0, function["start"] - 5):function["start"]])
        assert re.search(r"#\[(?:tokio::)?test(?:\(|\])", attributes), mapping
        identity = (target, function["name"])
        assert identity not in unique_tests, identity
        unique_tests.add(identity)
        mapped.add(function["name"])
        expected_steps = re.findall(r"^\s*((?:Given|When|Then|And|But) .+)$",
                                    section, re.M)
        actual_steps = re.findall(r"@step (.+)", function["text"])
        assert collections.Counter(expected_steps) == collections.Counter(actual_steps), {
            "scenario": scenario["name"], "expected": expected_steps, "actual": actual_steps,
        }
        assert re.search(r"\bassert(?:_eq|_ne)?!", function["text"]), identity
        assert mapping["implMappings"], scenario["name"]
        line_count += end - start + 1
        for implementation in mapping["implMappings"]:
            path = implementation["file"]
            numbers = implementation["lines"]
            production = source(path).splitlines()
            assert path.startswith("rust/core/src/"), implementation
            assert numbers and len(numbers) == len(set(numbers)), implementation
            assert all(1 <= number <= len(production) for number in numbers), implementation
            assert any(production[number - 1].strip() for number in numbers), implementation
            line_count += len(numbers)
            if path not in implementation_files:
                implementation_files.append(path)
        rows.append(dict(scenario=scenario["name"], test=target,
                         function=function["name"], lines=mapping["lines"],
                         implementations=mapping["implMappings"]))
        commands.append(run([
            "fspec", "link-coverage", name, "--scenario", scenario["name"],
            "--test-file", target, "--test-lines", mapping["lines"],
        ], temporary))
    assert len(targets) == 1, targets
    target = next(iter(targets))
    actual_tests = set()
    lines = source(target).splitlines()
    for function in functions(target):
        attributes = "\n".join(lines[max(0, function["start"] - 5):function["start"]])
        if re.search(r"#\[(?:tokio::)?test(?:\(|\])", attributes):
            actual_tests.add(function["name"])
    assert mapped == actual_tests, {"mapped": sorted(mapped), "actual": sorted(actual_tests)}
    stats = coverage["stats"]
    assert stats["totalScenarios"] == stats["coveredScenarios"] == len(sections), stats
    assert stats["coveragePercent"] == 100, stats
    assert stats["testFiles"] == sorted(targets), stats
    assert stats["implFiles"] == implementation_files, stats
    assert stats["totalLinesCovered"] == line_count, stats
    for command in [
        ["fspec", "validate", relative], ["fspec", "format", relative],
        ["fspec", "audit-coverage", name],
    ]:
        commands.append(run(command, temporary))
    assert (temporary / relative).read_text() == text, name
    return dict(feature=name, scenarios=len(sections), target=target,
                mappings=rows, commands=commands, declared_lines=line_count)


def main():
    output = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(
        "/tmp/cmpct-core-supervised-034833d1-traceability.json"
    )
    unique_tests = set()
    with tempfile.TemporaryDirectory(prefix="cmpct-core-completion-") as directory:
        temporary = pathlib.Path(directory)
        (temporary / "spec/features").mkdir(parents=True)
        (temporary / "rust").symlink_to(ROOT / "rust", target_is_directory=True)
        for name in ["work-units.json", "tags.json", "prefixes.json"]:
            path = ROOT / "spec" / name
            if path.exists():
                shutil.copy2(path, temporary / "spec" / name)
        rows = [audit_feature(name, temporary, unique_tests) for name in FEATURES]
        tags = run(["fspec", "validate-tags"], temporary)
    changed = [relative for relative, digest in HASHES.items()
               if hashlib.sha256((ROOT / relative).read_bytes()).hexdigest() != digest]
    assert not changed, changed
    summary = dict(features=len(rows), scenarios=sum(row["scenarios"] for row in rows),
                   unique_tests=len(unique_tests), tags_exit=tags["exit"],
                   changed_during_audit=changed)
    output.write_text(json.dumps(dict(summary=summary, features=rows, tags=tags,
                                      source_sha256=HASHES), indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    print(f"PASS: complete scenario/test/implementation traceability; evidence: {output}")


if __name__ == "__main__":
    main()
