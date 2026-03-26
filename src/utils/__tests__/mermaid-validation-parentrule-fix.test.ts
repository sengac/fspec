/**
 * Feature: spec/features/generate-foundation-md-fails-with-mermaid-jsdom-parentrule-error-when-bounded-contexts-exist.feature
 *
 * This test file validates that mermaid.render() works correctly after
 * fixing the JSDOM CSSStyleDeclaration.parentRule getter-only crash.
 */

import { describe, it, expect } from 'vitest';
import { validateMermaidSyntax } from '../mermaid-validation';

describe('Feature: generate-foundation-md fails with mermaid JSDOM parentRule error when bounded contexts exist', () => {
  describe('Scenario: Mermaid validation succeeds with valid diagram after parentRule fix', () => {
    it('should validate a valid flowchart without parentRule error', async () => {
      // @step Given a valid mermaid flowchart diagram
      const validDiagram = `flowchart TD
  A[Client] --> B[Server]
  B --> C[Database]`;

      // @step When I validate the diagram using validateMermaidSyntax
      const result = await validateMermaidSyntax(validDiagram);

      // @step Then the validation result should be valid
      expect(result.valid).toBe(true);
      expect(result.error).toBeUndefined();
    });
  });

  describe('Scenario: Mermaid validation still rejects invalid diagrams after parentRule fix', () => {
    it('should reject an invalid diagram with a syntax error', async () => {
      // @step Given an invalid mermaid diagram with syntax errors
      const invalidDiagram = `flowchart TD
  INVALID SYNTAX %%% NOT VALID
  --> --> -->`;

      // @step When I validate the diagram using validateMermaidSyntax
      const result = await validateMermaidSyntax(invalidDiagram);

      // @step Then the validation result should not be valid
      expect(result.valid).toBe(false);

      // @step Then the error message should describe the syntax problem
      expect(result.error).toBeDefined();
      expect(result.error).not.toContain('parentRule');
    });
  });

  describe('Scenario: Bounded context mermaid diagram renders successfully', () => {
    it('should validate a bounded context diagram with subgraphs', async () => {
      // @step Given a mermaid diagram with bounded context subgraphs
      const boundedContextDiagram = `flowchart TD
  subgraph BrowserAgent[Browser Agent]
    A[Extension] --> B[Content Script]
  end
  subgraph CoreEngine[Core Engine]
    C[Scheduler] --> D[Worker]
  end
  BrowserAgent --> CoreEngine`;

      // @step When I validate the diagram using validateMermaidSyntax
      const result = await validateMermaidSyntax(boundedContextDiagram);

      // @step Then the validation result should be valid
      expect(result.valid).toBe(true);
      expect(result.error).toBeUndefined();
    });
  });
});
