import json
import pathlib
import re
import shutil
import subprocess
import tempfile


ROOT = pathlib.Path(__file__).resolve().parents[3]
FEATURES = [
    "custom-provider-usage-normalization", "custom-provider-runtime-usage",
    "physical-request-settlement", "runtime-request-settlement",
    "serialized-request-admission", "selected-provider-request-preparation",
    "provider-physical-admission", "provider-hook-cancellation",
    "codex-latest-context-occupancy", "codex-physical-request-admission",
    "codex-committed-lineage", "codex-committed-checkpoint-precedence",
    "codex-checkpoint-replay-integrity", "codex-invalid-checkpoint-state",
    "codex-explicit-transport", "custom-provider-physical-serializer-parity",
    "stub-provider-physical-serializer-parity",
    "codex-idempotent-boundary-installation", "selected-request-media-semantics",
    "codex-physical-request-freshness", "provider-checkpoint-restore-preflight",
    "codex-typed-continuation", "codex-interrupted-continuation", "codex-refreshed-reminders",
]
NEW = {
    "codex-typed-continuation": (
        "prov149_codex_history", {
            "rust/providers/src/codex/history.rs": ["snapshot", "commit_for_generation", "restore", "prepare"],
            "rust/providers/src/codex/history_lineage.rs": ["assemble"],
            "rust/providers/src/codex/history_replay.rs": ["validate"],
        },
    ),
    "codex-interrupted-continuation": (
        "prov149_codex_interruption", {
            "rust/providers/src/codex/history.rs": ["snapshot", "prepare", "transition_generation"],
            "rust/providers/src/codex/history_lineage.rs": ["assemble"],
            "rust/providers/src/codex/history_replay.rs": ["validate"],
        },
    ),
    "codex-refreshed-reminders": (
        "prov149_codex_reminders", {
            "rust/providers/src/codex/history.rs": ["snapshot", "prepare", "transition_generation"],
            "rust/providers/src/codex/history_lineage.rs": ["assemble"],
            "rust/providers/src/codex/history_replay.rs": ["validate"],
        },
    ),
    "codex-invalid-checkpoint-state": (
        "cmpct080_invalid_checkpoint", {
            "rust/providers/src/codex/history.rs": ["snapshot", "restore", "prepare", "validate_checkpoint", "promote_checked"],
            "rust/providers/src/codex/session.rs": ["restore", "set_writer"],
        },
    ),
    "selected-request-media-semantics": (
        "cmpct071_media_semantics", {
            "rust/providers/src/selected_request.rs": ["media_occupancy", "inspect_media_occupancy"],
        },
    ),
    "codex-physical-request-freshness": (
        "cmpct078_physical_freshness", {
            "rust/providers/src/codex/streaming.rs": ["stream"],
            "rust/providers/src/codex/history.rs": ["ensure_generation", "commit_for_generation", "promote_checked"],
        },
    ),
    "provider-checkpoint-restore-preflight": (
        "cmpct080_restore_preflight", {
            "rust/providers/src/codex/session.rs": ["validate_restore", "validate_committed_restore"],
            "rust/providers/src/codex/checkpoint_restore.rs": ["select", "validate"],
        },
    ),
    "codex-idempotent-boundary-installation": (
        "cmpct078_idempotent_boundary", {
            "rust/providers/src/codex/history_generation.rs": ["install_committed_generation"],
            "rust/providers/src/codex/checkpoint_restore.rs": ["select", "validate"],
        },
    ),
}
CACHE = {}


def run(*args, directory=ROOT):
    result = subprocess.run(args, cwd=directory, text=True, capture_output=True)
    print(json.dumps({"command": args, "exit": result.returncode,
                      "output": result.stdout + result.stderr}))
    assert result.returncode == 0, args
    return result.stdout


def functions(path):
    key = str(path)
    if key not in CACHE:
        entries = json.loads(run(
            "ast-grep", "scan", "--inline-rules",
            "id: functions\nlanguage: Rust\nrule:\n  kind: function_item",
            "--json=compact", str(path),
        ))
        CACHE[key] = {
            re.search(r"\bfn (\w+)", entry["text"])[1]: (
                entry["range"]["start"]["line"] + 1,
                entry["range"]["end"]["line"] + 1,
            ) for entry in entries
        }
        source = path.read_text()
        for matched in re.finditer(
            r"(?m)^    fn (\w+)\([^\n]*\) \{\n(?:(?!    \}).*\n)*    \}", source
        ):
            CACHE[key].setdefault(matched[1], (
                source[:matched.start()].count("\n") + 1,
                source[:matched.end()].count("\n") + 1,
            ))
    return CACHE[key]


def expand(lines):
    if isinstance(lines, list):
        return set(lines)
    result = set()
    for part in lines.split(","):
        bounds = list(map(int, part.split("-")))
        result.update(range(bounds[0], bounds[-1] + 1))
    return result


def assertion_reachable(path, name, visited=None):
    visited = set() if visited is None else visited
    if name in visited:
        return False
    visited.add(name)
    start, end = functions(path)[name]
    body = "\n".join(path.read_text().splitlines()[start - 1:end])
    if re.search(r"\b(?:prop_)?assert(?:_eq|_ne)?!", body):
        return True
    return any(assertion_reachable(path, called, visited)
               for called in functions(path)
               if re.search(r"\b" + re.escape(called) + r"\(", body))


