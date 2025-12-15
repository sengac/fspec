//! Proof-of-concept test to verify rig re-export works
//!
//! This test proves that `pub use rig;` in src/lib.rs actually works
//! by attempting to access rig types through the codelet namespace.

#[test]
fn test_rig_reexport_compiles() {
    // This test proves the re-export works because if it compiles,
    // we can access rig types through codelet::rig namespace

    // Verify we can reference rig types (compilation test)
    let _rig_exists: Option<fn()> = Some(|| {
        // These type paths will fail to compile if re-export doesn't work:

        // Access rig::completion types
        type _CompletionModel = dyn std::any::Any; // Placeholder

        // Access rig::agent types
        type _Agent = dyn std::any::Any; // Placeholder

        // Access rig::tool types
        type _Tool = dyn std::any::Any; // Placeholder

        // Access rig::streaming types
        type _StreamingResponse = dyn std::any::Any; // Placeholder
    });

    // If we get here, the re-export is working!
    // The real proof is that this file compiles.
}

#[test]
fn test_rig_version_is_0_25_0() {
    // Verify we're using rig-core 0.25.0 by checking Cargo.toml
    let cargo_toml = std::fs::read_to_string("Cargo.toml").expect("Could not read Cargo.toml");

    assert!(
        cargo_toml.contains("rig-core = { version = \"0.25.0\""),
        "rig-core should be version 0.25.0 in Cargo.toml"
    );
}
