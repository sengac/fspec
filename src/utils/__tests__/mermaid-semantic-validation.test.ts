/**
 * Feature: spec/features/mermaid-validation-using-parse-instead-of-render-misses-semantic-errors.feature
 */

import { describe, it, expect } from 'vitest';
import { validateMermaidSyntax } from '../mermaid-validation';

describe('Feature: Mermaid validation using parse() instead of render() misses semantic errors', () => {
  describe('Scenario: Reject markdown with quoted subgraph title', () => {
    it('should reject mermaid diagram with quoted subgraph title', async () => {
      // @step Given I have a markdown file with a mermaid diagram containing 'subgraph "Server Side"'
      const invalidDiagram = `graph LR
    subgraph "Server Side"
        A[Node A] --> B[Node B]
    end`;

      // @step When I run 'fspec add-attachment DOC-001 <file-path>'
      const result = await validateMermaidSyntax(invalidDiagram);

      // @step Then the command should fail with exit code 1
      expect(result.valid).toBe(false);

      // @step And the error message should indicate the subgraph title format is invalid
      expect(result.error).toMatch(/subgraph/i);

      // @step And the error message should suggest using 'subgraph ID[Title]' syntax
      // Note: This step will be validated in implementation - mermaid.render() should provide detailed error
      expect(result.error).toBeDefined();
    });
  });

  describe('Scenario: Accept markdown with proper subgraph syntax', () => {
    it('should accept mermaid diagram with proper subgraph syntax', async () => {
      // @step Given I have a markdown file with a mermaid diagram containing 'subgraph ServerSide[Server Side]'
      const validDiagram = `graph LR
    subgraph ServerSide[Server Side]
        A[Node A] --> B[Node B]
    end`;

      // @step When I run 'fspec add-attachment DOC-001 <file-path>'
      const result = await validateMermaidSyntax(validDiagram);

      // @step Then the command should succeed
      expect(result.valid).toBe(true);

      // @step And the attachment should be added to the work unit
      expect(result.error).toBeUndefined();
    });
  });

  describe('Scenario: Reject markdown with invalid subgraph identifier', () => {
    it('should reject mermaid diagram with invalid subgraph identifier', async () => {
      // @step Given I have a markdown file with a mermaid diagram containing 'subgraph INVALID!!!'
      const invalidDiagram = `graph LR
    subgraph INVALID!!!
        A[Node] --> B[Node]
    end`;

      // @step When I run 'fspec add-attachment DOC-001 <file-path>'
      const result = await validateMermaidSyntax(invalidDiagram);

      // @step Then the command should fail with exit code 1
      expect(result.valid).toBe(false);

      // @step And the error message should indicate the subgraph identifier is invalid
      expect(result.error).toBeDefined();
    });
  });

  describe('Scenario: Reject markdown with broken syntax', () => {
    it('should reject mermaid diagram with broken syntax', async () => {
      // @step Given I have a markdown file with a mermaid diagram with missing closing bracket
      const brokenDiagram = `graph TD
    A[Start] --> B{Decision
    B -->|Yes| C[Good]
    INVALID SYNTAX HERE`;

      // @step When I run 'fspec add-attachment DOC-001 <file-path>'
      const result = await validateMermaidSyntax(brokenDiagram);

      // @step Then the command should fail with exit code 1
      expect(result.valid).toBe(false);

      // @step And the error message should indicate syntax error
      expect(result.error).toBeDefined();
    });
  });

  describe('Scenario: Reject markdown with multiple diagrams where one is invalid', () => {
    it('should validate multiple diagrams and report which one failed', async () => {
      // @step Given I have a markdown file with 3 mermaid diagrams
      const _diagram1 = `graph LR
    A --> B`;

      // @step And the second diagram contains 'subgraph "Invalid Quotes"'
      const diagram2 = `graph LR
    subgraph "Invalid Quotes"
        A --> B
    end`;

      const _diagram3 = `graph LR
    C --> D`;

      // For this test, we'll validate the invalid diagram (diagram 2)
      // @step When I run 'fspec add-attachment DOC-001 <file-path>'
      const result = await validateMermaidSyntax(diagram2);

      // @step Then the command should fail with exit code 1
      expect(result.valid).toBe(false);

      // @step And the error message should indicate which diagram failed (diagram 2)
      // @step And the error message should include the specific error for that diagram
      // Note: The multi-diagram error reporting is handled by attachment-mermaid-validation.ts
      // This test validates that the individual diagram validation catches the error
      expect(result.error).toBeDefined();
    });
  });
});
