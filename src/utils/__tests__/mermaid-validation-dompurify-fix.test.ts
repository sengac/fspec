/**
 * Feature: spec/features/mermaid-validation-fails-with-dompurify-error.feature
 */

import { describe, it, expect } from 'vitest';
import { validateMermaidSyntax } from '../mermaid-validation';

describe('Feature: Mermaid Validation Fails with DOMPurify Error', () => {
  describe('Scenario: Attach markdown file with valid Mermaid code block', () => {
    it('should validate without DOMPurify error', async () => {
      // @step Given I have a work unit AUTH-001 in the backlog
      // (Not needed for this unit test)

      // @step And I have a file "architecture.md" containing a mermaid code block with valid syntax
      const validMermaidCode = `graph LR
  A[Client] --> B[Server]
  B --> C[Database]`;

      // @step When I run 'fspec add-attachment AUTH-001 architecture.md'
      // (Testing the underlying validation function)
      const result = await validateMermaidSyntax(validMermaidCode);

      // @step Then the mermaid code block should be extracted and validated
      // @step And the attachment should be added successfully
      // @step And no DOMPurify error should occur
      if (!result.valid) {
        console.error('Validation failed:', result.error);
      }
      expect(result.valid).toBe(true);
      expect(result.error).toBeUndefined();
    });
  });

  describe('Scenario: Attach Mermaid diagram file with valid syntax', () => {
    it('should validate successfully without DOMPurify.addHook error', async () => {
      // @step Given I have a work unit AUTH-001 in the backlog
      // (Not needed for this unit test)

      // @step And I have a file "flowchart.mmd" with valid Mermaid syntax
      const validMermaidCode = `graph TD
  A[Start] --> B[End]`;

      // @step When I run 'fspec add-attachment AUTH-001 flowchart.mmd'
      // (Testing the underlying validation function)
      const result = await validateMermaidSyntax(validMermaidCode);

      // @step Then the diagram should be validated successfully
      // @step And no DOMPurify.addHook error should occur
      // @step And the file should be copied to spec/attachments/AUTH-001/
      expect(result.valid).toBe(true);
      expect(result.error).toBeUndefined();
      // Should NOT throw "DOMPurify.addHook is not a function" error
    });
  });

  describe('Scenario: Attach Mermaid diagram with invalid syntax shows clear error', () => {
    it('should show clear syntax error without mentioning DOMPurify', async () => {
      // @step Given I have a work unit AUTH-001 in the backlog
      // (Not needed for this unit test)

      // @step And I have a file "diagram.mermaid" with invalid Mermaid syntax
      const invalidMermaidCode = `sequenceDiagram
  Alice->Bob: Hello
  INVALID SYNTAX HERE`;

      // @step When I run 'fspec add-attachment AUTH-001 diagram.mermaid'
      // (Testing the underlying validation function)
      const result = await validateMermaidSyntax(invalidMermaidCode);

      // @step Then the command should fail with a clear syntax error message
      // @step And the error should NOT mention DOMPurify
      // @step And the error should indicate the syntax problem
      expect(result.valid).toBe(false);
      expect(result.error).toBeDefined();
      expect(result.error).not.toContain('DOMPurify');
      expect(result.error).not.toContain('addHook');
    });
  });
});
