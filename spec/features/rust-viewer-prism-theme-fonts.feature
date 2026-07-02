@done
@rust
@attachment-viewer
@viewer
@RPC-377
Feature: Client-side viewer parity: Prism syntax highlighting, copy button, language badge, theme toggle and font-size controls in the Rust markdown viewer
  """
  Server-emitted client JS; Rust tests assert on the emitted HTML string (script/style/markup presence, Prism 1.29 + prism-vsc-dark-plus, alias entries, localStorage keys fspec-theme/fspec-base-font-size, clamp bounds 10/24). Only the <title> is escaped via html_escape. Existing rust-attachment-viewer-server.feature scenarios and RPC-376 anchor tests must not regress.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The served HTML includes Prism 1.29 core + autoloader script tags and the prism-vsc-dark-plus theme stylesheet link
  #   2. A DOMContentLoaded script applies a language alias map (sh/shell/console->bash, js->javascript, ts->typescript, py->python, rb->ruby, yml->yaml, text->plaintext) to code blocks and calls Prism.highlightAll()
  #   3. Each code block gets a Copy button (navigator.clipboard, 'Copied!' for 2s) and an uppercase language badge
  #   4. A theme-toggle control and JS read/write localStorage['fspec-theme'] and apply the theme on load; mermaid theme follows (dark/default)
  #   5. Font-size controls and JS read/write localStorage['fspec-base-font-size'], clamp to 10-24 (step 2, default 16), and disable the buttons at the bounds
  #   6. The stylesheet defines both dark (default :root) and light (:root.light-theme) CSS variable sets
  #   7. template.rs and each new submodule (mod.rs, styles.rs, scripts.rs) stay under 300 lines, and viewer_template(title, content_html)->String keeps working
  #
  # EXAMPLES:
  #   1. The emitted HTML contains 'prism/1.29.0/components/prism-core.min.js', the autoloader plugin script, and a link to prism-vsc-dark-plus.min.css
  #   2. The emitted HTML contains a DOMContentLoaded handler, the alias entries (e.g. sh: 'bash', ts: 'typescript'), and a Prism.highlightAll() call
  #   3. The emitted HTML/JS contains copy-button markup/creation, 'Copied!' text, and an uppercase language-badge
  #   4. The emitted HTML contains a theme-toggle button (id=theme-toggle) and JS reading/writing localStorage key 'fspec-theme'
  #   5. The emitted HTML contains font-size controls and JS with localStorage key 'fspec-base-font-size', bounds 10 and 24, and button-disabling at the bounds
  #   6. The stylesheet contains ':root' dark variables and ':root.light-theme' light variables (e.g. --bg-color differs)
  #   7. The existing scenario still holds: the page still embeds the mermaid script and the .markdown-content wrapper with the rendered content, and the title is escaped
  #
  # ========================================
  Background: User Story
    As a documentation reader
    I want to view rendered markdown with syntax highlighting, copy buttons, a theme toggle and font-size controls in the Rust viewer
    So that the Rust viewer offers the same client-side experience as the TypeScript viewer

  Scenario: Page includes Prism 1.29 scripts and theme stylesheet
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the HTML contains the Prism core script "prism/1.29.0/components/prism-core.min.js"
    And the HTML contains the Prism autoloader plugin script
    And the HTML contains a stylesheet link to "prism-vsc-dark-plus.min.css"

  Scenario: Page highlights code blocks with an alias map on DOMContentLoaded
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the HTML contains a "DOMContentLoaded" handler
    And the HTML contains the alias entry "sh: 'bash'"
    And the HTML contains the alias entry "ts: 'typescript'"
    And the HTML contains a "Prism.highlightAll()" call

  Scenario: Page adds copy buttons and uppercase language badges
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the HTML contains copy-button creation markup
    And the HTML contains the "Copied!" feedback text
    And the HTML contains a language-badge element

  Scenario: Page provides a theme toggle persisted to localStorage
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the HTML contains a theme-toggle button with id "theme-toggle"
    And the HTML reads and writes the localStorage key "fspec-theme"

  Scenario: Page provides clamped font-size controls persisted to localStorage
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the HTML contains font-size controls
    And the HTML reads and writes the localStorage key "fspec-base-font-size"
    And the HTML clamps the font size to the bounds 10 and 24
    And the HTML disables the controls at the bounds

  Scenario: Stylesheet defines both dark and light theme variables
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the stylesheet defines dark variables under ":root"
    And the stylesheet defines light variables under ":root.light-theme"

  Scenario: Existing mermaid script and content wrapper are preserved
    Given I render a viewer page with title "<x>.md" and body "<h1>Hi</h1>"
    When I inspect the emitted HTML
    Then the HTML still embeds the mermaid module script
    And the HTML still contains the ".markdown-content" wrapper around the body
    And the document title is escaped to "&lt;x&gt;.md"

  Scenario: Mermaid theme follows the saved viewer theme
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the mermaid script reads the localStorage key "fspec-theme"
    And the mermaid init selects theme "'dark'" or "'default'" from the saved theme

  Scenario: Prism language aliases map shorthand languages to Prism grammars
    Given I render a viewer page for some content
    When I inspect the emitted HTML
    Then the alias map contains "shell: 'bash'" and "console: 'bash'"
    And the alias map contains "js: 'javascript'", "py: 'python'", "rb: 'ruby'" and "yml: 'yaml'"
    And the language resolver maps "text" to "plaintext"
