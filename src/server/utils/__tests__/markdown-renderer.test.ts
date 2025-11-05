/**
 * Feature: spec/features/anchor-links-not-working-in-markdown-attachment-viewer.feature
 *
 * Tests for TUI-022: Anchor links not working in markdown attachment viewer
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 *
 * Testing strategy: Call renderMarkdown() directly (NO HTTP server startup)
 * - Faster execution
 * - Simpler test setup
 * - Focused on markdown rendering logic
 * - No network layer complexity
 */

import { describe, it, expect } from 'vitest';
import { renderMarkdown } from '../markdown-renderer';

describe('Feature: Anchor links not working in markdown attachment viewer', () => {
  describe('Scenario: Render basic heading with GitHub-compatible slug ID', () => {
    it('should generate heading with id attribute for anchor navigation', () => {
      // @step Given I have markdown content with a heading "## Tag System"
      const markdown = '## Tag System\n\nSome content here.';

      // @step When I call renderMarkdown() with the markdown
      const html = renderMarkdown(markdown);

      // @step Then the rendered HTML should contain '<h2 id="tag-system">Tag System</h2>'
      expect(html).toContain('<h2 id="tag-system">Tag System</h2>');

      // @step And the heading should be navigable via anchor link "#tag-system"
      expect(html).toContain('id="tag-system"');
    });
  });

  describe('Scenario: Render heading with multi-word title', () => {
    it('should generate hyphenated slug for multi-word heading', () => {
      // @step Given I have markdown content with a heading "## Domain-to-Tag Mapping Rules"
      const markdown =
        '## Domain-to-Tag Mapping Rules\n\nMapping rules content.';

      // @step When I call renderMarkdown() with the markdown
      const html = renderMarkdown(markdown);

      // @step Then the rendered HTML should contain '<h2 id="domain-to-tag-mapping-rules">Domain-to-Tag Mapping Rules</h2>'
      expect(html).toContain(
        '<h2 id="domain-to-tag-mapping-rules">Domain-to-Tag Mapping Rules</h2>'
      );

      // @step And the heading should be navigable via anchor link "#domain-to-tag-mapping-rules"
      expect(html).toContain('id="domain-to-tag-mapping-rules"');
    });
  });

  describe('Scenario: Handle duplicate headings with numbered suffixes', () => {
    it('should add numbered suffixes to duplicate heading IDs', () => {
      // @step Given I have markdown content with two headings both titled "## Summary"
      const markdown = `
# Document

## Summary
First summary section.

## Summary
Second summary section.
`;

      // @step When I call renderMarkdown() with the markdown
      const html = renderMarkdown(markdown);

      // @step Then the first heading should have '<h2 id="summary">Summary</h2>'
      expect(html).toMatch(/<h2 id="summary">Summary<\/h2>/);

      // @step And the second heading should have '<h2 id="summary-1">Summary</h2>'
      expect(html).toMatch(/<h2 id="summary-1">Summary<\/h2>/);

      // @step And both headings should be uniquely navigable
      expect(html).toContain('id="summary"');
      expect(html).toContain('id="summary-1"');
    });
  });

  describe('Scenario: Slugify heading with special characters', () => {
    it('should remove special characters from heading ID', () => {
      // @step Given I have markdown content with a heading "## What's New?"
      const markdown = "## What's New?\n\nNew features listed here.";

      // @step When I call renderMarkdown() with the markdown
      const html = renderMarkdown(markdown);

      // @step Then the rendered HTML should contain '<h2 id="whats-new">What's New?</h2>'
      // Note: marked escapes apostrophes to &#39; in HTML output
      expect(html).toContain('<h2 id="whats-new">What&#39;s New?</h2>');

      // @step And special characters should be removed from the ID
      expect(html).toContain('id="whats-new"');
      expect(html).not.toContain('id="what\'s-new?"');

      // @step And the heading should be navigable via anchor link "#whats-new"
      expect(html).toContain('id="whats-new"');
    });
  });

  describe('Scenario: Anchor links navigate to headings with matching IDs', () => {
    it('should preserve anchor links that reference heading IDs', () => {
      // @step Given I have markdown with a heading "## Summary" and a link "[Jump to summary](#summary)"
      const markdown = `
[Jump to summary](#summary)

## Summary
Summary content here.
`;

      // @step When I call renderMarkdown() with the markdown
      const html = renderMarkdown(markdown);

      // @step Then the heading should have 'id="summary"'
      expect(html).toContain('id="summary"');

      // @step And the link should have 'href="#summary"'
      expect(html).toContain('href="#summary"');

      // @step And clicking the link should navigate to the heading element
      // (This is verified by matching href="#summary" to id="summary")
      expect(html).toMatch(/href="#summary".*id="summary"/s);
    });
  });

  describe('Scenario: Mermaid code blocks continue to render correctly (regression test)', () => {
    it('should not break mermaid rendering when adding heading IDs', () => {
      // @step Given I have markdown content with a mermaid code block
      const markdown = `
# Diagram

\`\`\`mermaid
graph TD
  A[Start] --> B[End]
\`\`\`
`;

      // @step When I call renderMarkdown() with the markdown
      const html = renderMarkdown(markdown);

      // @step Then the rendered HTML should contain '<pre class="mermaid">'
      expect(html).toContain('<pre class="mermaid">');

      // @step And the mermaid diagram content should be preserved
      expect(html).toContain('graph TD');
      expect(html).toContain('A[Start] --&gt; B[End]');

      // @step And mermaid rendering should not be broken by heading ID changes
      // Verify heading still has ID
      expect(html).toContain('id="diagram"');
    });
  });

  describe('Scenario: Code blocks continue to render with syntax highlighting (regression test)', () => {
    it('should not break code block rendering when adding heading IDs', () => {
      // @step Given I have markdown content with a Python code block
      const markdown = `
# Code Example

\`\`\`python
def hello():
    print("Hello, World!")
\`\`\`
`;

      // @step When I call renderMarkdown() with the markdown
      const html = renderMarkdown(markdown);

      // @step Then the rendered HTML should contain 'class="code-block"'
      expect(html).toContain('class="code-block"');

      // @step And the code block should have 'data-language="python"'
      expect(html).toContain('data-language="python"');

      // @step And syntax highlighting should not be broken by heading ID changes
      // Verify code content is preserved
      expect(html).toContain('def hello():');
      // Verify heading still has ID
      expect(html).toContain('id="code-example"');
    });
  });
});
