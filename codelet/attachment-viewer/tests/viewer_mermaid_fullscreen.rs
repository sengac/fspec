#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/rust-viewer-mermaid-fullscreen.feature
//
// Fullscreen mermaid modal with Panzoom zoom/pan and SVG download. Tests assert
// on the EMITTED HTML/JS STRING produced by `viewer_template` (and the markdown
// pipeline for the server-render scenario). Each test maps 1:1 to a scenario;
// every Gherkin step has a matching `// @step` comment.

use std::fs;
use std::path::Path;

use codelet_attachment_viewer::markdown::{render_markdown, viewer_template};

#[test]
fn mermaid_initialized_theme_aware_with_run_on_load() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the HTML imports mermaid version 11 as an ESM module
    assert!(
        html.contains("mermaid@11/dist/mermaid.esm.min.mjs"),
        "mermaid v11 ESM import missing: {html}"
    );

    // @step And the HTML initializes mermaid with securityLevel loose and monospace font
    assert!(
        html.contains("securityLevel: 'loose'"),
        "securityLevel loose missing"
    );
    assert!(
        html.contains("fontFamily: 'monospace'"),
        "monospace font missing"
    );

    // @step And the HTML configures the flowchart curve as basis
    assert!(html.contains("curve: 'basis'"), "flowchart curve missing");

    // @step And the HTML derives the mermaid theme from the saved fspec-theme
    assert!(
        html.contains("localStorage.getItem('fspec-theme')"),
        "theme from fspec-theme missing"
    );
    assert!(
        html.contains("theme: isDark ? 'dark' : 'default'"),
        "theme derivation missing"
    );

    // @step And the HTML calls mermaid.run on load
    assert!(html.contains("mermaid.run()"), "mermaid.run() missing");
}

#[test]
fn page_loads_panzoom_cdn_script() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the HTML contains a script src for panzoom version 4.5.1
    assert!(
        html.contains("@panzoom/panzoom@4.5.1/dist/panzoom.min.js"),
        "panzoom 4.5.1 CDN missing: {html}"
    );
}

#[test]
fn each_diagram_gets_wrapper_with_fullscreen_and_download_buttons() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the JS selects pre.mermaid diagrams
    assert!(
        html.contains("querySelectorAll('pre.mermaid')"),
        "pre.mermaid selection missing"
    );

    // @step And the JS creates a div with class mermaid-wrapper
    assert!(
        html.contains("'mermaid-wrapper'"),
        "mermaid-wrapper class missing"
    );

    // @step And the JS injects a mermaid-fullscreen-btn button
    assert!(
        html.contains("mermaid-fullscreen-btn"),
        "fullscreen button missing"
    );

    // @step And the JS injects a mermaid-download-btn button
    assert!(
        html.contains("mermaid-download-btn"),
        "download button missing"
    );
}

#[test]
fn modal_opens_and_closes_on_esc_and_backdrop() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the JS defines an openMermaidModal function
    assert!(
        html.contains("function openMermaidModal"),
        "openMermaidModal missing"
    );

    // @step And the JS defines a closeMermaidModal function
    assert!(
        html.contains("function closeMermaidModal"),
        "closeMermaidModal missing"
    );

    // @step And the JS closes the modal on a backdrop click of the mermaid-modal element
    assert!(
        html.contains("e.target.id === 'mermaid-modal'"),
        "backdrop click guard missing"
    );

    // @step And the JS closes the modal when the Escape key is pressed
    assert!(
        html.contains("e.key === 'Escape'"),
        "Escape key handler missing"
    );
}

