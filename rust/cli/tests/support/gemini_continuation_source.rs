pub fn gemini_continuation_source() -> &'static str {
    concat!(
        include_str!("../../src/interactive/gemini_continuation.rs"),
        "\n",
        include_str!("../../src/interactive/gemini_continuation/continuation_loop.rs"),
        "\n",
        include_str!("../../src/interactive/gemini_continuation/state.rs"),
    )
}
