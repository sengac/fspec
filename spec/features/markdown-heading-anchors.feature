@done
@attachment-viewer
@markdown-formatting
@rust
@markdown
@viewer
@RPC-376
Feature: Heading anchor IDs and in-page anchor navigation in the Rust markdown viewer
  """
  Render options: ENABLE_SMART_PUNCTUATION removed (no curly quotes), SoftBreak mapped to <br> for breaks parity. ENABLE_TABLES/STRIKETHROUGH/TASKLISTS/FOOTNOTES kept.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each heading h1-h6 renders a GitHub-style slug id derived from its plain-text content
  #   2. Slugify lowercases, strips characters that are not alphanumeric/space/hyphen, and replaces whitespace runs with a single hyphen
  #   3. Duplicate heading slugs within one document are de-duplicated with -1, -2 numeric suffixes per a document-scoped counter
  #   4. Author-written anchor links keep their href so they navigate to the matching heading id
  #   5. A soft line break inside a paragraph renders as a hard <br> (breaks parity with marked breaks:true)
  #   6. Smart punctuation is NOT applied; straight apostrophes and quotes stay literal
  #   7. Mermaid and language-tagged code blocks continue to render unchanged when heading ids are added
  #
  # EXAMPLES:
  #   1. A heading '## Summary' renders as '<h2 id="summary">Summary</h2>'
  #   2. A heading '## Domain-to-Tag Mapping Rules' renders id 'domain-to-tag-mapping-rules'
  #   3. A heading "## What's New?" renders id 'whats-new'
  #   4. Three headings all titled 'Summary' render ids summary, summary-1, summary-2
  #   5. '[Jump to summary](#summary)' above a '## Summary' keeps href='#summary' and heading gets id='summary'
  #   6. Paragraph 'line1\nline2' renders 'line1<br>line2'
  #   7. Text "it's" renders a literal apostrophe (&#39;) not a curly quote
  #   8. A mermaid block still renders '<pre class="mermaid">' and a python block still renders data-language='python'
  #
  # ========================================
  Background: User Story
    As a documentation author
    I want to write anchor links to markdown headings in the Rust attachment viewer
    So that in-page navigation works the same as the TypeScript viewer

  Scenario: Render heading with a single-word title
    Given I have markdown content with a heading "## Summary"
    When I render the markdown to HTML
    Then the rendered HTML should contain "<h2 id=\"summary\">Summary</h2>"

  Scenario: Render heading with a multi-word title
    Given I have markdown content with a heading "## Domain-to-Tag Mapping Rules"
    When I render the markdown to HTML
    Then the rendered HTML should contain "<h2 id=\"domain-to-tag-mapping-rules\">Domain-to-Tag Mapping Rules</h2>"

  Scenario: Strip special characters from the slug
    Given I have markdown content with a heading "## What's New?"
    When I render the markdown to HTML
    Then the rendered HTML should contain "id=\"whats-new\""
    And the rendered HTML should not contain "id=\"what's-new?\""

  Scenario: De-duplicate repeated heading slugs with numeric suffixes
    Given I have markdown content with three headings all titled "Summary"
    When I render the markdown to HTML
    Then the rendered HTML should contain "id=\"summary\""
    And the rendered HTML should contain "id=\"summary-1\""
    And the rendered HTML should contain "id=\"summary-2\""

  Scenario: Anchor link round-trips to a heading id
    Given I have markdown with a link "[Jump to summary](#summary)" above a heading "## Summary"
    When I render the markdown to HTML
    Then the rendered HTML should contain "href=\"#summary\""
    And the rendered HTML should contain "id=\"summary\""

  Scenario: Soft line break renders as a hard break
    Given I have a paragraph with a soft line break between "line1" and "line2"
    When I render the markdown to HTML
    Then the rendered HTML should contain "line1<br>" followed by "line2"

  Scenario: Smart punctuation is not applied
    Given I have markdown text containing a straight apostrophe in "it's"
    When I render the markdown to HTML
    Then the rendered HTML should contain a straight apostrophe in "it's"
    And the rendered HTML should not contain a curly apostrophe

  Scenario: Code and mermaid blocks remain unaffected by heading ids
    Given I have markdown with a heading, a mermaid block, and a python code block
    When I render the markdown to HTML
    Then the rendered HTML should contain "<pre class=\"mermaid\">"
    And the rendered HTML should contain "data-language=\"python\""
    And the rendered HTML should contain a heading id for the heading
