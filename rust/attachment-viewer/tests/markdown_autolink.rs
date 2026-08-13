#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/markdown-autolink-literals.feature
//
// Integration tests verifying bare-URL and email autolink parity with marked:
// literal http/https URLs and bare emails become anchors, while existing links,
// inline code, headings, and code/mermaid blocks are unaffected.

use codelet_attachment_viewer::markdown::render_markdown;

#[test]
fn autolink_a_bare_https_url_in_plain_text() {
    // @step Given markdown text "See https://example.com for details"
    let markdown = "See https://example.com for details";

    // @step When the markdown is rendered to HTML
    let html = render_markdown(markdown);

    // @step Then the output contains "<a href=\"https://example.com\">https://example.com</a>"
    assert!(
        html.contains("<a href=\"https://example.com\">https://example.com</a>"),
        "url not autolinked: {html}"
    );
}

#[test]
fn autolink_a_bare_email_address_as_a_mailto_link() {
    // @step Given markdown text "Email a@b.com please"
    let markdown = "Email a@b.com please";

    // @step When the markdown is rendered to HTML
    let html = render_markdown(markdown);

    // @step Then the output contains "<a href=\"mailto:a@b.com\">a@b.com</a>"
    assert!(
        html.contains("<a href=\"mailto:a@b.com\">a@b.com</a>"),
        "email not autolinked: {html}"
    );
}

#[test]
fn an_existing_markdown_link_is_not_double_linked() {
    // @step Given markdown text "[label](https://example.com)"
    let markdown = "[label](https://example.com)";

    // @step When the markdown is rendered to HTML
    let html = render_markdown(markdown);

    // @step Then the output contains a single anchor with visible text "label"
    assert_eq!(
        html.matches("<a ").count(),
        1,
        "expected one anchor: {html}"
    );
    assert!(
        html.contains(">label</a>"),
        "anchor text not 'label': {html}"
    );

    // @step And the output does not wrap the destination URL in a nested anchor
    assert!(
        !html.contains(">https://example.com</a>"),
        "destination URL was double-linked: {html}"
    );
}

#[test]
fn trailing_sentence_punctuation_is_excluded_from_the_autolink() {
    // @step Given markdown text "Visit https://example.com."
    let markdown = "Visit https://example.com.";

    // @step When the markdown is rendered to HTML
    let html = render_markdown(markdown);

    // @step Then the output contains an anchor with href "https://example.com"
    assert!(
        html.contains("<a href=\"https://example.com\">https://example.com</a>"),
        "anchor href incorrect: {html}"
    );

    // @step And the trailing period is left outside the anchor
    assert!(
        html.contains("</a>."),
        "trailing period not left outside anchor: {html}"
    );
    assert!(
        !html.contains("example.com.</a>"),
        "trailing period leaked inside anchor: {html}"
    );
}

#[test]
fn a_url_inside_an_inline_code_span_is_not_autolinked() {
    // @step Given markdown text "Run `https://x.com` now"
    let markdown = "Run `https://x.com` now";

    // @step When the markdown is rendered to HTML
    let html = render_markdown(markdown);

    // @step Then the URL stays inside a code element
    assert!(
        html.contains("<code>https://x.com</code>"),
        "url not preserved in code element: {html}"
    );

    // @step And the output does not contain an anchor for that URL
    assert!(
        !html.contains("<a "),
        "code-span url was autolinked: {html}"
    );
}

#[test]
fn both_http_and_https_schemes_are_autolinked() {
    // @step Given markdown text "http://example.com and https://example.com"
    let markdown = "http://example.com and https://example.com";

    // @step When the markdown is rendered to HTML
    let html = render_markdown(markdown);

    // @step Then the output contains an anchor with href "http://example.com"
    assert!(
        html.contains("<a href=\"http://example.com\">http://example.com</a>"),
        "http url not autolinked: {html}"
    );

    // @step And the output contains an anchor with href "https://example.com"
    assert!(
        html.contains("<a href=\"https://example.com\">https://example.com</a>"),
        "https url not autolinked: {html}"
    );
}

#[test]
fn existing_rendering_is_unaffected_by_autolinking() {
    // @step Given markdown containing a mermaid block and a python code block and a heading
    let markdown = "# Title\n\n```mermaid\ngraph TD\n  A-->B\n```\n\n```python\nx = 1\n```\n";

    // @step When the markdown is rendered to HTML
    let html = render_markdown(markdown);

    // @step Then the mermaid block still renders as "<pre class=\"mermaid\">"
    assert!(
        html.contains("<pre class=\"mermaid\">"),
        "mermaid block missing: {html}"
    );

    // @step And the python code block still renders with data-language "python"
    assert!(
        html.contains("data-language=\"python\""),
        "python code block missing: {html}"
    );

    // @step And the heading still renders a slug id
    assert!(
        html.contains("<h1 id=\"title\">"),
        "heading slug id missing: {html}"
    );
}

#[test]
fn a_url_preceded_by_non_ascii_text_is_autolinked_without_panicking() {
    // @step Given markdown text "café https://example.com"
    let markdown = "café https://example.com";

    // @step When the markdown is rendered to HTML
    let html = render_markdown(markdown);

    // @step Then the output contains "<a href=\"https://example.com\">https://example.com</a>"
    assert!(
        html.contains("<a href=\"https://example.com\">https://example.com</a>"),
        "non-ascii-prefixed url not autolinked: {html}"
    );

    // @step And rendering does not panic
    // (reaching this assertion proves render_markdown returned without panicking)
    assert!(html.contains("café"), "non-ascii text dropped: {html}");
}