def implementation_lines(path, names):
    result = set()
    for name in names:
        start, end = functions(ROOT / path)[name]
        result.update(range(start, end + 1))
    return result


total = 0
with tempfile.TemporaryDirectory(prefix="cmpct-provider-audit-") as temporary:
    scratch = pathlib.Path(temporary)
    (scratch / "spec/features").mkdir(parents=True)
    for name in ["tags.json", "work-units.json", "prefixes.json"]:
        source = ROOT / "spec" / name
        if source.exists():
            shutil.copy2(source, scratch / "spec" / name)
    for feature in FEATURES:
        feature_path = ROOT / "spec/features" / f"{feature}.feature"
        shutil.copy2(feature_path, scratch / "spec/features" / feature_path.name)
        coverage_path = feature_path.with_suffix(".feature.coverage")
        if coverage_path.exists():
            shutil.copy2(coverage_path, scratch / "spec/features" / coverage_path.name)
    run("fspec", "generate-coverage", directory=scratch)
    for feature in FEATURES:
        coverage_name = f"{feature}.feature.coverage"
        shutil.copy2(scratch / "spec/features" / coverage_name,
                     ROOT / "spec/features" / coverage_name)
    for feature in FEATURES:
        feature_path = ROOT / "spec/features" / f"{feature}.feature"
        coverage_path = feature_path.with_suffix(".feature.coverage")
        coverage = json.loads(coverage_path.read_text()) if coverage_path.exists() else {"scenarios": []}
        scenarios = re.findall(r"^\s*Scenario(?: Outline)?: (.+)$", feature_path.read_text(), re.M)
        existing = {scenario["name"]: scenario for scenario in coverage["scenarios"]}
        for name in scenarios:
            normalized = re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")
            previous = existing.get(name, {}).get("testMappings", [])
            if feature in NEW:
                target, implementation_names = NEW[feature]
                test_file = f"rust/providers/tests/{target}.rs"
                implementations = {path: implementation_lines(path, names)
                                   for path, names in implementation_names.items()}
            else:
                assert len(previous) == 1, (feature, name, previous)
                test_file = previous[0]["file"]
                implementations = {}
                for mapping in previous[0].get("implMappings", []):
                    path = mapping["file"]
                    lines = expand(mapping["lines"])
                    original = subprocess.run(["git", "show", f":{path}"], cwd=ROOT,
                                              capture_output=True, text=True)
                    if original.returncode == 0 and original.stdout != (ROOT / path).read_text():
                        saved = scratch / (path.replace("/", "_") + ".rs")
                        saved.write_text(original.stdout)
                        names = [name for name, (start, end) in functions(saved).items()
                                 if lines.intersection(range(start, end + 1))]
                        if names:
                            lines = implementation_lines(path, names)
                    implementations[path] = lines
            if name == "Isolated inaccessible checkpoint locks fail without substituting fresh history":
                test_file = "rust/providers/src/codex/history_state_tests.rs"
            if name == "Nonstreaming responses cannot outlive their checkpoint authority":
                test_file = "rust/providers/tests/cmpct078_completion_freshness.rs"
                implementations = {
                    "rust/providers/src/codex/model.rs": implementation_lines(
                        "rust/providers/src/codex/model.rs", ["completion"]),
                    "rust/providers/src/codex/history.rs": implementation_lines(
                        "rust/providers/src/codex/history.rs", ["ensure_generation", "commit_for_generation", "promote_checked"]),
                }
            start, end = functions(ROOT / test_file)[normalized]
            body = "\n".join((ROOT / test_file).read_text().splitlines()[start - 1:end])
            selected = next(block for block in re.split(
                r"^\s*Scenario(?: Outline)?: ", feature_path.read_text(), flags=re.M)[1:]
                if block.splitlines()[0] == name)
            expected_steps = re.findall(r"^\s*((?:Given|When|Then|And|But) .+)$", selected, re.M)
            actual_steps = re.findall(r"@step (.+)", body)
            assert expected_steps == actual_steps, (feature, name, expected_steps, actual_steps)
            assert assertion_reachable(ROOT / test_file, normalized), (feature, name, "missing assertion")
            if previous:
                run("fspec", "unlink-coverage", feature, "--scenario", name, "--all")
            run("fspec", "link-coverage", feature, "--scenario", name,
                "--test-file", test_file, "--test-lines", f"{start}-{end}")
            for path, lines in implementations.items():
                assert lines and min(lines) > 0 and max(lines) <= len((ROOT / path).read_text().splitlines())
                run("fspec", "link-coverage", feature, "--scenario", name,
                    "--test-file", test_file, "--impl-file", path,
                    "--impl-lines", ",".join(map(str, sorted(lines))))
            total += 1
        run("fspec", "validate", str(feature_path))
        run("fspec", "audit-coverage", feature)
        shutil.copy2(feature_path, scratch / "spec/features" / feature_path.name)
        updated = json.loads(coverage_path.read_text())
        assert updated["stats"]["coveragePercent"] == 100
        assert {scenario["name"] for scenario in updated["scenarios"]} == set(scenarios)
        assert all(len(scenario["testMappings"]) == 1 for scenario in updated["scenarios"])
    run("fspec", "validate-tags", directory=scratch)
print(json.dumps({"features": len(FEATURES), "scenarios": total, "coveragePercent": 100}))
