@done
@viewer
@markdown
@rust
@markdown-formatting
@attachment-viewer
@RPC-380
Feature: GFM footnote-option alignment with marked in the Rust markdown viewer

  """
  In render.rs, remove options.insert(Options::ENABLE_FOOTNOTES); from the Options set passed to Parser::new_ext. Keep ENABLE_TABLES, ENABLE_STRIKETHROUGH, ENABLE_TASKLISTS. Update the module doc comment accordingly. Verify the existing tests still pass and add a test asserting footnote syntax does not produce footnote markup. Keep files <300 lines; cargo test/clippy/fmt clean. No HTTP-layer (axum) changes.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The pulldown-cmark Options used by render_markdown must NOT include ENABLE_FOOTNOTES, so footnote definition/reference syntax is not given dedicated footnote rendering
  #   2. The rendered HTML for footnote-reference syntax must not contain footnote-specific markup (no <sup> footnote reference, no footnote-definition list/section) that marked never emits
  #   3. The remaining enabled extensions (tables, strikethrough, task lists) continue to render unchanged after footnotes are disabled
  #
  # EXAMPLES:
  #   1. Markdown 'Text[^1]\n\n[^1]: a note' renders without any footnote-specific markup (no element with a footnote reference id or footnote definition section)
  #   2. A GFM table still renders a <table> element after footnotes are disabled
  #   3. Strikethrough '~~gone~~' still renders a <del> element after footnotes are disabled
  #
  # ========================================

  Background: User Story
    As a fspec user viewing markdown attachments in the browser viewer
    I want to see footnote-style text rendered the same way the TypeScript viewer renders it
    So that the Rust viewer does not produce footnote markup the original viewer never generates

  Scenario: Footnote syntax does not produce footnote markup
    Given markdown text "Text[^1]" with a definition line "[^1]: a note"
    When the markdown is rendered to HTML
    Then the output does not contain a footnote reference element
    And the output does not contain a footnote definition section

  Scenario: Tables still render after footnotes are disabled
    Given markdown containing a GFM table
    When the markdown is rendered to HTML
    Then the output contains a "<table>" element

  Scenario: Strikethrough still renders after footnotes are disabled
    Given markdown text "~~gone~~"
    When the markdown is rendered to HTML
    Then the output contains a "<del>" element
