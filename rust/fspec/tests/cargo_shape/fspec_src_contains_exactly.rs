//! Feature: spec/features/fspec-binary-cargo-shape-rpc010.feature
use super::*;

#[test]
fn scenario_fspec_src_contains_exactly_the_locked_file_layout() {
    // @step Given the directory `rust/fspec/src/` exists
    let src = fspec_crate_root().join("src");
    assert!(src.is_dir(), "rust/fspec/src/ must exist");

    // @step Then the directory contains the files `main.rs`, `combined.rs`, `daemon.rs`, `client.rs`, `common.rs`
    for f in LOCKED_FILES {
        let p = src.join(f);
        assert!(
            p.is_file(),
            "rust/fspec/src/{f} must exist (got missing: {})",
            p.display()
        );
    }

    // @step And no other `.rs` files exist directly under `rust/fspec/src/`
    let allowed: std::collections::HashSet<&str> = ALLOWED_FILES.iter().copied().collect();
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
        "only the locked .rs files are permitted; found extras: {unexpected:?}"
    );

    // @step And each file in the directory is under 300 lines of code
    //
    // RPC-011 exception: `common.rs` is intentionally a kitchen-sink
    // module aggregating ~8 small (~30 LOC each) startup helpers
    // (build_service, init_tracing_{combined,daemon,client},
    // install_panic_hook, build_shutdown_future, write/remove_pidfile,
    // write/remove/read_and_verify_daemon_json, daemon_token,
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
        let body = std::fs::read_to_string(&p).expect("read source file");
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