#[test]
fn zoom_clamped_with_reset_and_live_percentage() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the JS clamps the scale between 0.5 and 5
    assert!(
        html.contains("Math.max(0.5, Math.min(5,"),
        "scale clamp 0.5-5 missing"
    );

    // @step And the JS uses a 1.2 factor for zoom in and zoom out
    assert!(
        html.contains("ownScale * 1.2"),
        "zoom-in 1.2 factor missing"
    );
    assert!(
        html.contains("ownScale / 1.2"),
        "zoom-out 1.2 factor missing"
    );

    // @step And the JS provides a zoom-reset control
    assert!(
        html.contains("getElementById('zoom-reset')"),
        "zoom-reset control missing"
    );

    // @step And the JS updates the zoom-level element with a percentage
    assert!(
        html.contains("getElementById('zoom-level')"),
        "zoom-level element missing"
    );
    assert!(
        html.contains("percentage + '%'"),
        "percentage readout missing"
    );
}

#[test]
fn space_enters_pan_mode_and_svg_downloads_as_blob() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");

    // @step When I inspect the emitted HTML

    // @step Then the JS sets pan mode on Space key press
    assert!(html.contains("e.key === ' '"), "Space key guard missing");
    assert!(html.contains("isPanMode = true"), "isPanMode set missing");

    // @step And the JS builds a Blob of type image/svg+xml from the SVG markup for download
    assert!(
        html.contains("new Blob([svgData], { type: 'image/svg+xml' })"),
        "SVG Blob download missing"
    );
}

#[test]
fn existing_mermaid_server_render_works_and_files_stay_small() {
    // @step Given I render markdown containing a mermaid code fence
    let body = render_markdown("```mermaid\ngraph TD\n  A-->B\n```\n");
    let html = viewer_template("diagram.md", &body);

    // @step When I inspect the emitted HTML

    // @step Then the HTML still contains a pre.mermaid block for the diagram
    assert!(
        html.contains("<pre class=\"mermaid\">"),
        "pre.mermaid block missing: {html}"
    );

    // @step And every template source file remains under 300 lines
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/markdown/template");
    for name in [
        "mod.rs",
        "scripts.rs",
        "styles.rs",
        "modal_styles.rs",
        "mermaid_modal.rs",
        "mermaid_wheel.rs",
    ] {
        let path = dir.join(name);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let lines = content.lines().count();
        assert!(lines < 300, "{name} has {lines} lines (must be < 300)");
    }
}

#[test]
fn cursor_centered_wheel_zoom_is_clamped() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");
    // @step When I inspect the emitted HTML
    // @step Then the JS registers a wheel event listener on the modal body
    assert!(html.contains("addEventListener('wheel', handleModalWheel, { passive: false })"));
    // @step And the JS defines a handleModalWheel function that locks the zoom point at the cursor
    assert!(html.contains("function handleModalWheel(event)"));
    assert!(html.contains("lockedZoomPointX = event.clientX"));
    assert!(html.contains("Math.pow(2, zoomDelta)"));
    // @step And the wheel zoom clamps the new scale between 0.5 and 5
    assert!(html.contains("newScale = Math.max(0.5, Math.min(5, newScale))"));
}

#[test]
fn horizontal_scroll_pans_in_zoom_mode() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");
    // @step When I inspect the emitted HTML
    // @step Then the JS pans horizontally by deltaX divided by the current scale when not zooming
    assert!(html.contains("else if (Math.abs(deltaX) > 0)"));
    assert!(html.contains("ownPanX - deltaX / ownScale"));
}

#[test]
fn mode_indicator_fades_after_inactivity() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");
    // @step When I inspect the emitted HTML
    // @step Then the JS defines a showModeIndicator function that fades the indicator after a timeout
    assert!(html.contains("function showModeIndicator()"));
    assert!(html.contains("modeIndicatorTimeout = setTimeout("));
    assert!(html.contains("indicator.style.opacity = '0.5'"));
}

#[test]
fn space_toggles_pan_mode_class_on_container() {
    // @step Given I render a viewer page for some content
    let html = viewer_template("doc.md", "<p>hi</p>");
    // @step When I inspect the emitted HTML
    // @step Then the JS adds the pan-mode class to the diagram container in pan mode
    assert!(html.contains("container.classList.add('pan-mode')"));
    // @step And the JS removes the pan-mode class when leaving pan mode
    assert!(html.contains("container.classList.remove('pan-mode')"));
}
