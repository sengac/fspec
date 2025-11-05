@tui
@utils
@attachment-viewer
@validation
@markdown
@marked
@bug-fix
@unit-test
@TUI-022
Feature: Anchor links not working in markdown attachment viewer
  """
  Use marked-gfm-heading-id extension (official GitHub-compatible heading ID generator for marked library) - handles slug generation, duplicates, and special characters automatically
  Create new test file markdown-renderer.test.ts that tests renderMarkdown() directly (unit tests, no server startup) - faster, simpler, more focused than integration tests
  Existing TUI-020 integration tests should continue to pass - this is additive enhancement, not breaking change
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All markdown headings (h1-h6) must have unique id attributes for anchor link navigation
  #   2. Heading IDs must be GitHub-compatible slugs: lowercase, hyphenated, special chars removed
  #   3. Duplicate headings must get numbered suffixes (-1, -2, etc.) to ensure uniqueness
  #   4. Mermaid diagram rendering must continue to work (no regression)
  #   5. Code block rendering must continue to work with syntax highlighting (no regression)
  #   6. Tests must test renderMarkdown() directly without starting the HTTP server
  #
  # EXAMPLES:
  #   1. Heading '## Tag System' becomes <h2 id="tag-system">Tag System</h2>
  #   2. Heading '## Domain-to-Tag Mapping Rules' becomes <h2 id="domain-to-tag-mapping-rules">Domain-to-Tag Mapping Rules</h2>
  #   3. Two headings '## Summary' become <h2 id="summary">Summary</h2> and <h2 id="summary-1">Summary</h2>
  #   4. Heading with special chars '## What's New?' becomes <h2 id="whats-new">What's New?</h2>
  #   5. Anchor link [Jump to summary](#summary) navigates to heading with id="summary"
  #   6. Mermaid code block still renders as <pre class="mermaid"> with diagram content
  #
  # ========================================
  Background: User Story
    As a developer viewing markdown attachments in TUI
    I want to navigate using table of contents anchor links
    So that I can quickly jump to specific sections in long documents

  Scenario: Render basic heading with GitHub-compatible slug ID
    Given I have markdown content with a heading "## Tag System"
    When I call renderMarkdown() with the markdown
    Then the rendered HTML should contain '<h2 id="tag-system">Tag System</h2>'
    And the heading should be navigable via anchor link "#tag-system"

  Scenario: Render heading with multi-word title
    Given I have markdown content with a heading "## Domain-to-Tag Mapping Rules"
    When I call renderMarkdown() with the markdown
    Then the rendered HTML should contain '<h2 id="domain-to-tag-mapping-rules">Domain-to-Tag Mapping Rules</h2>'
    And the heading should be navigable via anchor link "#domain-to-tag-mapping-rules"

  Scenario: Handle duplicate headings with numbered suffixes
    Given I have markdown content with two headings both titled "## Summary"
    When I call renderMarkdown() with the markdown
    Then the first heading should have '<h2 id="summary">Summary</h2>'
    And the second heading should have '<h2 id="summary-1">Summary</h2>'
    And both headings should be uniquely navigable

  Scenario: Slugify heading with special characters
    Given I have markdown content with a heading "## What's New?"
    When I call renderMarkdown() with the markdown
    Then the rendered HTML should contain '<h2 id="whats-new">What\'s New?</h2>'
    And special characters should be removed from the ID
    And the heading should be navigable via anchor link "#whats-new"

  Scenario: Anchor links navigate to headings with matching IDs
    Given I have markdown with a heading "## Summary" and a link "[Jump to summary](#summary)"
    When I call renderMarkdown() with the markdown
    Then the heading should have 'id="summary"'
    And the link should have 'href="#summary"'
    And clicking the link should navigate to the heading element

  Scenario: Mermaid code blocks continue to render correctly (regression test)
    Given I have markdown content with a mermaid code block
    When I call renderMarkdown() with the markdown
    Then the rendered HTML should contain '<pre class="mermaid">'
    And the mermaid diagram content should be preserved
    And mermaid rendering should not be broken by heading ID changes

  Scenario: Code blocks continue to render with syntax highlighting (regression test)
    Given I have markdown content with a Python code block
    When I call renderMarkdown() with the markdown
    Then the rendered HTML should contain 'class="code-block"'
    And the code block should have 'data-language="python"'
    And syntax highlighting should not be broken by heading ID changes
