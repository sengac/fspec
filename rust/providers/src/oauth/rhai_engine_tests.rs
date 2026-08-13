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
fn engine_has_no_filesystem_or_process_or_env_access() {
    // @step And scripts cannot access the filesystem, spawn processes, or make unregistered network calls
    //
    // PROV-095 clarification: earlier this test used `print("hello")`
    // as a proxy for "no stdlib". That proxy is misleading — `print`
    // is a no-op language primitive and has nothing to do with
    // filesystem, process, or network access. After PROV-095 the
    // sandbox registers `CorePackage`, `BasicArrayPackage`,
    // `LogicPackage`, and `BasicMapPackage` so provider scripts can
    // iterate arrays (`for msg in request.messages { … }`), mutate
    // maps, concatenate strings, and do arithmetic — none of which
    // give them access to the host. The true sandbox guarantees are
    // that scripts cannot open files, spawn processes, or read env
    // vars: we verify each of those is unavailable by attempting to
    // call functions from namespaces that are **deliberately** never
    // registered.
    let engine = build_sandboxed_engine(Vec::new());

    // Filesystem: no `fs::` namespace is registered.
    let fs_result = engine.eval::<rhai::Dynamic>(r#"fs::read("/etc/passwd")"#);
    assert!(
        fs_result.is_err(),
        "fs::read should not be available in sandboxed engine"
    );

    // Process spawning: no `proc::` or `shell::` namespace is registered.
    let proc_result = engine.eval::<rhai::Dynamic>(r#"proc::spawn("/bin/sh", ["-c", "echo"])"#);
    assert!(
        proc_result.is_err(),
        "proc::spawn should not be available in sandboxed engine"
    );

    // Environment: no `env::` namespace is registered by default (the
    // PROV-086 `cred::` module is a separate, narrowly-scoped
    // namespace built via `build_provider_engine`, not this generic
    // factory).
    let env_result = engine.eval::<rhai::Dynamic>(r#"env::get("HOME")"#);
    assert!(
        env_result.is_err(),
        "env::get should not be available in sandboxed engine"
    );

    // File-eval: Rhai's historical `eval_file`-style helpers are not
    // registered — confirm by trying to call one.
    let eval_file_result = engine.eval::<rhai::Dynamic>(r#"eval_file("/etc/passwd")"#);
    assert!(
        eval_file_result.is_err(),
        "eval_file should not be available in sandboxed engine"
    );
}

#[test]
fn engine_has_iterator_and_array_packages_for_provider_scripts() {
    // PROV-095 regression guard: the sandboxed engine must support
    // `for msg in request.messages { … }` and common array methods,
    // otherwise custom provider scripts (e.g. `claude_rhai.rhai`) hit
    // "For loop expects iterable type" at runtime — exactly the error
    // that prompted this fix.
    let engine = build_sandboxed_engine(Vec::new());

    // Array iteration via for-in.
    let iter_script = r#"
        let xs = [1, 2, 3, 4];
        let total = 0;
        for x in xs { total += x; }
        total
    "#;
    let iter_result: i64 = engine
        .eval(iter_script)
        .expect("for-in over array must work in sandboxed engine");
    assert_eq!(iter_result, 10, "for-in summation must see all elements");

    // Array methods (.push, .len) used by claude_rhai.rhai.
    let push_script = r#"
        let parts = [];
        parts.push("a");
        parts.push("b");
        parts.len()
    "#;
    let push_result: i64 = engine
        .eval(push_script)
        .expect("Array.push / Array.len must be registered");
    assert_eq!(push_result, 2);

    // Map access (.contains, .len) used by scripts that inspect
    // `request.tools`.
    let map_script = r#"
        let m = #{ a: 1, b: 2 };
        m.len()
    "#;
    let map_len: i64 = engine.eval(map_script).expect("Map.len must be registered");
    assert_eq!(map_len, 2);

    // String equality + concatenation used by `type_of(...) == "array"`.
    let string_script = r#"
        let t = type_of([1, 2]);
        if t == "array" { "yes" } else { "no" }
    "#;
    let string_result: String = engine
        .eval(string_script)
        .expect("string equality + type_of must work");
    assert_eq!(string_result, "yes");
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
    custom_module.set_native_fn(
        "hello",
        || -> Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
            Ok(rhai::Dynamic::from("world".to_string()))
        },
    );

    let modules = vec![RhaiModule {
        name: "custom".to_string(),
        module: custom_module,
    }];

    let engine = build_sandboxed_engine(modules);
    let result: String = engine.eval(r#"custom::hello()"#).unwrap();
    assert_eq!(result, "world");
}
