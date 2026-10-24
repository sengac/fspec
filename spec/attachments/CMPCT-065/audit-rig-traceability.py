import hashlib
import json
import pathlib
import re
import shutil
import subprocess
import tempfile


ROOT = pathlib.Path(__file__).resolve().parents[3]
FEATURES = [
    "anthropic-terminal-cache-usage",
    "builtin-provider-gemini-tool-pairing",
    "builtin-provider-openai-usage-evidence",
    "builtin-provider-output-limit-serialization",
    "builtin-provider-physical-serializer-parity-options",
    "builtin-provider-physical-serializer-parity-unicode",
    "builtin-provider-physical-serializer-parity",
    "builtin-provider-structured-stream-rejection",
    "builtin-provider-terminal-usage-evidence",
    "builtin-provider-responses-usage-aggregation",
]


def functions(source):
    lines = source.splitlines()
    found = []
    for index, line in enumerate(lines):
        match = re.match(r"( *)(?:async )?fn (\w+)", line)
        if not match:
            continue
        indent, name = match.groups()
        end = next(
            (row for row in range(index + 1, len(lines)) if lines[row] == indent + "}"),
            index,
        )
        start = index
        while start and lines[start - 1].lstrip().startswith(("#[", "///")):
            start -= 1
        body = "\n".join(lines[start : end + 1])
        if re.search(r"#\[(?:tokio::)?test\b", body):
            found.append((start + 1, end + 1, name, body))
    return found


def scenarios(text):
    found = {}
    current = None
    for line in text.splitlines():
        if line.strip().startswith("Scenario:"):
            current = line.strip().split(":", 1)[1].strip()
            found[current] = []
        elif current and re.match(r"\s+(Given|When|Then|And|But) ", line):
            found[current].append(line.strip())
    return found


def command(arguments, directory, issues):
    result = subprocess.run(
        ["fspec", *arguments], cwd=directory, capture_output=True, text=True
    )
    print(json.dumps({
        "command": ["fspec", *arguments],
        "directory": str(directory),
        "exit": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }))
    if result.returncode:
        issues.append("command failed: " + " ".join(arguments))


def inspect(feature, scratch, issues):
    relative = pathlib.Path("spec/features") / (feature + ".feature")
    path = ROOT / relative
    expected = scenarios(path.read_text())
    sidecar = path.with_suffix(".feature.coverage")
    data = json.loads(sidecar.read_text())
    entries = data["scenarios"]
    if sorted(expected) != sorted(entry["name"] for entry in entries):
        issues.append(feature + ": scenario identity mismatch")
    files = {relative, relative.with_suffix(".feature.coverage")}
    tests = set()
    implementations = set()
    total = 0
    for entry in entries:
        mappings = entry["testMappings"]
        if len(mappings) != 1:
            issues.append(feature + ": expected one actual test per scenario")
        for mapping in mappings:
            tests.add(mapping["file"])
            files.add(pathlib.Path(mapping["file"]))
            source = (ROOT / mapping["file"]).read_text()
            first, last = map(int, mapping["lines"].split("-"))
            exact = [item for item in functions(source) if item[:2] == (first, last)]
            if len(exact) != 1:
                issues.append(feature + ": not an exact complete test: " + mapping["lines"])
            else:
                local_steps = re.findall(
                    r"^\s*///?\s*@step (.+)$", exact[0][3], re.MULTILINE
                )
                for step in expected.get(entry["name"], []):
                    if local_steps.count(step) != 1:
                        issues.append(feature + ": missing literal test-local step: " + step)
            if not 1 <= first <= last <= len(source.splitlines()):
                issues.append(feature + ": test bounds invalid")
            total += last - first + 1
            if not mapping["implMappings"]:
                issues.append(feature + ": missing implementation")
            for implementation in mapping["implMappings"]:
                implementations.add(implementation["file"])
                files.add(pathlib.Path(implementation["file"]))
                length = len((ROOT / implementation["file"]).read_text().splitlines())
                numbers = implementation["lines"]
                if not numbers or any(not 1 <= number <= length for number in numbers):
                    issues.append(feature + ": implementation bounds invalid")
                total += len(numbers)
    stats = data["stats"]
    if (
        stats["totalScenarios"] != len(expected)
        or stats["coveredScenarios"] != len(expected)
        or stats["coveragePercent"] != 100
        or set(stats["testFiles"]) != tests
        or set(stats["implFiles"]) != implementations
        or stats["totalLinesCovered"] != total
    ):
        issues.append(feature + ": metadata mismatch")
    for name in sorted(files):
        target = scratch / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / name, target)
        print(json.dumps({
            "source": str(name),
            "sha256": hashlib.sha256((ROOT / name).read_bytes()).hexdigest(),
        }))
    for entry in entries:
        for mapping in entry["testMappings"]:
            command([
                "link-coverage", feature, "--scenario", entry["name"],
                "--test-file", mapping["file"], "--test-lines", mapping["lines"],
            ], scratch, issues)
            for implementation in mapping["implMappings"]:
                command([
                    "link-coverage", feature, "--scenario", entry["name"],
                    "--test-file", mapping["file"],
                    "--impl-file", implementation["file"],
                    "--impl-lines", ",".join(map(str, implementation["lines"])),
                ], scratch, issues)
    for arguments in [
        ["validate", str(relative)],
        ["validate-tags", str(relative)],
        ["audit-coverage", feature],
    ]:
        command(arguments, ROOT, issues)
    print(json.dumps({"feature": feature, "scenarios": len(expected), "mappedLines": total}))
    return len(expected)


def main():
    issues = []
    count = 0
    with tempfile.TemporaryDirectory(prefix="cmpct-rig-steps-") as directory:
        scratch = pathlib.Path(directory)
        for feature in FEATURES:
            count += inspect(feature, scratch, issues)
    if len(FEATURES) != 10 or count != 39:
        issues.append("expected exactly ten features and 39 scenarios")
    print(json.dumps({"features": len(FEATURES), "scenarios": count, "issues": issues}, indent=2))
    return bool(issues)


if __name__ == "__main__":
    raise SystemExit(main())
