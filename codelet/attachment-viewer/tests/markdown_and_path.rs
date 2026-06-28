#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/rust-attachment-viewer-server.feature
//
// Unit tests for the markdown renderer and the path-validation guard, exercised
// directly (no HTTP). Supports the mermaid / code-block / escaping rules and the
// directory-traversal containment rule from the feature's business rules.

use std::path::Path;

use codelet_attachment_viewer::markdown::{html_escape, render_markdown, viewer_template};
use codelet_attachment_viewer::validate_path;

#[test]
fn render_markdown_wraps_mermaid_blocks() {
    let html = render_markdown("```mermaid\ngraph TD\n  A-->B\n```\n");
    assert!(html.contains("<pre class=\"mermaid\">"), "got: {html}");
    assert!(html.contains("graph TD"), "code content missing");
    assert!(
        !html.contains("<code>"),
        "mermaid must not be wrapped in <code>"
    );
}

#[test]
fn render_markdown_wraps_regular_code_blocks_with_language() {
    let html = render_markdown("```rust\nlet x = 1;\n```\n");
    assert!(
        html.contains("<pre class=\"code-block\" data-language=\"rust\">"),
        "got: {html}"
    );
    assert!(html.contains("<code>"), "code tag missing");
    assert!(html.contains("let x = 1;"), "code content missing");
}

#[test]
fn render_markdown_escapes_code_content() {
    let html = render_markdown("```html\n<script>alert(1)</script>\n```\n");
    assert!(html.contains("&lt;script&gt;"), "code not escaped: {html}");
    assert!(!html.contains("<script>"), "raw script leaked");
}

#[test]
fn render_markdown_supports_gfm_tables() {
    let html = render_markdown("| a | b |\n|---|---|\n| 1 | 2 |\n");
    assert!(html.contains("<table>"), "table not rendered: {html}");
}

#[test]
fn html_escape_escapes_all_special_chars() {
    assert_eq!(
        html_escape("<a href=\"x\">&'</a>"),
        "&lt;a href=&quot;x&quot;&gt;&amp;&#039;&lt;/a&gt;"
    );
}

#[test]
fn viewer_template_embeds_escaped_title_and_mermaid_script() {
    let html = viewer_template("design.md", "<h1>Hi</h1>");
    assert!(html.starts_with("<!DOCTYPE html>"), "no doctype");
    assert!(html.contains("<title>design.md</title>"), "title missing");
    assert!(html.contains("mermaid.initialize("));
    assert!(html.contains("startOnLoad: true"));
    assert!(html.contains("class=\"markdown-content\""));
    assert!(html.contains("<h1>Hi</h1>"), "content not embedded");
}

#[test]
fn viewer_template_escapes_title() {
    let html = viewer_template("<x>.md", "");
    assert!(html.contains("<title>&lt;x&gt;.md</title>"), "got: {html}");
}

#[test]
fn validate_path_resolves_relative_under_cwd() {
    let cwd = Path::new("/home/project");
    let got = validate_path(cwd, "spec/attachments/a.md");
    assert_eq!(
        got.as_deref(),
        Some(Path::new("/home/project/spec/attachments/a.md"))
    );
}

#[test]
fn validate_path_rejects_parent_traversal() {
    let cwd = Path::new("/home/project");
    assert!(validate_path(cwd, "../../etc/passwd").is_none());
}

#[test]
fn validate_path_rejects_absolute_outside_cwd() {
    let cwd = Path::new("/home/project");
    assert!(validate_path(cwd, "/etc/passwd").is_none());
}

#[test]
fn validate_path_allows_absolute_inside_cwd() {
    let cwd = Path::new("/home/project");
    let got = validate_path(cwd, "/home/project/file.md");
    assert_eq!(got.as_deref(), Some(Path::new("/home/project/file.md")));
}
