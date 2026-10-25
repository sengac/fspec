"""Bounded offline final-source validation using isolated process inputs."""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
RUST = ROOT / "rust"
HERE = Path(__file__).resolve().parent
BASE = ["--offline", "--profile", "ci-test", "-j", "2"]
CLI = [
    "cont011_native_completion_composition", "auto_continue_engine_test",
    "cont006_goal_immediate_termination", "goal_enforcement_test",
    "bug223_gemini_helper_settlement", "cmpct042_gemini_continuation_token_state_test",
    "bug262_ordered_chat_tools", "bug263_chat_terminal", "bug264_responses_terminal",
    "bug266_custom_done_settlement", "bug268_copilot_outbound_dialect",
    "prov162_protocol_outer_loop",
]

PROVIDERS = [
    "cmpct066_anthropic_terminal_usage", "claude_refreshing_client_test",
    "claude_oauth_routing_test", "cmpct071_builtin_gemini_tool_pairing",
    "bug260_refusal_history", "bug261_forced_refresh_recovery",
    "bug266_custom_stream_settlement", "bug268_copilot_issuer_isolation",
    "bug268_copilot_lineage_boundaries", "bug268_copilot_nonstream_terminal",
    "bug268_copilot_origin_boundary", "bug268_copilot_request_refresh",
    "bug269_scripted_request_refresh", "prov162_typed_protocol_continuation",
]
SHIPPING = ["codelet-providers", "codelet-cli", "codelet-agent-loop",
            "codelet-sessions", "codelet-napi"]


def packages(names):
    return [argument for name in names for argument in ("-p", name)]


def targets(names):
    return [argument for name in names for argument in ("--test", name)]


commands = [
    ("final-cli-matrix.log", ["cargo", "test", *BASE, "-p", "codelet-cli",
        "--features", "test-support", "--no-fail-fast", *targets(CLI)]),
    ("final-provider-matrix.log", ["cargo", "test", *BASE, "-p", "codelet-providers",
        "--features", "test-support", "--no-fail-fast", *targets(PROVIDERS)]),
    ("final-shared-matrix.log", ["cargo", "test", *BASE, "-p", "codelet-agent-loop",
        "--features", "test-support", "--no-fail-fast", *targets([
            "cmpct041_pre_compaction_basis_test", "bug265_recovery_completion_accounting",
            "bug267_scripted_dispatch_identity", "bug268_live_session_opaque"])]),
    ("final-shipping-check.log", ["cargo", "check", *BASE, *packages(SHIPPING), "--lib"]),
    ("final-shipping-clippy.log", ["cargo", "clippy", *BASE, *packages(SHIPPING), "--lib"]),
    ("final-focused-test-clippy.log", ["cargo", "clippy", *BASE, "-p", "codelet-cli",
        "--features", "test-support", *targets([
            "cont011_native_completion_composition", "bug223_gemini_helper_settlement"])]),
]

supplemental = sys.argv[1:] == ["supplemental"]
safety_controls = sys.argv[1:] == ["safety-controls"]
post_format = sys.argv[1:] == ["post-format"]
native_gate = sys.argv[1:] == ["native-gate"]
if supplemental:
    commands = [
        ("final-cli-credential-fixture-controls.log", ["cargo", "test", *BASE,
            "-p", "codelet-cli", "--no-fail-fast", *targets([
                "auto_continue_engine_test", "cont006_goal_immediate_termination",
                "goal_enforcement_test"])]),
        ("final-core-batch-control.log", ["cargo", "test", *BASE,
            "-p", "codelet-core", "--test", "bug259_recovery_contention"]),
        ("final-standalone-check.log", ["cargo", "check", *BASE,
            "-p", "codelet-fspec", "--bin", "fspec"]),
        ("final-adapter-lib-clippy.log", ["cargo", "clippy", *BASE,
            "--manifest-path", "/tmp/cont011-rig-unit/Cargo.toml", "--target-dir",
            str(RUST / "target"), "-p", "rig-core", "--lib"]),
    ]

