#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/markdown-heading-anchors.feature
//
// Heading anchor IDs + GFM render-option parity for the Rust markdown viewer.
// Each test maps 1:1 to a scenario in the feature file; every Gherkin step has a
// matching `// @step` comment.

use codelet_attachment_viewer::markdown::render_markdown;

#[test]
fn render_heading_with_single_word_title() {
    // @step Given I have markdown content with a heading "## Summary"
    let markdown = "## Summary\n\nSome body.";

    // @step When I render the markdown to HTML
    let html = render_markdown(markdown);

    // @step Then the rendered HTML should contain "<h2 id=\"summary\">Summary</h2>"
    assert!(
        html.contains("<h2 id=\"summary\">Summary</h2>"),
        "got: {html}"
    );
}

#[test]
fn render_heading_with_multi_word_title() {
    // @step Given I have markdown content with a heading "## Domain-to-Tag Mapping Rules"
    let markdown = "## Domain-to-Tag Mapping Rules\n\nMapping rules content.";

    // @step When I render the markdown to HTML
    let html = render_markdown(markdown);

    // @step Then the rendered HTML should contain "<h2 id=\"domain-to-tag-mapping-rules\">Domain-to-Tag Mapping Rules</h2>"
    assert!(
        html.contains("<h2 id=\"domain-to-tag-mapping-rules\">Domain-to-Tag Mapping Rules</h2>"),
        "got: {html}"
    );
}

#[test]
fn strip_special_characters_from_slug() {
    // @step Given I have markdown content with a heading "## What's New?"
    let markdown = "## What's New?\n\nNew features listed here.";

    // @step When I render the markdown to HTML
    let html = render_markdown(markdown);

    // @step Then the rendered HTML should contain "id=\"whats-new\""
    assert!(html.contains("id=\"whats-new\""), "got: {html}");

    // @step And the rendered HTML should not contain "id=\"what's-new?\""
    assert!(!html.contains("id=\"what's-new?\""), "got: {html}");
}

#[test]
fn deduplicate_repeated_heading_slugs() {
    // @step Given I have markdown content with three headings all titled "Summary"
    let markdown = "## Summary\n\nFirst.\n\n## Summary\n\nSecond.\n\n## Summary\n\nThird.";

    // @step When I render the markdown to HTML
    let html = render_markdown(markdown);

    // @step Then the rendered HTML should contain "id=\"summary\""
    assert!(html.contains("id=\"summary\""), "got: {html}");

    // @step And the rendered HTML should contain "id=\"summary-1\""
    assert!(html.contains("id=\"summary-1\""), "got: {html}");

    // @step And the rendered HTML should contain "id=\"summary-2\""
    assert!(html.contains("id=\"summary-2\""), "got: {html}");
}

#[test]
fn anchor_link_round_trips_to_heading_id() {
    // @step Given I have markdown with a link "[Jump to summary](#summary)" above a heading "## Summary"
    let markdown = "[Jump to summary](#summary)\n\n## Summary\n\nSummary content here.";

    // @step When I render the markdown to HTML
    let html = render_markdown(markdown);

    // @step Then the rendered HTML should contain "href=\"#summary\""
    assert!(html.contains("href=\"#summary\""), "got: {html}");

    // @step And the rendered HTML should contain "id=\"summary\""
    assert!(html.contains("id=\"summary\""), "got: {html}");
}

#[test]
fn soft_line_break_renders_as_hard_break() {
    // @step Given I have a paragraph with a soft line break between "line1" and "line2"
    let markdown = "line1\nline2";

    // @step When I render the markdown to HTML
    let html = render_markdown(markdown);

    // @step Then the rendered HTML should contain "line1<br>" followed by "line2"
    assert!(html.contains("line1<br>"), "got: {html}");
    let br_pos = html.find("line1<br>").expect("line1<br> present");
    let l2_pos = html.find("line2").expect("line2 present");
    assert!(l2_pos > br_pos, "line2 must follow line1<br>: {html}");
}

#[test]
fn smart_punctuation_is_not_applied() {
    // @step Given I have markdown text containing a straight apostrophe in "it's"
    let markdown = "it's fine";

    // @step When I render the markdown to HTML
    let html = render_markdown(markdown);

    // @step Then the rendered HTML should contain a straight apostrophe in "it's"
    // pulldown-cmark renders body-text apostrophes literally as a straight ' (it
    // does NOT emit the &#39; entity and does NOT apply smart punctuation).
    assert!(
        html.contains("it's"),
        "expected straight apostrophe: {html}"
    );
    assert!(
        !html.contains("it&#39;s"),
        "unexpected &#39; entity form: {html}"
    );

    // @step And the rendered HTML should not contain a curly apostrophe
    assert!(
        !html.contains('\u{2019}'),
        "curly apostrophe leaked: {html}"
    );
}

#[test]
fn code_and_mermaid_blocks_unaffected_by_heading_ids() {
    // @step Given I have markdown with a heading, a mermaid block, and a python code block
    let markdown = "# Diagram\n\n```mermaid\ngraph TD\n  A-->B\n```\n\n```python\ndef hello():\n    print(\"hi\")\n```\n";

    // @step When I render the markdown to HTML
    let html = render_markdown(markdown);

    // @step Then the rendered HTML should contain "<pre class=\"mermaid\">"
    assert!(html.contains("<pre class=\"mermaid\">"), "got: {html}");

    // @step And the rendered HTML should contain "data-language=\"python\""
    assert!(html.contains("data-language=\"python\""), "got: {html}");

    // @step And the rendered HTML should contain a heading id for the heading
    assert!(html.contains("id=\"diagram\""), "got: {html}");
}
