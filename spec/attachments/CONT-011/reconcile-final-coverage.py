"""Rebase owned test ranges through native unlink/link, preserving impl witnesses."""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
BINARY = "/Users/rquast/.local/bin/fspec"
NATIVE = "rust/cli/tests/cont011_native_completion_composition.rs"
ANTHROPIC = "rust/patches/rig-core/src/providers/anthropic/streaming/transport.rs"
GEMINI = "rust/patches/rig-core/src/providers/gemini/streaming.rs"
EARLY = "rust/cli/src/interactive/done_early_exit.rs"
FEATURES = {
    "completion-registration-layout-guards": [
        ("Armed Done registration stays guarded across provider builders",
         "rust/cli/tests/auto_continue_engine_test.rs",
         "done_is_registered_only_while_auto_continue_is_armed",
         [("rust/providers/src/codex/agent.rs", "40-108")])],
    "completion-goal-teardown-layout-guards": [
        ("Goal teardown guards preserve a single reminder owner",
         "rust/cli/tests/cont006_goal_immediate_termination.rs",
         "stream_loop_wiring_pins_goal_mode_early_exit_and_single_teardown_invariants",
         [("rust/cli/src/session/reminders.rs", "1-28"), (EARLY, "65-120")])],
    "completion-snapshot-delegation-guards": [
        ("Shared snapshot delegation retains semantic and writer guards",
         "rust/agent-loop/tests/cmpct041_pre_compaction_basis_test.rs",
         "auto_and_manual_snapshots_agree_on_the_same_basis",
         [("rust/sessions/src/background_session/metrics.rs", "19-27"),
          ("rust/agent-loop/src/background_output.rs", "644-664")])],
    "native-completion-loopback-composition": [],
}


def entire(path):
    return path, "1-" + str(len((ROOT / path).read_text().splitlines()))


for title, function, implementation in [
    ("Native clean stops compose nudge work and accepted Done",
     "native_clean_stops_compose_nudge_work_and_accepted_done",
     [entire(ANTHROPIC), entire(GEMINI), (EARLY, "65-120")]),
    ("Native rejected goal completion remains a rejection",
     "native_rejected_goal_completion_remains_a_rejection", [(EARLY, "65-120")]),
    ("Native malformed delivery and cancellation do not complete work",
     "native_malformed_delivery_and_cancellation_do_not_complete_work",
     [entire(ANTHROPIC), entire(GEMINI)]),
    ("Anthropic client calls require authoritative terminal settlement",
     "anthropic_client_calls_require_authoritative_terminal_settlement", [entire(ANTHROPIC)]),
    ("Gemini client calls require safe authoritative terminal settlement",
     "gemini_client_calls_require_safe_authoritative_terminal_settlement", [entire(GEMINI)]),
    ("Isolated security still blocks high risk native file work",
     "isolated_security_still_blocks_high_risk_native_file_work",
     [("rust/tools/src/rlcd/security.rs", "104-211")]),
]:
    FEATURES["native-completion-loopback-composition"].append(
        (title, NATIVE, function, implementation))

for feature in ["gemini-helper-completion-settlement", "gemini-helper-accepted-completion"]:
    data = json.loads((ROOT / f"spec/features/{feature}.feature.coverage").read_text())
    FEATURES[feature] = []
    for scenario in data["scenarios"]:
        mappings = scenario["testMappings"]
        assert mappings and len({mapping["file"] for mapping in mappings}) == 1
        path = mappings[0]["file"]
        witnesses = {(entry["file"], ",".join(map(str, entry["lines"])))
                     for mapping in mappings for entry in mapping["implMappings"]}
        function = scenario["name"].lower().replace(" ", "_")
        FEATURES[feature].append((scenario["name"], path, function, sorted(witnesses)))


def native(*args):
    print("COMMAND:", json.dumps(list(args)), flush=True)
    subprocess.run([BINARY, *args], cwd=ROOT, check=True)


def test_range(path, function):
    lines = (ROOT / path).read_text().splitlines()
    start = next(index for index, line in enumerate(lines)
                 if re.match(r"(?:async )?fn " + re.escape(function) + r"\b", line))
    stop = next((index for index in range(start + 1, len(lines))
                 if re.match(r"#\[(?:tokio::)?test", lines[index])), len(lines))
    while stop > start and not lines[stop - 1].strip():
        stop -= 1
    return f"{start + 1}-{stop}"


before = {feature: json.loads((ROOT / f"spec/features/{feature}.feature.coverage").read_text())
          for feature in FEATURES}
snapshot = "post-format-coverage-before-native-rebase.json" if sys.argv[1:] == ["post-format"] else "final-coverage-before-native-rebase.json"
destination = HERE / snapshot
if destination.exists():
    raise RuntimeError("Preserve the existing before-rebase evidence; do not overwrite it")
destination.write_text(json.dumps(before, indent=2) + "\n")
for feature, entries in FEATURES.items():
    for scenario, path, function, witnesses in entries:
        old = next(item for item in before[feature]["scenarios"] if item["name"] == scenario)
        assert all(mapping["file"] == path for mapping in old["testMappings"])
        for mapping in old["testMappings"]:
            native("unlink-coverage", feature, "--scenario", scenario, "--test-file", path)
        native("link-coverage", feature, "--scenario", scenario, "--test-file", path,
               "--test-lines", test_range(path, function))
        for implementation, span in witnesses:
            native("link-coverage", feature, "--scenario", scenario, "--test-file", path,
                   "--impl-file", implementation, "--impl-lines", span)
    native("audit-coverage", feature)
