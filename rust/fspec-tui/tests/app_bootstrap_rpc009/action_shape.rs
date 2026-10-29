/// Scenario: Action enum gains seven new variants while existing variants are preserved
#[test]
fn action_enum_gains_seven_new_variants_while_existing_variants_are_preserved() {
    // @step Given the Action enum in rust/fspec-tui/src/components/mod.rs
    let src = codelet_test_helpers::source_composition::read_source(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/components/mod.rs"),
    )
    .expect("read composed components/mod.rs");
    // @step Then it contains the existing variants Quit, Redraw, Custom(String)
    assert!(src.contains("Quit,"));
    assert!(src.contains("Redraw,"));
    assert!(src.contains("Custom(String)"));
    // @step And it additionally contains LoadWorkUnits
    assert!(src.contains("LoadWorkUnits,"));
    // @step And it additionally contains WorkUnitsLoaded(Vec<WorkUnitInfo>)
    assert!(src.contains("WorkUnitsLoaded(Vec<codelet_rpc_types::WorkUnitInfo>)"));
    // @step And it additionally contains SessionCreated(SessionId)
    assert!(src.contains("SessionCreated(codelet_rpc_types::SessionId)"));
    // @step And it additionally contains ChunkReceived(SessionId, StreamChunk)
    assert!(
        src.contains("ChunkReceived(codelet_rpc_types::SessionId, codelet_rpc_types::StreamChunk)")
    );
    // @step And it additionally contains InputSubmitted(String)
    assert!(src.contains("InputSubmitted(String)"));
    // @step And it additionally contains Interrupt
    assert!(src.contains("Interrupt,"));
    // @step And it additionally contains FocusNext
    assert!(src.contains("FocusNext,"));
    // @step And the enum still derives Clone, Debug
    assert!(src.contains("#[derive(Debug, Clone)]"));
    // @step And the enum drops PartialEq and Eq because StreamChunk does not derive PartialEq
    // (asserted by the absence of PartialEq, Eq in the derive line — exactly `#[derive(Debug, Clone)]`)
}