if post_format:
    cli_targets = ["cont011_native_completion_composition", "auto_continue_engine_test",
                   "cont006_goal_immediate_termination", "bug223_gemini_helper_settlement"]
    shared_targets = ["cmpct041_pre_compaction_basis_test", "cont011_persisted_completion"]
    commands = [
        ("final-post-format-cli.log", ["cargo", "test", *BASE, "-p", "codelet-cli",
            "--features", "test-support", "--no-fail-fast", *targets(cli_targets)]),
        ("final-post-format-shared.log", ["cargo", "test", *BASE, "-p", "codelet-agent-loop",
            "--features", "test-support", "--no-fail-fast", *targets(shared_targets)]),
        ("final-post-format-cli-clippy.log", ["cargo", "clippy", *BASE, "-p", "codelet-cli",
            "--features", "test-support", *targets(cli_targets)]),
        ("final-post-format-shared-clippy.log", ["cargo", "clippy", *BASE, "-p", "codelet-agent-loop",
            "--features", "test-support", *targets(shared_targets)]),
        ("final-post-format-shipping-check.log", ["cargo", "check", *BASE, *packages(SHIPPING), "--lib"]),
    ]

if safety_controls:
    commands = [
        ("final-truncation-recovery-controls.log", ["cargo", "test", *BASE,
            "-p", "codelet-cli", "--test", "truncation_recovery_test"]),
        ("final-stop-reason-controls.log", ["cargo", "test", *BASE,
            "-p", "codelet-providers", "--test", "stop_reason_propagation_test"]),
    ]

if native_gate:
    commands = [
        ("native-gate-codex-history.log", ["cargo", "test", *BASE,
            "-p", "codelet-providers", "--test", "prov149_codex_history"]),
        ("native-gate-codex-reload.log", ["cargo", "test", *BASE,
            "-p", "codelet-agent-loop", "--test", "prov149_codex_reload"]),
    ]

results = []
with tempfile.TemporaryDirectory(prefix="cont011-final-inputs-") as home:
    environment = os.environ.copy()
    for key in list(environment):
        if key.upper().endswith("_PROXY") or key.startswith((
            "OPENAI_", "ANTHROPIC_", "CLAUDE_", "GOOGLE_", "GEMINI_", "ZAI_",
            "COPILOT_", "GITHUB_TOKEN", "GH_TOKEN", "CODEX_",
        )):
            environment.pop(key)
    environment.update({
        "HOME": home, "FSPEC_HOME": str(Path(home) / ".fspec/credentials"),
        "FSPEC_USER_DIR": str(Path(home) / ".fspec"),
        "CODEX_HOME": str(Path(home) / "codex"),
        "CARGO_HOME": "/Users/rquast/.cargo", "RUSTUP_HOME": "/Users/rquast/.rustup",
        "CARGO_PROFILE_CI_TEST_DEBUG": "0", "NO_PROXY": "*", "no_proxy": "*",
        "ANTHROPIC_API_KEY": "synthetic-loopback-only",
        "OPENAI_API_KEY": "synthetic-loopback-only",
    })
    for filename, command in commands:
        available = os.statvfs(RUST).f_bavail * os.statvfs(RUST).f_frsize
        if available < 6 * 1024**3:
            raise RuntimeError("Six GiB reserve reached; supervisor cleanup required")
        print(filename, json.dumps(command), flush=True)
        with (HERE / filename).open("w") as output:
            output.write("COMMAND: " + json.dumps(command) + "\n")
            output.flush()
            result = subprocess.run(command, cwd=RUST, env=environment,
                                    stdout=output, stderr=subprocess.STDOUT, check=False)
        results.append({"log": filename, "exit_code": result.returncode, "argv": command})
        receipt = ("native-gate-command-results.json" if native_gate else
                   "final-post-format-command-results.json" if post_format else
                   "final-safety-command-results.json" if safety_controls else
                   "final-supplemental-command-results.json" if supplemental else
                   "final-command-results.json")
        (HERE / receipt).write_text(json.dumps(results, indent=2) + "\n")
        print(filename, "exit", result.returncode, flush=True)
raise SystemExit(1 if any(row["exit_code"] for row in results) else 0)
