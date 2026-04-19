//! Tests for Sandboxed Rhai Engine (PROV-060)
//!
//! Feature: spec/features/shared-oauth-building-blocks.feature
//! Scenario: Sandboxed Rhai engine enforces safety limits

use crate::oauth::engine::{build_sandboxed_engine, MAX_OPERATIONS};

// @step Given a Rhai engine created via build_sandboxed_engine with registered modules
// @step When a script exceeds the operation limit of 50000 operations
// @step Then the engine terminates the script with an error
// @step And scripts cannot access the filesystem, spawn processes, or make unregistered network calls

#[test]
fn engine_terminates_script_exceeding_operation_limit() {
    // @step Given a Rhai engine created via build_sandboxed_engine with registered modules
    let engine = build_sandboxed_engine(Vec::new());

    // @step When a script exceeds the operation limit of 50000 operations
    // A tight infinite loop will exceed 50,000 ops quickly
    let script = r#"
        let x = 0;
        loop {
            x += 1;
        }
    "#;

    // @step Then the engine terminates the script with an error
    let result = engine.eval::<rhai::Dynamic>(script);
    assert!(result.is_err(), "Script should have been terminated");
    let err_msg = format!("{}", result.unwrap_err());
    // Rhai reports exceeding max operations
    assert!(
        err_msg.contains("operations") || err_msg.contains("limit") || err_msg.contains("exceed"),
        "Error should mention operations limit, got: {err_msg}"
    );
}

#[test]
fn engine_has_no_std_library_filesystem_access() {
    // @step And scripts cannot access the filesystem, spawn processes, or make unregistered network calls
    let engine = build_sandboxed_engine(Vec::new());

    // Attempting to use standard library functions should fail
    // since Engine::new_raw() provides no stdlib
    let result = engine.eval::<rhai::Dynamic>(r#"print("hello")"#);
    assert!(
        result.is_err(),
        "print should not be available in raw engine"
    );
}

#[test]
fn engine_operation_limit_is_50000() {
    assert_eq!(MAX_OPERATIONS, 50_000);
}

#[test]
fn engine_enforces_call_depth_limit() {
    let engine = build_sandboxed_engine(Vec::new());

    // Deep recursion should hit the call depth limit
    let script = r#"
        fn recurse(n) {
            if n > 0 { recurse(n - 1) }
        }
        recurse(100)
    "#;

    let result = engine.eval::<rhai::Dynamic>(script);
    assert!(
        result.is_err(),
        "Deep recursion should be terminated by call depth limit"
    );
}

#[test]
fn engine_accepts_extensible_module_list() {
    use crate::oauth::engine::RhaiModule;
    use rhai::Module;

    // @step And the engine factory accepts an extensible module list for future modules
    let mut custom_module = Module::new();
    custom_module.set_native_fn("hello", || -> Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
        Ok(rhai::Dynamic::from("world".to_string()))
    });

    let modules = vec![RhaiModule {
        name: "custom".to_string(),
        module: custom_module,
    }];

    let engine = build_sandboxed_engine(modules);
    let result: String = engine.eval(r#"custom::hello()"#).unwrap();
    assert_eq!(result, "world");
}
