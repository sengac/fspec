#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/rust-viewer-prism-theme-fonts.feature
//
// Client-side viewer parity: Prism syntax highlighting, copy button, language
// badge, theme toggle and font-size controls. Tests assert on the EMITTED HTML
// STRING produced by `viewer_template`. Each test maps 1:1 to a scenario; every
// Gherkin step has a matching `// @step` comment.

use codelet_attachment_viewer::markdown::viewer_template;

#[test]
fn page_includes_prism_scripts_and_theme_stylesheet() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML
    // (the rendered string is inspected by the assertions below)

    // @step Then the HTML contains the Prism core script "prism/1.29.0/components/prism-core.min.js"
    assert!(
        html.contains("prism/1.29.0/components/prism-core.min.js"),
        "prism core missing: {html}"
    );

    // @step And the HTML contains the Prism autoloader plugin script
    assert!(
        html.contains("prism/1.29.0/plugins/autoloader/prism-autoloader.min.js"),
        "autoloader missing"
    );

    // @step And the HTML contains a stylesheet link to "prism-vsc-dark-plus.min.css"
    assert!(
        html.contains("prism-vsc-dark-plus.min.css"),
        "prism theme css missing"
    );
}

#[test]
fn page_highlights_code_blocks_with_alias_map() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the HTML contains a "DOMContentLoaded" handler
    assert!(html.contains("DOMContentLoaded"), "no DOMContentLoaded");

    // @step And the HTML contains the alias entry "sh: 'bash'"
    assert!(html.contains("sh: 'bash'"), "sh alias missing");

    // @step And the HTML contains the alias entry "ts: 'typescript'"
    assert!(html.contains("ts: 'typescript'"), "ts alias missing");

    // @step And the HTML contains a "Prism.highlightAll()" call
    assert!(
        html.contains("Prism.highlightAll()"),
        "highlightAll missing"
    );
}

#[test]
fn page_adds_copy_buttons_and_language_badges() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the HTML contains copy-button creation markup
    assert!(html.contains("copy-button"), "copy-button missing");

    // @step And the HTML contains the "Copied!" feedback text
    assert!(html.contains("Copied!"), "Copied! text missing");

    // @step And the HTML contains a language-badge element
    assert!(html.contains("language-badge"), "language-badge missing");
}

#[test]
fn page_provides_theme_toggle_persisted_to_local_storage() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the HTML contains a theme-toggle button with id "theme-toggle"
    assert!(
        html.contains("id=\"theme-toggle\""),
        "theme-toggle button missing"
    );

    // @step And the HTML reads and writes the localStorage key "fspec-theme"
    assert!(
        html.contains("localStorage.getItem('fspec-theme')"),
        "fspec-theme read missing"
    );
    assert!(
        html.contains("localStorage.setItem('fspec-theme'"),
        "fspec-theme write missing"
    );
}

#[test]
fn page_provides_clamped_font_size_controls() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the HTML contains font-size controls
    assert!(
        html.contains("font-size-controls"),
        "font-size-controls missing"
    );

    // @step And the HTML reads and writes the localStorage key "fspec-base-font-size"
    assert!(
        html.contains("localStorage.getItem('fspec-base-font-size')"),
        "font-size read missing"
    );
    assert!(
        html.contains("localStorage.setItem('fspec-base-font-size'"),
        "font-size write missing"
    );

    // @step And the HTML clamps the font size to the bounds 10 and 24
    assert!(html.contains("MIN_FONT_SIZE = 10"), "min bound 10 missing");
    assert!(html.contains("MAX_FONT_SIZE = 24"), "max bound 24 missing");

    // @step And the HTML disables the controls at the bounds
    assert!(
        html.contains(".disabled = currentFontSize <= MIN_FONT_SIZE"),
        "decrease disable missing"
    );
    assert!(
        html.contains(".disabled = currentFontSize >= MAX_FONT_SIZE"),
        "increase disable missing"
    );
}

#[test]
fn stylesheet_defines_dark_and_light_theme_variables() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the stylesheet defines dark variables under ":root"
    assert!(html.contains(":root {"), "dark :root vars missing");
    assert!(html.contains("--bg-color: #1e1e1e"), "dark bg var missing");

    // @step And the stylesheet defines light variables under ":root.light-theme"
    assert!(
        html.contains(":root.light-theme {"),
        "light theme vars missing"
    );
    assert!(html.contains("--bg-color: #ffffff"), "light bg var missing");
}

#[test]
fn existing_mermaid_script_and_content_wrapper_preserved() {
    // @step Given I render a viewer page with title "<x>.md" and body "<h1>Hi</h1>"
    let html = viewer_template("<x>.md", "<h1>Hi</h1>");

    // @step When I inspect the emitted HTML

    // @step Then the HTML still embeds the mermaid module script
    assert!(
        html.contains("mermaid.esm.min.mjs"),
        "mermaid module script missing"
    );
    assert!(html.contains("mermaid.initialize("), "mermaid init missing");

    // @step And the HTML still contains the ".markdown-content" wrapper around the body
    assert!(
        html.contains("class=\"markdown-content\""),
        "markdown-content wrapper missing"
    );
    assert!(html.contains("<h1>Hi</h1>"), "body content missing");

    // @step And the document title is escaped to "&lt;x&gt;.md"
    assert!(
        html.contains("<title>&lt;x&gt;.md</title>"),
        "escaped title missing: {html}"
    );
}

#[test]
fn mermaid_theme_follows_saved_viewer_theme() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the mermaid script reads the localStorage key "fspec-theme"
    assert!(
        html.contains("localStorage.getItem('fspec-theme')"),
        "fspec-theme read missing in mermaid init: {html}"
    );

    // @step And the mermaid init selects theme "'dark'" or "'default'" from the saved theme
    assert!(
        html.contains("theme: isDark ? 'dark' : 'default'"),
        "mermaid theme selection missing: {html}"
    );
}

#[test]
fn prism_language_aliases_map_shorthand_languages() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the alias map contains "shell: 'bash'" and "console: 'bash'"
    assert!(html.contains("shell: 'bash'"), "shell alias missing");
    assert!(html.contains("console: 'bash'"), "console alias missing");

    // @step And the alias map contains "js: 'javascript'", "py: 'python'", "rb: 'ruby'" and "yml: 'yaml'"
    assert!(html.contains("js: 'javascript'"), "js alias missing");
    assert!(html.contains("py: 'python'"), "py alias missing");
    assert!(html.contains("rb: 'ruby'"), "rb alias missing");
    assert!(html.contains("yml: 'yaml'"), "yml alias missing");

    // @step And the language resolver maps "text" to "plaintext"
    assert!(
        html.contains("language === 'text') return 'plaintext'"),
        "text->plaintext special case missing: {html}"
    );
}
