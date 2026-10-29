/// Scenario: codelet_common::file_lock is lifted out of schedule_handler.rs
#[test]
fn codelet_common_file_lock_is_lifted_out_of_schedule_handler() {
    // @step Given the codelet workspace after RPC-017 lands
    let file_lock = workspace_root()
        .join("common")
        .join("src")
        .join("file_lock.rs");
    let common_lib = workspace_root().join("common").join("src").join("lib.rs");
    let schedule_handler = workspace_root()
        .join("napi")
        .join("src")
        .join("schedule_handler.rs");

    // @step Then the file rust/common/src/file_lock.rs exists
    assert!(
        file_lock.exists(),
        "rust/common/src/file_lock.rs must exist after RPC-017"
    );

    // @step And rust/common/src/file_lock.rs contains the substring "pub fn with_file_lock"
    let file_lock_body = read_raw(&file_lock);
    assert!(
        file_lock_body.contains("pub fn with_file_lock"),
        "rust/common/src/file_lock.rs must export `pub fn with_file_lock`"
    );

    // @step And rust/common/src/lib.rs contains the substring "pub mod file_lock"
    let lib_body = read_raw(&common_lib);
    assert!(
        lib_body.contains("pub mod file_lock"),
        "rust/common/src/lib.rs must declare `pub mod file_lock`"
    );

    // @step And rust/napi/src/schedule_handler.rs contains the substring "codelet_common::file_lock"
    let sched_body = read_raw(&schedule_handler);
    let sched_stripped = common::strip_rust_comments(&sched_body);
    assert!(
        sched_stripped.contains("codelet_common::file_lock"),
        "schedule_handler.rs must depend on codelet_common::file_lock"
    );

    // @step And rust/napi/src/schedule_handler.rs does NOT contain the substring "fn acquire_lock"
    assert!(
        !sched_stripped.contains("fn acquire_lock"),
        "schedule_handler.rs must no longer inline `fn acquire_lock`"
    );

    // @step And rust/napi/src/schedule_handler.rs does NOT contain the substring "fn release_lock"
    assert!(
        !sched_stripped.contains("fn release_lock"),
        "schedule_handler.rs must no longer inline `fn release_lock`"
    );
}

/// Scenario: codelet_core::work_units_write module exists and stays under 300 LoC
#[test]
fn codelet_core_work_units_write_module_exists_and_stays_under_300_loc() {
    // @step Given the codelet workspace after RPC-017 lands
    let wuw = workspace_root()
        .join("core")
        .join("src")
        .join("work_units_write.rs");
    let core_lib = workspace_root().join("core").join("src").join("lib.rs");

    // @step Then the file rust/core/src/work_units_write.rs exists
    assert!(
        wuw.exists(),
        "rust/core/src/work_units_write.rs must exist after RPC-017"
    );

    // @step And rust/core/src/work_units_write.rs has fewer than 300 lines
    let lines = count_lines_path(&wuw);
    assert!(
        lines < 300,
        "rust/core/src/work_units_write.rs must stay < 300 LoC (currently {lines})"
    );

    let body = read_raw(&wuw);

    // @step And rust/core/src/work_units_write.rs contains the substring "pub fn move_work_unit"
    assert!(
        body.contains("pub fn move_work_unit"),
        "work_units_write.rs must export `pub fn move_work_unit`"
    );

    // @step And rust/core/src/work_units_write.rs contains the substring "pub enum Direction"
    assert!(
        body.contains("pub enum Direction"),
        "work_units_write.rs must export `pub enum Direction`"
    );

    // @step And rust/core/src/lib.rs contains the substring "pub mod work_units_write"
    let core_lib_body = read_raw(&core_lib);
    assert!(
        core_lib_body.contains("pub mod work_units_write"),
        "rust/core/src/lib.rs must declare `pub mod work_units_write`"
    );

    // @step And rust/core/src/work_units.rs (the read-side module) still exports `pub fn read_snapshot` and `pub struct WorkUnitsWatcher`
    let work_units_rs = workspace_root()
        .join("core")
        .join("src")
        .join("work_units.rs");
    let work_units_body = read_raw(&work_units_rs);
    assert!(
        work_units_body.contains("pub fn read_snapshot"),
        "work_units.rs (read-side) must still export read_snapshot"
    );
    assert!(
        work_units_body.contains("pub struct WorkUnitsWatcher"),
        "work_units.rs (read-side) must still export WorkUnitsWatcher"
    );
}
