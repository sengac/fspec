#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/markdown-footnote-option-parity.feature
//
// Integration tests verifying the Rust markdown renderer aligns with marked's
// lack of footnote support: footnote syntax must NOT produce footnote-specific
// markup, while tables and strikethrough continue to render unchanged.

use codelet_attachment_viewer::markdown::render_markdown;

#[test]
fn footnote_syntax_does_not_produce_footnote_markup() {
    // @step Given markdown text "Text[^1]" with a definition line "[^1]: a note"
    let markdown = "Text[^1]\n\n[^1]: a note";

    // @step When the markdown is rendered to HTML
    let html = render_markdown(markdown);

    // @step Then the output does not contain a footnote reference element
    // pulldown-cmark's ENABLE_FOOTNOTES emits a <sup class="footnote-reference">
    // reference and "fr-"/"fn-" anchor ids; marked never emits any of these.
    assert!(
        !html.contains("footnote-reference"),
        "footnote reference markup leaked: {html}"
    );
    assert!(
        !html.contains("<sup"),
        "footnote <sup> element leaked: {html}"
    );

    // @step And the output does not contain a footnote definition section
    assert!(
        !html.contains("footnote-definition"),
        "footnote definition section leaked: {html}"
    );
}

#[test]
fn tables_still_render_after_footnotes_are_disabled() {
    // @step Given markdown containing a GFM table
    let markdown = "| a | b |\n|---|---|\n| 1 | 2 |\n";

    // @step When the markdown is rendered to HTML
    let html = render_markdown(markdown);

    // @step Then the output contains a "<table>" element
    assert!(html.contains("<table>"), "table not rendered: {html}");
}

#[test]
fn strikethrough_still_renders_after_footnotes_are_disabled() {
    // @step Given markdown text "~~gone~~"
    let markdown = "~~gone~~";

    // @step When the markdown is rendered to HTML
    let html = render_markdown(markdown);

    // @step Then the output contains a "<del>" element
    assert!(html.contains("<del>"), "strikethrough not rendered: {html}");
}
