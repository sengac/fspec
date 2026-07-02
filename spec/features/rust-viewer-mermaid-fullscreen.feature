@done
@rust
@attachment-viewer
@viewer
@RPC-378
Feature: Fullscreen mermaid modal with Panzoom zoom/pan and SVG download in the Rust markdown viewer
  """
  The viewer_template (codelet/attachment-viewer src/markdown/template) emits all interactivity as static server-rendered HTML/JS strings; tests assert on the emitted string, not browser execution. Mermaid ESM v11 + Panzoom v4.5.1 are loaded from CDN. The fullscreen modal JS is split into a mermaid_modal submodule and modal CSS into a modal_styles submodule so every file stays under 300 lines. The public viewer_template(title, content_html) signature is unchanged.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Mermaid is loaded as ESM v11 and initialized with securityLevel 'loose', monospace font, flowchart options, a theme derived from the saved fspec-theme, and mermaid.run() on load
  #   2. The page loads the Panzoom v4.5.1 CDN script
  #   3. The client JS wraps each pre.mermaid in a .mermaid-wrapper and adds Fullscreen and Download-SVG hover buttons
  #   4. The JS defines an open/close fullscreen modal flow that closes on ESC and backdrop click
  #   5. Zoom is clamped to 0.5x-5x with zoom-in x1.2, zoom-out divided by 1.2, reset, and a live percentage readout
  #   6. The JS implements Space-to-pan mode and serializes the SVG to a Blob for download
  #   7. Existing markdown/mermaid server-render scenarios still pass and every template file stays under 300 lines
  #
  # EXAMPLES:
  #   1. The emitted HTML imports mermaid@11 ESM and calls mermaid.initialize with securityLevel: 'loose', fontFamily: 'monospace', flowchart curve basis, theme from fspec-theme, then mermaid.run()
  #   2. The emitted HTML contains a script src for @panzoom/panzoom@4.5.1 panzoom.min.js
  #   3. The emitted JS selects pre.mermaid, creates a div.mermaid-wrapper, and injects mermaid-fullscreen-btn and mermaid-download-btn buttons
  #   4. The emitted JS defines openMermaidModal and closeMermaidModal, closes on backdrop click of #mermaid-modal, and closes when key Escape is pressed
  #   5. The emitted JS clamps newScale with Math.max(0.5, Math.min(5, ...)), uses 1.2 factors for zoom-in/out, has a zoom-reset, and updates #zoom-level with a percentage
  #   6. The emitted JS sets isPanMode on Space keydown and builds a Blob of type image/svg+xml from the SVG outerHTML for download
  #   7. Rendering markdown containing a mermaid code fence still emits pre.mermaid and the existing server scenarios pass; mod.rs, scripts.rs, styles.rs, modal_styles.rs and mermaid_modal.rs are each under 300 lines
  #
  # ========================================
  Background: User Story
    As a developer viewing markdown attachments
    I want to open mermaid diagrams in a fullscreen modal with zoom, pan and SVG download
    So that I can inspect and export complex diagrams comfortably

  Scenario: Mermaid is initialized theme-aware with run on load
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the HTML imports mermaid version 11 as an ESM module
    And the HTML initializes mermaid with securityLevel loose and monospace font
    And the HTML configures the flowchart curve as basis
    And the HTML derives the mermaid theme from the saved fspec-theme
    And the HTML calls mermaid.run on load

  Scenario: Page loads the Panzoom CDN script
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the HTML contains a script src for panzoom version 4.5.1

  Scenario: Each mermaid diagram gets a wrapper with fullscreen and download buttons
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the JS selects pre.mermaid diagrams
    And the JS creates a div with class mermaid-wrapper
    And the JS injects a mermaid-fullscreen-btn button
    And the JS injects a mermaid-download-btn button

  Scenario: Modal opens and closes on ESC and backdrop click
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the JS defines an openMermaidModal function
    And the JS defines a closeMermaidModal function
    And the JS closes the modal on a backdrop click of the mermaid-modal element
    And the JS closes the modal when the Escape key is pressed

  Scenario: Zoom is clamped with reset and live percentage
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the JS clamps the scale between 0.5 and 5
    And the JS uses a 1.2 factor for zoom in and zoom out
    And the JS provides a zoom-reset control
    And the JS updates the zoom-level element with a percentage

  Scenario: Space enters pan mode and SVG downloads as a Blob
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the JS sets pan mode on Space key press
    And the JS builds a Blob of type image/svg+xml from the SVG markup for download

  Scenario: Existing mermaid server-render still works and files stay small
    Given I render markdown containing a mermaid code fence
    When I inspect the emitted HTML
    Then the HTML still contains a pre.mermaid block for the diagram
    And every template source file remains under 300 lines

  Scenario: Cursor-centered wheel zoom is clamped to 0.5x and 5x
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the JS registers a wheel event listener on the modal body
    And the JS defines a handleModalWheel function that locks the zoom point at the cursor
    And the wheel zoom clamps the new scale between 0.5 and 5

  Scenario: Horizontal scroll pans the diagram in zoom mode
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the JS pans horizontally by deltaX divided by the current scale when not zooming

  Scenario: The mode indicator fades after a period of inactivity
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the JS defines a showModeIndicator function that fades the indicator after a timeout

  Scenario: Holding Space toggles the pan-mode class on the diagram container
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the JS adds the pan-mode class to the diagram container in pan mode
    And the JS removes the pan-mode class when leaving pan mode
