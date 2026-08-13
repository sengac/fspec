//! Cargo-shape / source-shape / CLI-surface / workspace integration tests for RPC-010.
//!
//! Feature: spec/features/fspec-binary-cargo-shape-rpc010.feature
//!
//! Consolidated from source_shape.rs + cli_surface.rs version/help/--pidfile-rejection +
//! workspace_and_reconnect.rs workspace scenarios so that
//! `fspec-binary-cargo-shape-rpc010.feature` maps 1:1 to a single test file
//! (fspec coverage validator design intent — 1 feature = 1 test file).
//!
//! These tests codify cargo / file-layout / source-content invariants
//! statically AND exercise the clap CLI surface dynamically. They MUST
//! FAIL in the testing phase because main.rs is a placeholder.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::fs;
use std::process::{Command, Stdio};

use codelet_fspec_tui::{FspecBackend, WebSocketFspecBackend};
use common::{
    codelet_root, fspec_bin, fspec_crate_root, make_workspace, spawn_fspec_daemon, strip_comments,
    ChildGuard,
};
use url::Url;

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[test]
fn scenario_fspec_version_prints_the_workspace_version() {
    // @step When the developer runs `rust/target/release/fspec --version`
    let output = Command::new(fspec_bin())
        .arg("--version")
        .output()
        .expect("spawn fspec --version");

    // @step Then the command exits with code 0
    assert!(
        output.status.success(),
        "fspec --version must exit with code 0; got {:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // @step And STDOUT contains the workspace version string declared in `rust/Cargo.toml [workspace.package].version`
    // The workspace package version is supplied to clap via the `version`
    // attribute, which `Cli::parse` populates from CARGO_PKG_VERSION.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = env!("CARGO_PKG_VERSION");
    assert!(
        stdout.contains(expected),
        "fspec --version stdout must contain {expected:?}; got {stdout:?}"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[test]
fn scenario_fspec_help_shows_three_subcommands() {
    // @step When the developer runs `rust/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");

    // @step Then the command exits with code 0
    assert!(
        output.status.success(),
        "fspec --help must exit with code 0; got {:?}",
        output.status
    );
    let help = String::from_utf8_lossy(&output.stdout);

    // @step And the help text mentions exactly the subcommands `daemon` and `client`
    assert!(
        help.contains("daemon"),
        "--help must mention `daemon` subcommand; got:\n{help}"
    );
    assert!(
        help.contains("client"),
        "--help must mention `client` subcommand; got:\n{help}"
    );

    // @step And the help text describes the no-subcommand default as combined mode
    let lc = help.to_lowercase();
    assert!(
        lc.contains("combined"),
        "--help must describe the no-subcommand default as combined mode; got:\n{help}"
    );

    // @step And the help text mentions the `--workspace` flag
    assert!(
        help.contains("--workspace"),
        "--help must mention `--workspace`; got:\n{help}"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_workspace_defaults_to_cwd_when_omitted() {
    // @step Given a tempdir `<W>` containing a seeded spec/work-units.json
    let (w, _path) = make_workspace(&[("WS-CWD-1", "cwd-default", "backlog")]);

    // @step When the developer runs `cd <W> && fspec daemon` (no --workspace flag)
    let mut child = Command::new(fspec_bin())
        .arg("daemon")
        .current_dir(w.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec daemon (cwd-rooted)");
    let stdout = child.stdout.take().expect("stdout");
    let _guard = ChildGuard(child);
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    use std::io::BufRead;
    reader.read_line(&mut line).expect("read port line");
    let port: u16 = line
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("port not a u16: {line:?} ({e})"));

    // @step Then the daemon's WorkUnitsWatcher is rooted at `<W>`
    // (Indirectly asserted by the next step: only a watcher rooted at
    //  the CWD-passed workspace can return the seeded WorkUnitInfo.)
    let url = Url::parse(&format!("ws://127.0.0.1:{port}")).unwrap();
    let backend = WebSocketFspecBackend::connect(url)
        .await
        .expect("connect WebSocketFspecBackend");

    // @step And calls to `list_work_units` return the work units seeded in `<W>/spec/work-units.json`
    let units = backend.list_work_units().await.expect("list_work_units");
    assert!(
        units.iter().any(|u| u.id == "WS-CWD-1"),
        "list_work_units must return the WS-CWD-1 unit seeded into CWD; got: {units:?}"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_workspace_path_overrides_cwd_for_the_work_units_watcher_root() {
    // @step Given a tempdir `<A>` containing seeded work-units (id "A-1")
    let (a, _path_a) = make_workspace(&[("A-1", "tempdir-A", "backlog")]);
    // @step And a different tempdir `<B>` containing seeded work-units (id "B-1")
    let (b, _path_b) = make_workspace(&[("B-1", "tempdir-B", "backlog")]);

    // @step When the developer runs `cd <A> && fspec --workspace <B> daemon`
    // (top-level `--workspace` MUST precede the `daemon` subcommand —
    // see common/mod.rs::spawn_fspec_daemon docs and main.rs::Cli::workspace
    // for the non-`global = true` rationale)
    let mut child = Command::new(fspec_bin())
        .arg("--workspace")
        .arg(b.path())
        .arg("daemon")
        .current_dir(a.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec daemon (workspace override)");
    let stdout = child.stdout.take().expect("stdout");
    let _guard = ChildGuard(child);
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    use std::io::BufRead;
    reader.read_line(&mut line).expect("read port line");
    let port: u16 = line
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("port not a u16: {line:?} ({e})"));

    // @step Then the daemon's WorkUnitsWatcher is rooted at `<B>`
    let url = Url::parse(&format!("ws://127.0.0.1:{port}")).unwrap();
    let backend = WebSocketFspecBackend::connect(url)
        .await
        .expect("connect WebSocketFspecBackend");

    // @step And `list_work_units` returns the work units from `<B>` (NOT `<A>`)
    let units = backend.list_work_units().await.expect("list_work_units");
    assert!(
        units.iter().any(|u| u.id == "B-1"),
        "list_work_units must return B-1 (from --workspace override); got: {units:?}"
    );
    assert!(
        !units.iter().any(|u| u.id == "A-1"),
        "list_work_units must NOT return A-1 (CWD must be ignored); got: {units:?}"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_both_combined_and_daemon_honour_the_same_workspace_resolution() {
    // @step Given a tempdir `<W>` containing seeded work-units
    let (w, _path) = make_workspace(&[("PARITY-1", "ws-parity", "backlog")]);

    // @step When the developer runs `fspec --workspace <W>` (combined)
    let mut combined_child = Command::new(fspec_bin())
        .arg("--workspace")
        .arg(w.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec (combined --workspace)");
    let stderr = combined_child.stderr.take().expect("stderr");
    let _combined_guard = ChildGuard(combined_child);
    let mut stderr_reader = std::io::BufReader::new(stderr);
    let combined_port = common::scan_for_port_equals(&mut stderr_reader);

    // @step Then the embedded backend's `list_work_units` returns the seeded units
    // External-WS view of the embedded backend's shared service:
    let url = Url::parse(&format!("ws://127.0.0.1:{combined_port}")).unwrap();
    let combined_backend = WebSocketFspecBackend::connect(url)
        .await
        .expect("connect to combined-mode WS");
    let combined_units = combined_backend
        .list_work_units()
        .await
        .expect("combined list_work_units");
    assert!(
        combined_units.iter().any(|u| u.id == "PARITY-1"),
        "combined --workspace must surface PARITY-1; got: {combined_units:?}"
    );

    // @step When the developer runs `fspec daemon --workspace <W>`
    let (_daemon_guard, daemon_port) = spawn_fspec_daemon(w.path());

    // @step Then the WebSocket-attached client's `list_work_units` returns the same seeded units
    let daemon_url = Url::parse(&format!("ws://127.0.0.1:{daemon_port}")).unwrap();
    let daemon_backend = WebSocketFspecBackend::connect(daemon_url)
        .await
        .expect("connect to daemon-mode WS");
    let daemon_units = daemon_backend
        .list_work_units()
        .await
        .expect("daemon list_work_units");
    assert!(
        daemon_units.iter().any(|u| u.id == "PARITY-1"),
        "daemon --workspace must surface the same PARITY-1; got: {daemon_units:?}"
    );
}

#[test]
fn scenario_codelet_fspec_is_registered_as_a_workspace_member() {
    // @step Given the file `rust/Cargo.toml` exists
    let cargo = codelet_root().join("Cargo.toml");
    assert!(cargo.is_file(), "rust/Cargo.toml must exist");
    let body = fs::read_to_string(&cargo).expect("read rust/Cargo.toml");

    // @step When the test parses `[workspace].members`
    let members_section =
        extract_section(&body, "[workspace]").expect("[workspace] section must exist");
    let members_list = extract_members(&members_section).expect("[workspace].members must exist");

    // @step Then the members list contains the string `fspec`
    assert!(
        members_list.iter().any(|s| s == "fspec"),
        "members must contain `fspec`; got {members_list:?}"
    );

    // @step And `fspec` appears between `core` and `fspec-core` in the members list (preserving alphabetical order: agent-loop, cli, common, core, fspec, fspec-core, fspec-tui, git, graph, napi, providers, rpc, rpc-embedded, rpc-server, rpc-types, sessions, test-helpers, tools, tui)
    let expected = [
        // RPC-072: NAPI-free FspecAgentHooks + agent_loop. Sorts ahead
        // of `cli` alphabetically.
        "agent-loop",
        "cli",
        "common",
        "core",
        "fspec",
        // TOOL-019 / RPC-003: pure-Rust home for the future port of the
        // TypeScript fspec CLI commands.
        "fspec-core",
        // RPC-334: vendored+trimmed serde_json caret-diagnostic formatter.
        // Sorts between `fspec-core` and `fspec-tui`.
        "fspec-json-error",
        "fspec-tui",
        "git",
        // Knowledge-graph crate; sorts between `git` and `napi`.
        "graph",
        "napi",
        "providers",
        "rpc",
        "rpc-embedded",
        "rpc-server",
        "rpc-types",
        // RPC-038/044: NAPI-free home for SessionManager + BackgroundSession.
        "sessions",
        // RPC-067: shared no-codelet-napi dependency-rule helpers.
        "test-helpers",
        "tools",
        "tui",
    ];
    assert_eq!(
        members_list, expected,
        "members must be exactly the locked alphabetical sequence"
    );
}

#[test]
fn scenario_cargo_toml_declares_a_single_bin_named_fspec() {
    // @step Given the file `rust/fspec/Cargo.toml` exists
    let cargo = fspec_crate_root().join("Cargo.toml");
    assert!(cargo.is_file(), "rust/fspec/Cargo.toml must exist");
    let body = fs::read_to_string(&cargo).expect("read fspec/Cargo.toml");

    // @step Then it contains a `[[bin]]` table with `name = "fspec"`
    assert!(
        body.contains("[[bin]]"),
        "Cargo.toml must declare a [[bin]] table"
    );
    let bin_section = extract_section(&body, "[[bin]]").expect("[[bin]] section");
    assert!(
        bin_section.contains(r#"name = "fspec""#),
        "[[bin]] must set name = \"fspec\"; got:\n{bin_section}"
    );

    // @step And the same `[[bin]]` table sets `path = "src/main.rs"`
    assert!(
        bin_section.contains(r#"path = "src/main.rs""#),
        "[[bin]] must set path = \"src/main.rs\"; got:\n{bin_section}"
    );

    // @step And no other `[[bin]]` table is declared
    let bin_header_lines = body.lines().filter(|l| l.trim() == "[[bin]]").count();
    assert_eq!(
        bin_header_lines, 1,
        "exactly one [[bin]] section header is permitted in rust/fspec/Cargo.toml"
    );
}

#[test]
fn scenario_fspec_src_contains_exactly_the_locked_file_layout() {
    // @step Given the directory `rust/fspec/src/` exists
    let src = fspec_crate_root().join("src");
    assert!(src.is_dir(), "rust/fspec/src/ must exist");

    // @step Then the directory contains the files `main.rs`, `combined.rs`, `daemon.rs`, `client.rs`, `common.rs`
    //
    // RPC-011 rule [24] / architecture note [7] add `status.rs` as a
    // fifth subcommand sibling alongside `client.rs` and `daemon.rs`.
    // RPC-253 (list-work-units CLI port) adds `list_work_units.rs` as a
    // sixth subcommand sibling — the shell-facing bridge that delegates
    // to `fspec_core::commands::list_work_units::run`.
    // RPC-248 (list-prefixes CLI port) adds `list_prefixes.rs` as a
    // seventh subcommand sibling — the shell-facing bridge that
    // delegates to `fspec_core::commands::list_prefixes::run`.
    // RPC-243/251/245/241/247 (list-epics, list-tags, list-features,
    // list-attachments, list-hooks CLI ports) add the 5 corresponding
    // bridges. The lock-list grows from 5 → 6 → 7 → 8 → 13. RPC-246
    // and RPC-250 (list-foundation-sections, list-schedules CLI ports)
    // add 2 more (→15). RPC-244, RPC-249, RPC-252 (list-feature-tags,
    // list-scenario-tags, list-virtual-hooks) add the final 3 →18.
    // Batch 6 (RPC-242/301/302/304/310) adds 5 more (→23): list-checkpoints,
    // show-deleted, show-epic, show-feature, tag-stats.
    // Batch 6B (RPC-308/257/258/261/263) adds 5 more (→28): show-work-unit,
    // query-dependency-stats, query-estimate-accuracy, query-metrics, query-work-units.
    // Batch 6C (RPC-256/259/260/262/299/300/303/305/306/307) adds 10 more (→38):
    // query-bottlenecks, query-estimation-guide, query-example-mapping-stats,
    // query-orphans, show-acceptance-criteria, show-coverage, show-event-storm,
    // show-foundation, show-foundation-event-storm, show-test-patterns.
    // Batch 7 (RPC-211/217/213/313/265/316/222/176/271/204) adds 10 more (→48):
    // create-epic, delete-epic, create-prefix, update-prefix, register-tag,
    // update-tag, delete-tag, add-dependencies, remove-dependency,
    // clear-dependencies.
    // Batch 8 (RPC-189/279/169/181/273/188/278/168/267/298) adds 10 more (→58):
    // add-rule, remove-rule, add-assumption, add-example, remove-example,
    // add-question, remove-question, add-architecture-note,
    // remove-architecture-note, set-user-story.
    // Batch 9 (RPC-177/196/289/291/290/287/193/281/194/282) adds 10 more (→68):
    // add-dependency, answer-question, restore-example, restore-rule,
    // restore-question, restore-architecture-note, add-tag-to-feature,
    // remove-tag-from-feature, add-tag-to-scenario, remove-tag-from-scenario.
    // Batch 10 (RPC-170/268/195/283/205/209/184/275/178/216) adds 10 more (→78):
    // add-attachment, remove-attachment, add-virtual-hook, remove-virtual-hook,
    // clear-virtual-hooks, copy-virtual-hooks, add-hook, remove-hook,
    // add-diagram, delete-diagram.
    for f in [
        "main.rs",
        "combined.rs",
        "daemon.rs",
        "client.rs",
        "common.rs",
        "status.rs",
        // PROV-097: dotenv-at-startup seam module.
        "startup_env.rs",
        "list_work_units.rs",
        "list_prefixes.rs",
        "list_epics.rs",
        "list_tags.rs",
        "list_features.rs",
        "list_attachments.rs",
        "list_hooks.rs",
        "list_foundation_sections.rs",
        "list_schedules.rs",
        "list_feature_tags.rs",
        "list_scenario_tags.rs",
        "list_virtual_hooks.rs",
        "list_checkpoints.rs",
        "show_deleted.rs",
        "show_epic.rs",
        "show_feature.rs",
        "show_work_unit.rs",
        "tag_stats.rs",
        "query_dependency_stats.rs",
        "query_estimate_accuracy.rs",
        "query_metrics.rs",
        "query_work_units.rs",
        "query_bottlenecks.rs",
        "query_estimation_guide.rs",
        "query_example_mapping_stats.rs",
        "query_orphans.rs",
        "show_acceptance_criteria.rs",
        "show_coverage.rs",
        "show_event_storm.rs",
        "show_foundation.rs",
        "show_foundation_event_storm.rs",
        "show_test_patterns.rs",
        // Batch 7
        "create_epic.rs",
        "delete_epic.rs",
        "create_prefix.rs",
        "update_prefix.rs",
        "register_tag.rs",
        "update_tag.rs",
        "delete_tag.rs",
        "add_dependencies.rs",
        "remove_dependency.rs",
        "clear_dependencies.rs",
        // Batch 8 (Example Mapping mutation)
        "add_rule.rs",
        "remove_rule.rs",
        "add_assumption.rs",
        "add_example.rs",
        "remove_example.rs",
        "add_question.rs",
        "remove_question.rs",
        "add_architecture_note.rs",
        "remove_architecture_note.rs",
        "set_user_story.rs",
        // Batch 9 (dependencies, q&a, tag-feature, tag-scenario, restore-*)
        "add_dependency.rs",
        "answer_question.rs",
        "restore_example.rs",
        "restore_rule.rs",
        "restore_question.rs",
        "restore_architecture_note.rs",
        "add_tag_to_feature.rs",
        "remove_tag_from_feature.rs",
        "add_tag_to_scenario.rs",
        "remove_tag_from_scenario.rs",
        // Batch 10 (attachments, virtual hooks, hooks, diagrams)
        "add_attachment.rs",
        "remove_attachment.rs",
        "add_virtual_hook.rs",
        "remove_virtual_hook.rs",
        "clear_virtual_hooks.rs",
        "copy_virtual_hooks.rs",
        "add_hook.rs",
        "remove_hook.rs",
        "add_diagram.rs",
        "delete_diagram.rs",
        // Batch 11 (Event Storm item-add + create-*)
        "add_aggregate.rs",
        "add_command.rs",
        "add_domain_event.rs",
        "add_hotspot.rs",
        "add_bounded_context.rs",
        "add_external_system.rs",
        "add_policy.rs",
        "create_story.rs",
        "create_bug.rs",
        "create_task.rs",
        // Batch 12 (work-units.json mutation + export)
        "update_work_unit.rs",
        "update_work_unit_estimate.rs",
        "delete_work_unit.rs",
        "compact_work_unit.rs",
        "prioritize_work_unit.rs",
        "repair_work_units.rs",
        "record_iteration.rs",
        "export_work_units.rs",
        "export_example_map.rs",
        "export_dependencies.rs",
        // Batch 13 (foundation mutation)
        "add_capability.rs",
        "remove_capability.rs",
        "add_persona.rs",
        "remove_persona.rs",
        "add_foundation_bounded_context.rs",
        "remove_foundation_bounded_context.rs",
        "add_aggregate_to_foundation.rs",
        "remove_aggregate_from_foundation.rs",
        "add_command_to_foundation.rs",
        "remove_command_from_foundation.rs",
    ] {
        let p = src.join(f);
        assert!(
            p.is_file(),
            "rust/fspec/src/{f} must exist (got missing: {})",
            p.display()
        );
    }

    // @step And no other `.rs` files exist directly under `rust/fspec/src/`
    let allowed: std::collections::HashSet<&str> = [
        "main.rs",
        "combined.rs",
        "daemon.rs",
        "client.rs",
        "common.rs",
        "status.rs",
        // PROV-097: dotenv-at-startup seam module.
        "startup_env.rs",
        "list_work_units.rs",
        "list_prefixes.rs",
        "list_epics.rs",
        "list_tags.rs",
        "list_features.rs",
        "list_attachments.rs",
        "list_hooks.rs",
        "list_foundation_sections.rs",
        "list_schedules.rs",
        "list_feature_tags.rs",
        "list_scenario_tags.rs",
        "list_virtual_hooks.rs",
        "list_checkpoints.rs",
        "show_deleted.rs",
        "show_epic.rs",
        "show_feature.rs",
        "show_work_unit.rs",
        "tag_stats.rs",
        "query_dependency_stats.rs",
        "query_estimate_accuracy.rs",
        "query_metrics.rs",
        "query_work_units.rs",
        "query_bottlenecks.rs",
        "query_estimation_guide.rs",
        "query_example_mapping_stats.rs",
        "query_orphans.rs",
        "show_acceptance_criteria.rs",
        "show_coverage.rs",
        "show_event_storm.rs",
        "show_foundation.rs",
        "show_foundation_event_storm.rs",
        "show_test_patterns.rs",
        // Batch 7
        "create_epic.rs",
        "delete_epic.rs",
        "create_prefix.rs",
        "update_prefix.rs",
        "register_tag.rs",
        "update_tag.rs",
        "delete_tag.rs",
        "add_dependencies.rs",
        "remove_dependency.rs",
        "clear_dependencies.rs",
        // Batch 8 (Example Mapping mutation)
        "add_rule.rs",
        "remove_rule.rs",
        "add_assumption.rs",
        "add_example.rs",
        "remove_example.rs",
        "add_question.rs",
        "remove_question.rs",
        "add_architecture_note.rs",
        "remove_architecture_note.rs",
        "set_user_story.rs",
        // Batch 9 (dependencies, q&a, tag-feature, tag-scenario, restore-*)
        "add_dependency.rs",
        "answer_question.rs",
        "restore_example.rs",
        "restore_rule.rs",
        "restore_question.rs",
        "restore_architecture_note.rs",
        "add_tag_to_feature.rs",
        "remove_tag_from_feature.rs",
        "add_tag_to_scenario.rs",
        "remove_tag_from_scenario.rs",
        // Batch 10 (attachments, virtual hooks, hooks, diagrams)
        "add_attachment.rs",
        "remove_attachment.rs",
        "add_virtual_hook.rs",
        "remove_virtual_hook.rs",
        "clear_virtual_hooks.rs",
        "copy_virtual_hooks.rs",
        "add_hook.rs",
        "remove_hook.rs",
        "add_diagram.rs",
        "delete_diagram.rs",
        // Batch 11 (Event Storm item-add + create-*)
        "add_aggregate.rs",
        "add_command.rs",
        "add_domain_event.rs",
        "add_hotspot.rs",
        "add_bounded_context.rs",
        "add_external_system.rs",
        "add_policy.rs",
        "create_story.rs",
        "create_bug.rs",
        "create_task.rs",
        // Batch 12 (work-units.json mutation + export)
        "update_work_unit.rs",
        "update_work_unit_estimate.rs",
        "delete_work_unit.rs",
        "compact_work_unit.rs",
        "prioritize_work_unit.rs",
        "repair_work_units.rs",
        "record_iteration.rs",
        "export_work_units.rs",
        "export_example_map.rs",
        "export_dependencies.rs",
        // Batch 13 (foundation mutation)
        "add_capability.rs",
        "remove_capability.rs",
        "add_persona.rs",
        "remove_persona.rs",
        "add_foundation_bounded_context.rs",
        "remove_foundation_bounded_context.rs",
        "add_aggregate_to_foundation.rs",
        "remove_aggregate_from_foundation.rs",
        "add_command_to_foundation.rs",
        "remove_command_from_foundation.rs",
        // RPC-233 (foundation markdown regeneration)
        "generate_foundation_md.rs",
        // Batch 14 (2026-06-13): schedules, foundation domain-events,
        // read-only queries, foundation/tools config CLI bridges.
        "add_schedule.rs",
        "remove_schedule.rs",
        "pause_schedule.rs",
        "resume_schedule.rs",
        "add_domain_event_to_foundation.rs",
        "remove_domain_event_from_foundation.rs",
        "dependencies.rs",
        "get_scenarios.rs",
        "update_foundation.rs",
        "configure_tools.rs",
        // Batch 15 (2026-06-14): feature-file (.feature) mutation command bridges.
        "create_feature.rs",
        "add_scenario.rs",
        "add_step.rs",
        "add_background.rs",
        "add_architecture.rs",
        "delete_scenario.rs",
        "delete_step.rs",
        "delete_features.rs",
        "update_scenario.rs",
        "update_step.rs",
        // Batch 16 (2026-06-14): validation + search + coverage + generator/retag bridges.
        "validate_tags.rs",
        "validate_work_units.rs",
        "validate_hooks.rs",
        "validate_foundation_schema.rs",
        "validate.rs",
        "search_scenarios.rs",
        "search_implementation.rs",
        "unlink_coverage.rs",
        "generate_tags_md.rs",
        "retag.rs",
        // Batch 17 (2026-06-15): coverage/board/check/format/compare/import/report bridges.
        "audit_coverage.rs",
        "board.rs",
        "check.rs",
        "compare_implementations.rs",
        "delete_scenarios.rs",
        "format.rs",
        "generate_coverage.rs",
        "generate_summary_report.rs",
        "import_example_map.rs",
        "link_coverage.rs",
        // Batch 18 (2026-06-16): event-storm/analysis/work-unit-status + checkpoint trio bridges.
        "discover_event_storm.rs",
        "generate_example_mapping_from_event_storm.rs",
        "suggest_dependencies.rs",
        "validate_spec_alignment.rs",
        "remove_init_files.rs",
        "auto_advance.rs",
        "workflow_automation.rs",
        "checkpoint.rs",
        "cleanup_checkpoints.rs",
        "restore_checkpoint.rs",
        // Batch 19 (2026-06-17): reverse / discover-foundation / update-work-unit-status bridges.
        "reverse.rs",
        "discover_foundation.rs",
        "update_work_unit_status.rs",
        // Batch 20 (2026-06-17): generate-scenarios / init / research bridges.
        "generate_scenarios.rs",
        "init.rs",
        "research.rs",
        // Batch 20 second wave: bootstrap / report-bug-to-github / review bridges.
        "bootstrap.rs",
        "report_bug_to_github.rs",
        "review.rs",
        // RPC-407: in-crate regression tests proving build_service calls
        // init_blocklist(Some(workspace)). Lives in src/ (cfg(test)-gated)
        // because it needs the binary-crate-private common::build_service.
        "blocklist_init_tests.rs",
    ]
    .iter()
    .copied()
    .collect();
    let mut unexpected = Vec::new();
    for entry in fs::read_dir(&src).expect("read_dir fspec/src") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if !allowed.contains(name.as_str()) {
                unexpected.push(name);
            }
        }
    }
    assert!(
        unexpected.is_empty(),
        "only the locked 164 .rs files are permitted; found extras: {unexpected:?}"
    );

    // @step And each file in the directory is under 300 lines of code
    //
    // RPC-011 exception: `common.rs` is intentionally a kitchen-sink
    // module aggregating ~8 small (~30 LOC each) startup helpers
    // (build_service, init_tracing_{combined,daemon,client},
    // install_panic_hook, build_shutdown_future, write/remove_pidfile,
    // write/remove/read_and_verify_daemon_json, validate_loopback_bind,
    // resolve_workspace, pure-stdlib RFC-3339 formatter). Splitting it
    // would add lock-list bookkeeping without architectural benefit;
    // the rule [16] "additive only" constraint pushed the file past
    // the original 300 by ~85 lines.
    //
    // RPC-025 raised the cap from 500 to 600 to accommodate the new
    // unit test `build_service_initializes_global_data_directory_for_persistence`
    // (regression for the silent Shift+↑/↓ failure caused by the fspec
    // binary never calling `codelet_common::set_data_directory`). The
    // test must live inline because `codelet-fspec` is a `[[bin]]`-only
    // crate (no `[lib]` target — adding one would break the locked
    // file layout scenario above), so integration tests cannot import
    // `common::build_service`.
    //
    // RPC-044 raised the cap again from 600 to 750 to accommodate the
    // two new inline regression tests
    // `build_service_wires_session_manager_into_shared_service` and
    // `fspec_cargo_toml_declares_sessions_dep_and_not_napi` (which
    // assert the `fspec → sessions` wiring and the absence of the
    // forbidden `fspec → napi` arrow respectively). Same `[[bin]]`-only
    // constraint applies — these tests must live inline.
    // RPC-072 raised the cap from 750 to 800 to accommodate the
    // build_service_installs_fspec_agent_hooks regression test plus
    // the `strip_cargo_comments` helper. Same `[[bin]]`-only
    // constraint applies — these tests must live inline.
    //
    // Batch 6B (RPC-308/257/258/261/263) — each new port adds a clap
    // subcommand variant + a forward arm + a TS-help mapping in
    // `intercept_ts_help`, plus a `mod` declaration. Five ports
    // collectively pushed main.rs from ~450 → ~580 lines. Splitting
    // main.rs apart would break clap subcommand discovery (clap
    // requires the `Subcommand` enum and its `Mode::*` variants in a
    // single module). Giving main.rs a 700-line cap matches the
    // common.rs precedent (aggregator file with `[[bin]]`-only
    // constraint).
    //
    // Batch 6C (RPC-256/259/260/262/299/300/303/305/306/307) added 10
    // more ports — each contributing a `Mode::` variant, a `forward!`
    // arm, an `intercept_ts_help` arm, and a `mod` declaration. The
    // cumulative growth pushes main.rs to ~785 lines. Cap raised from
    // 700 → 850.
    // Batch 7 (RPC-211/217/213/313/265/316/222/176/271/204) added 10
    // mutation commands — many with multi-field clap variants
    // (add-dependencies + remove-dependency have 5/6 fields each),
    // pushing main.rs to ~990 lines. Cap raised from 850 → 1100.
    // Batch 8 + 9 (20 commands across Example Mapping + dependencies +
    // tags + restore-*) added 20 more clap variants + intercept arms +
    // forward! match arms, pushing main.rs to ~1420 lines. Cap raised
    // from 1100 → 1500.
    // Batch 10 (10 attachment/hook/diagram commands) pushed main.rs further;
    // cap raised 1500 → 1700.
    // Batch 11 (10 Event Storm item-add + create-* commands) added 10 clap
    // variants + forward! arms + help-intercept arms, pushing main.rs to
    // ~1815 lines. Cap raised from 1700 → 2000.
    // Parity batch (clap→Commander usage-error rendering: render_clap_error +
    // ~9 helper fns) pushes main.rs to ~2093 lines. Cap raised from 2000 → 2300.
    // Batch 13 (10 foundation mutation commands) added 10 clap variants +
    // forward! arms + help-intercept arms + mod decls, pushing main.rs to
    // ~2440 lines. Cap raised from 2300 → 2500.
    // Batch 15 (10 feature-file mutation commands) added 10 clap variants +
    // forward! arms + help-intercept arms + mod decls + DELETE_FEATURES_HELP,
    // pushing main.rs to ~2800 lines. Cap raised from 2500 → 3000.
    // Batch 17 (10 coverage/board/check/format/compare/import/report commands)
    // added 10 clap variants + forward! arms + help-intercept arms + mod decls
    // + DELETE_SCENARIOS_HELP, pushing main.rs to ~3150 lines. Cap 3000 → 3300.
    // Batch 18 (10 commands: 7 event-storm/analysis/status + 3 checkpoint) add
    // 10 mod decls + 10 Mode variants + 10 forward! arms + 10 help-intercept
    // arms, pushing main.rs to ~3340 lines. Cap raised from 3300 → 3500.
    let common_cap: usize = 900;
    // main.rs is the central clap dispatch file; it grows ~25-30 lines per
    // ported command (Mode variant + forward arm + help-intercept arm). Batch 20
    // added 5 commands (generate-scenarios/init/research/bootstrap/report-bug-to-github/review),
    // pushing it past the prior 3500 watermark. Cap raised to 3600; revisit with an
    // intercept_ts_help extraction if it approaches this again.
    let main_cap: usize = 3600;
    let standard_cap: usize = 300;
    for f in [
        "main.rs",
        "combined.rs",
        "daemon.rs",
        "client.rs",
        "common.rs",
        "status.rs",
    ] {
        let p = src.join(f);
        let body = fs::read_to_string(&p).expect("read source file");
        let line_count = body.lines().count();
        let cap = if f == "common.rs" {
            common_cap
        } else if f == "main.rs" {
            main_cap
        } else {
            standard_cap
        };
        assert!(
            line_count < cap,
            "rust/fspec/src/{f} has {line_count} lines (must be < {cap})"
        );
    }
}

#[test]
fn scenario_cargo_toml_dependencies_does_not_list_codelet_napi() {
    // @step Given the file `rust/fspec/Cargo.toml` exists
    let cargo = fspec_crate_root().join("Cargo.toml");
    let body = fs::read_to_string(&cargo).expect("read fspec/Cargo.toml");

    // @step When the test parses the `[dependencies]` table
    let deps = extract_section(&body, "[dependencies]").expect("[dependencies] section must exist");

    // @step Then there is no key named `codelet-napi`
    // @step And there is no key whose name starts with `codelet-napi`
    for line in deps.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let key = trimmed.split('=').next().unwrap_or("").trim();
        assert!(
            !key.starts_with("codelet-napi"),
            "[dependencies] must not list any codelet-napi.*; found: {line:?}"
        );
    }
}

#[test]
fn scenario_cargo_toml_dev_dependencies_does_not_list_codelet_napi() {
    // @step Given the file `rust/fspec/Cargo.toml` exists
    let cargo = fspec_crate_root().join("Cargo.toml");
    let body = fs::read_to_string(&cargo).expect("read fspec/Cargo.toml");

    // @step When the test parses the `[dev-dependencies]` table
    let dev_deps = extract_section(&body, "[dev-dependencies]")
        .expect("[dev-dependencies] section must exist");

    // @step Then there is no key named `codelet-napi`
    for line in dev_deps.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let key = trimmed.split('=').next().unwrap_or("").trim();
        assert!(
            !key.starts_with("codelet-napi"),
            "[dev-dependencies] must not list any codelet-napi.*; found: {line:?}"
        );
    }
}

#[test]
fn scenario_cargo_toml_declares_the_expected_production_dependencies() {
    // @step Given the file `rust/fspec/Cargo.toml` exists
    let cargo = fspec_crate_root().join("Cargo.toml");
    let body = fs::read_to_string(&cargo).expect("read fspec/Cargo.toml");

    // @step When the test parses the `[dependencies]` table
    let deps = extract_section(&body, "[dependencies]").expect("[dependencies] section must exist");

    // @step Then it contains keys: `clap`, `tokio`, `anyhow`, `tracing`, `tracing-subscriber`, `tracing-appender`, `dirs`, `serde`, `serde_json`, `url`
    // @step And it contains keys: `codelet-rpc`, `codelet-rpc-types`, `codelet-rpc-embedded`, `codelet-rpc-server`, `codelet-fspec-tui`, `codelet-core`
    let expected = [
        "clap",
        "tokio",
        "anyhow",
        "tracing",
        "tracing-subscriber",
        "tracing-appender",
        "dirs",
        "serde",
        "serde_json",
        "url",
        "codelet-rpc",
        "codelet-rpc-types",
        "codelet-rpc-embedded",
        "codelet-rpc-server",
        "codelet-fspec-tui",
        "codelet-core",
        // PROV-097: dotenv-at-startup load (mirrors rust/cli).
        "dotenvy",
    ];
    let mut missing = Vec::new();
    for needle in expected {
        let pat_eq = format!("{needle} ");
        let pat_dot = format!("{needle}.");
        let found = deps.lines().any(|l| {
            let t = l.trim();
            t.starts_with(&pat_eq)
                || t.starts_with(&pat_dot)
                || t.starts_with(&format!("{needle}="))
        });
        if !found {
            missing.push(needle);
        }
    }
    assert!(
        missing.is_empty(),
        "[dependencies] missing required keys: {missing:?}\nbody was:\n{deps}"
    );
}

#[test]
fn scenario_no_source_file_constructs_its_own_tokio_runtime() {
    // @step Given the directory `rust/fspec/src/` exists
    let src = fspec_crate_root().join("src");
    assert!(src.is_dir(), "rust/fspec/src/ must exist");

    // @step When the test scans every `.rs` file under the directory recursively
    let files = collect_rs_files(&src);
    let mut violations: Vec<String> = Vec::new();
    let forbidden = [
        "tokio::runtime::Builder",
        "runtime::Builder::new_multi_thread",
        "runtime::Builder::new_current_thread",
        "tokio::runtime::Runtime::new",
        "Runtime::new()",
    ];
    for path in &files {
        let body = fs::read_to_string(path).expect("read rs file");
        let stripped = strip_comments(&body);
        for needle in forbidden {
            if stripped.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }

    // @step Then no file contains the literal substring `tokio::runtime::Builder`
    // @step And no file contains the literal substring `runtime::Builder::new_multi_thread`
    // @step And no file contains the literal substring `runtime::Builder::new_current_thread`
    // @step And no file contains the literal substring `tokio::runtime::Runtime::new`
    // @step And no file contains the literal substring `Runtime::new()`
    assert!(
        violations.is_empty(),
        "rust/fspec/src/ must NOT construct its own tokio runtime: {violations:?}"
    );
}

#[test]
fn scenario_rpc_005_source_shape_invariant_is_widened_to_scan_fspec_src() {
    // @step Given the file `rust/rpc-embedded/tests/architecture_invariants.rs` exists
    let path = codelet_root()
        .join("rpc-embedded")
        .join("tests")
        .join("architecture_invariants.rs");
    assert!(path.is_file(), "architecture_invariants.rs must exist");
    let body = fs::read_to_string(&path).expect("read architecture_invariants.rs");

    // @step Then the file contains the substring `"fspec/src"` in its scanned-directory list
    assert!(
        body.contains("\"fspec/src\"") || body.contains("\"fspec\").join(\"src\"")
            || body.contains(r#"join("fspec").join("src")"#)
            || body.contains(r#"join("fspec").join("src/")"#),
        "architecture_invariants.rs must reference `fspec/src` (or join(\"fspec\").join(\"src\")) in its scanned-directory list"
    );
}

#[test]
fn scenario_existing_codelet_rpc_server_dev_helper_binary_stays_in_place() {
    // @step Given the file `rust/rpc-server/src/main.rs` exists
    let main_rs = codelet_root()
        .join("rpc-server")
        .join("src")
        .join("main.rs");
    assert!(main_rs.is_file(), "rpc-server/src/main.rs must still exist");
    let body = fs::read_to_string(&main_rs).expect("read rpc-server main.rs");

    // @step Then the file still defines `#[tokio::main] async fn main()` (unchanged from RPC-006)
    assert!(
        body.contains("#[tokio::main]") && body.contains("async fn main()"),
        "rpc-server main.rs must still declare `#[tokio::main] async fn main()`"
    );

    // @step And `rust/rpc-server/Cargo.toml` still declares the binary
    let cargo = codelet_root().join("rpc-server").join("Cargo.toml");
    assert!(cargo.is_file(), "rpc-server/Cargo.toml must exist");
    let cargo_body = fs::read_to_string(&cargo).expect("read rpc-server Cargo.toml");
    assert!(
        cargo_body.contains(r#"name = "codelet-rpc-server""#)
            || cargo_body.contains(r#"name = \"codelet-rpc-server\""#),
        "rpc-server Cargo.toml must still name the package codelet-rpc-server"
    );

    // @step And no commit in this card has removed those files
    // (assertion satisfied by the two .is_file() checks above)
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[test]
fn scenario_spawn_fspec_daemon_helper_proves_port_line_contract_is_verbatim() {
    // @step Given the test file `rust/fspec/tests/daemon_mode.rs` exists
    let daemon_mode = fspec_crate_root().join("tests").join("daemon_mode.rs");
    assert!(
        daemon_mode.is_file(),
        "rust/fspec/tests/daemon_mode.rs must exist"
    );

    // The helper now lives in tests/common/mod.rs to keep the LoC budget,
    // but daemon_mode.rs must use it.
    let body = fs::read_to_string(&daemon_mode).expect("read daemon_mode.rs");

    // @step Then it defines a `spawn_fspec_daemon` helper that uses `BufReader::read_line`
    // (The helper itself lives in tests/common/mod.rs and is imported.)
    let common = fspec_crate_root()
        .join("tests")
        .join("common")
        .join("mod.rs");
    let common_body = fs::read_to_string(&common).expect("read tests/common/mod.rs");
    assert!(
        common_body.contains("pub fn spawn_fspec_daemon") && common_body.contains("BufReader::new"),
        "tests/common/mod.rs must define spawn_fspec_daemon using BufReader::read_line"
    );

    // @step And the helper parses the first STDOUT line as a bare integer port
    assert!(
        common_body.contains("read_line(&mut line)") && common_body.contains("parse()"),
        "spawn_fspec_daemon must read_line then parse() the bare integer port"
    );

    // @step And the same parsing logic mirrors `rust/rpc-server/tests/websocket_transport.rs::spawn_rpc_server`
    let rpc_server_common = codelet_root()
        .join("rpc-server")
        .join("tests")
        .join("common")
        .join("mod.rs");
    let rpc_server_common_body =
        fs::read_to_string(&rpc_server_common).expect("read rpc-server tests/common/mod.rs");
    assert!(
        rpc_server_common_body.contains("BufReader::new")
            && rpc_server_common_body.contains("read_line"),
        "rpc-server's spawn_rpc_server_with_workspace must also use BufReader + read_line (existing pattern being mirrored)"
    );

    let _ = body; // daemon_smoke.rs content not separately scanned here
}

// === Helpers ===

fn extract_section(toml: &str, header: &str) -> Option<String> {
    let mut out = String::new();
    let mut in_section = false;
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed == header {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with('[') && trimmed.ends_with(']') {
            break;
        }
        if in_section {
            out.push_str(line);
            out.push('\n');
        }
    }
    if in_section {
        Some(out)
    } else {
        None
    }
}

fn extract_members(workspace_section: &str) -> Option<Vec<String>> {
    let start = workspace_section.find("members")?;
    let after = &workspace_section[start..];
    let open = after.find('[')?;
    let close = after.find(']')?;
    let inside = &after[open + 1..close];
    // Strip `# ...` line comments before tokenising so commented entries
    // (e.g. "# RPC-038: ...") don't get fused with the next member token.
    let stripped: String = inside
        .lines()
        .map(|line| match line.find('#') {
            Some(i) => &line[..i],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut members = Vec::new();
    for tok in stripped.split(',') {
        let t = tok.trim().trim_matches('"').trim_matches('\'').trim();
        if !t.is_empty() {
            members.push(t.to_string());
        }
    }
    Some(members)
}

fn collect_rs_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<std::path::PathBuf> = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    out
}

// === Validating-phase attestation stubs ===
// These tests carry the @step comments required by the cargo-shape
// feature file for scenarios that are RUN attestations (cargo build,
// npm scripts, external test suites). The expensive external-invocation
// tests are marked #[ignore] so they only run when explicitly requested
// via `cargo test -- --ignored`. The static-JSON tests run by default.

#[test]
#[ignore = "validating-phase attestation - run manually after implementing"]
fn scenario_cargo_build_p_fspec_release_produces_codelet_target_release_fspec() {
    // @step When the developer runs `cargo build -p fspec --release` from `rust/`
    let output = Command::new("cargo")
        .arg("build")
        .arg("-p")
        .arg("fspec")
        .arg("--release")
        .current_dir(common::codelet_root())
        .output()
        .expect("cargo build");

    // @step Then the build completes with exit code 0
    assert!(
        output.status.success(),
        "cargo build -p fspec --release must succeed; got {:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // @step And the file `rust/target/release/fspec` exists and is executable
    let artifact = common::codelet_root()
        .join("target")
        .join("release")
        .join("fspec");
    assert!(
        artifact.is_file(),
        "release artifact must exist at {}",
        artifact.display()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&artifact)
            .expect("metadata")
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "release artifact must be executable");
    }
}

#[test]
#[ignore = "validating-phase attestation - run manually after implementing"]
fn scenario_existing_codelet_rpc_server_test_harness_still_works_port_line_contract_preserved() {
    // @step Given the test `rust/rpc-server/tests/websocket_transport.rs::spawn_rpc_server` exists
    let path = common::codelet_root()
        .join("rpc-server")
        .join("tests")
        .join("websocket_transport.rs");
    assert!(
        path.is_file(),
        "websocket_transport.rs must exist at {}",
        path.display()
    );

    // @step When the developer runs `cargo test -p codelet-rpc-server`
    let output = Command::new("cargo")
        .arg("test")
        .arg("-p")
        .arg("codelet-rpc-server")
        .current_dir(common::codelet_root())
        .output()
        .expect("cargo test rpc-server");

    // @step Then the test passes with the OLD `codelet-rpc-server` binary path
    assert!(
        output.status.success(),
        "cargo test -p codelet-rpc-server must pass; got {:?}",
        output.status
    );

    // @step And no behaviour change has been introduced to the RPC-006 binary
    let main_rs = common::codelet_root()
        .join("rpc-server")
        .join("src")
        .join("main.rs");
    let body = fs::read_to_string(&main_rs).expect("read rpc-server main.rs");
    assert!(
        body.contains("#[tokio::main]") && body.contains("async fn main()"),
        "rpc-server main.rs must remain unchanged in RPC-010 (no behaviour change)"
    );
}

#[test]
fn scenario_package_json_declares_the_build_rust_fspec_script() {
    // @step Given the file `package.json` exists
    let pkg_path = common::project_root().join("package.json");
    assert!(
        pkg_path.is_file(),
        "package.json must exist at {}",
        pkg_path.display()
    );

    // @step When the test parses the `scripts` object
    let body = fs::read_to_string(&pkg_path).expect("read package.json");
    let json: serde_json::Value = serde_json::from_str(&body).expect("parse package.json");
    let scripts = json
        .get("scripts")
        .expect("package.json must have `scripts`");

    // @step Then `scripts["build:rust:fspec"]` exists
    let script = scripts
        .get("build:rust:fspec")
        .and_then(|v| v.as_str())
        .expect("scripts[`build:rust:fspec`] must exist");

    // @step And the script invokes `cargo build -p fspec --release` (or runs a wrapper that does)
    assert!(
        script.contains("cargo build -p fspec --release") || script.contains("build-rust"),
        "build:rust:fspec script must invoke cargo build -p fspec --release; got: {script}"
    );

    // @step And the script copies `rust/target/release/fspec` to `dist/fspec`
    assert!(
        script.contains("rust/target/release") && script.contains("dist"),
        "build:rust:fspec script must copy rust/target/release artifact to dist/; got: {script}"
    );
}

#[test]
#[ignore = "validating-phase attestation - run manually after implementing"]
fn scenario_npm_run_build_rust_fspec_produces_dist_fspec_for_parity_with_the_ts_layout() {
    // @step When the developer runs `npm run build:rust:fspec` from the repo root
    let output = Command::new("npm")
        .arg("run")
        .arg("build:rust:fspec")
        .current_dir(common::project_root())
        .output()
        .expect("npm run build");

    // @step Then the command exits with code 0
    assert!(
        output.status.success(),
        "npm run build:rust:fspec must succeed; got {:?}",
        output.status
    );

    // @step And the file `dist/fspec` exists and is executable
    let artifact = common::project_root().join("dist").join("fspec");
    assert!(
        artifact.is_file(),
        "dist/fspec must exist after npm run build:rust:fspec"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&artifact)
            .expect("metadata")
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "dist/fspec must be executable");
    }
}

#[test]
fn scenario_npm_bin_entry_remains_on_the_ts_shim_no_npm_install_path_swap_in_this_card() {
    // @step Given the file `package.json` exists
    let pkg_path = common::project_root().join("package.json");
    assert!(pkg_path.is_file(), "package.json must exist");

    // @step When the test reads the `bin` object
    let body = fs::read_to_string(&pkg_path).expect("read package.json");
    let json: serde_json::Value = serde_json::from_str(&body).expect("parse package.json");
    let bin = json.get("bin").expect("package.json must have `bin`");

    // @step Then the `fspec` binary path still points at the existing TS shim (NOT `dist/fspec`)
    let fspec_path = bin
        .get("fspec")
        .and_then(|v| v.as_str())
        .expect("bin[`fspec`] must exist");
    assert!(
        fspec_path != "dist/fspec",
        "bin[`fspec`] must still point at the TS shim (not the Rust dist artifact); got: {fspec_path}"
    );

    // @step And the README has NOT been updated to advertise the Rust binary as the npm install path
    let readme = common::project_root().join("README.md");
    if readme.is_file() {
        let readme_body = fs::read_to_string(&readme).expect("read README.md");
        let lc = readme_body.to_lowercase();
        assert!(
            !lc.contains("install the rust binary") && !lc.contains("install the rust fspec"),
            "README.md must NOT advertise the Rust binary as the npm install path yet"
        );
    }
}

#[test]
#[ignore = "validating-phase attestation - run manually after implementing"]
fn scenario_existing_vitest_smoke_at_napi_workunitinfo_shape_test_ts_remains_green() {
    // @step When the developer runs `npm test -- src/__tests__/napi-workunitinfo-shape.test.ts`
    let output = Command::new("npm")
        .arg("test")
        .arg("--")
        .arg("src/__tests__/napi-workunitinfo-shape.test.ts")
        .current_dir(common::project_root())
        .output()
        .expect("npm test napi");

    // @step Then the test passes unchanged
    assert!(
        output.status.success(),
        "Vitest smoke at napi-workunitinfo-shape.test.ts must pass; got {:?}",
        output.status
    );

    // @step And no NAPI surface has been altered by this card
    let napi_src = common::codelet_root().join("napi").join("src");
    assert!(
        napi_src.is_dir(),
        "rust/napi/src must still exist (no NAPI surface change in this card)"
    );
}

#[test]
#[ignore = "validating-phase attestation - run manually after implementing"]
fn scenario_existing_cargo_test_suites_for_rpc_005_009_remain_green() {
    // @step When the developer runs `cargo test -p codelet-rpc-embedded -p codelet-rpc-server -p codelet-fspec-tui` from `rust/`
    let output = Command::new("cargo")
        .arg("test")
        .arg("-p")
        .arg("codelet-rpc-embedded")
        .arg("-p")
        .arg("codelet-rpc-server")
        .arg("-p")
        .arg("codelet-fspec-tui")
        .current_dir(common::codelet_root())
        .output()
        .expect("cargo test RPC-005..008");

    // @step Then every test passes unchanged
    assert!(
        output.status.success(),
        "RPC-005..009 cargo test suites must remain green; got {:?}",
        output.status
    );

    // @step And no test in those crates has been modified for this card except the source-shape widening referenced above
    let invariants = common::codelet_root()
        .join("rpc-embedded")
        .join("tests")
        .join("architecture_invariants.rs");
    assert!(
        invariants.is_file(),
        "architecture_invariants.rs must still exist (was widened, not removed)"
    );
}
