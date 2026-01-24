import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { addDiagramJsonBacked } from '../add-diagram-json-backed';
import { createFoundationWithDiagrams } from '../../test-helpers/foundation-with-diagram-fixtures';

import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
describe('Feature: Add Diagram to JSON-Backed Foundation', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('add-diagram-json-backed');
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Add new Mermaid diagram to Architecture Diagrams section', () => {
    it('should add diagram to foundation.json and regenerate FOUNDATION.md', async () => {
      // Given I have a valid file "spec/foundation.json"
      const foundationFile = join(testDir, 'spec', 'foundation.json');
      const initialFoundation = createFoundationWithDiagrams();
      await writeFile(
        foundationFile,
        JSON.stringify(initialFoundation, null, 2)
      );

      // When I run add-diagram command
      const result = await addDiagramJsonBacked({
        title: 'New System Flow',
        mermaidCode: 'graph TB\n  A[Start]\n  B[End]\n  A-->B',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And "spec/foundation.json" should contain the new diagram
      const updatedContent = await readFile(foundationFile, 'utf-8');
      const updatedData = JSON.parse(updatedContent);
      expect(updatedData.architectureDiagrams).toHaveLength(1);
      expect(updatedData.architectureDiagrams[0].title).toBe('New System Flow');
      expect(updatedData.architectureDiagrams[0].mermaidCode).toBe(
        'graph TB\n  A[Start]\n  B[End]\n  A-->B'
      );

      // And "spec/FOUNDATION.md" should be regenerated
      const foundationMd = join(testDir, 'spec', 'FOUNDATION.md');
      const mdContent = await readFile(foundationMd, 'utf-8');
      expect(mdContent).toContain('### New System Flow');
      expect(mdContent).toContain('```mermaid');
      expect(mdContent).toContain('graph TB');
      expect(mdContent).toContain('A[Start]');
    });
  });

  describe('Scenario: Update existing diagram with same title', () => {
    it('should update diagram in foundation.json with same title', async () => {
      // Given I have a valid file "spec/foundation.json"
      // And it contains a diagram titled "fspec System Context"
      const foundationFile = join(testDir, 'spec', 'foundation.json');
      const initialFoundation = createFoundationWithDiagrams([
        {
          title: 'fspec System Context',
          mermaidCode: 'graph TB\n  OLD[Old]',
        },
      ]);
      await writeFile(
        foundationFile,
        JSON.stringify(initialFoundation, null, 2)
      );

      // When I run add-diagram with same title
      const result = await addDiagramJsonBacked({
        title: 'fspec System Context',
        mermaidCode: 'graph TB\n  NEW[Updated]',
        cwd: testDir,
      });

      // Then the existing diagram should be updated (not duplicated)
      expect(result.success).toBe(true);

      // And "spec/foundation.json" should contain only one diagram with that title
      const updatedContent = await readFile(foundationFile, 'utf-8');
      const updatedData = JSON.parse(updatedContent);
      expect(updatedData.architectureDiagrams).toHaveLength(1);
      expect(updatedData.architectureDiagrams[0].title).toBe(
        'fspec System Context'
      );
      expect(updatedData.architectureDiagrams[0].mermaidCode).toBe(
        'graph TB\n  NEW[Updated]'
      );
    });
  });

  describe('Scenario: Add diagram with description', () => {
    it('should include description field in diagram', async () => {
      // Given I have a valid file "spec/foundation.json"
      const foundationFile = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createFoundationWithDiagrams();
      await writeFile(
        foundationFile,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run add-diagram with description
      const result = await addDiagramJsonBacked({
        title: 'Data Flow',
        mermaidCode: 'graph LR\n  A-->B',
        description: 'Shows data flow between components',
        cwd: testDir,
      });

      // Then "spec/foundation.json" should contain the diagram with description
      expect(result.success).toBe(true);
      const content = await readFile(foundationFile, 'utf-8');
      const data = JSON.parse(content);
      expect(data.architectureDiagrams[0].description).toBe(
        'Shows data flow between components'
      );
    });
  });

  describe('Scenario: Read Mermaid code from file', () => {
    it('should read diagram code from external file', async () => {
      // Given I have a file "diagram.mmd" containing Mermaid code
      const diagramFile = join(testDir, 'diagram.mmd');
      await writeFile(
        diagramFile,
        'graph TB\n  Complex[Diagram]\n  From[File]'
      );

      const foundationFile = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createFoundationWithDiagrams();
      await writeFile(
        foundationFile,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run add-diagram with file parameter
      const result = await addDiagramJsonBacked({
        title: 'Complex Diagram',
        file: diagramFile,
        cwd: testDir,
      });

      // Then the Mermaid code should be read from file
      expect(result.success).toBe(true);

      // And it should be added to "spec/foundation.json"
      const content = await readFile(foundationFile, 'utf-8');
      const data = JSON.parse(content);
      expect(data.architectureDiagrams[0].mermaidCode).toBe(
        'graph TB\n  Complex[Diagram]\n  From[File]'
      );
    });
  });

  describe('Scenario: Validate Mermaid syntax', () => {
    it('should validate Mermaid syntax before adding to JSON', async () => {
      // Given I have a valid file "spec/foundation.json"
      const foundationFile = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createFoundationWithDiagrams();
      await writeFile(
        foundationFile,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run add-diagram with invalid Mermaid syntax
      const result = await addDiagramJsonBacked({
        title: 'Invalid Diagram',
        mermaidCode: 'invalid mermaid syntax',
        cwd: testDir,
      });

      // Then the command should display a warning
      expect(result.warning).toContain('Mermaid syntax may be invalid');

      // But the diagram should still be added (warning only, not error)
      expect(result.success).toBe(true);
      const content = await readFile(foundationFile, 'utf-8');
      const data = JSON.parse(content);
      expect(data.architectureDiagrams[0].mermaidCode).toBe(
        'invalid mermaid syntax'
      );
    });
  });

  describe('Scenario: Fail if section does not exist', () => {
    it('should error when diagram section does not exist in foundation.json', async () => {
      // Given I have a valid file "spec/foundation.json"
      const foundationFile = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createFoundationWithDiagrams();
      await writeFile(
        foundationFile,
        JSON.stringify(minimalFoundation, null, 2)
      );

      const originalContent = await readFile(foundationFile, 'utf-8');

      // When I run add-diagram with nonexistent section
      // Then the command should exit with code 1
      await expect(
        addDiagramJsonBacked({
          title: 'Diagram',
          mermaidCode: 'graph TB',
          section: 'Nonexistent Section',
          cwd: testDir,
        })
      ).rejects.toThrow("Section 'Nonexistent Section' not found");

      // And "spec/foundation.json" should not be modified
      const unchangedContent = await readFile(foundationFile, 'utf-8');
      expect(unchangedContent).toBe(originalContent);
    });
  });

  describe('Scenario: Rollback if markdown generation fails', () => {
    it('should rollback JSON changes if MD generation fails', async () => {
      // Given I have a valid file "spec/foundation.json"
      const foundationFile = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createFoundationWithDiagrams();
      await writeFile(
        foundationFile,
        JSON.stringify(minimalFoundation, null, 2)
      );

      const originalContent = await readFile(foundationFile, 'utf-8');

      // When I run add-diagram and markdown generation fails
      // Then the command should exit with code 1
      await expect(
        addDiagramJsonBacked({
          title: 'New Diagram',
          mermaidCode: 'graph TB',
          cwd: testDir,
          forceRegenerationFailure: true,
        })
      ).rejects.toThrow('Failed to regenerate FOUNDATION.md');

      // And "spec/foundation.json" should be rolled back to previous state
      const rolledBackContent = await readFile(foundationFile, 'utf-8');
      expect(rolledBackContent).toBe(originalContent);
    });
  });

  describe('Scenario: Support multiple diagram sections', () => {
    it('should support adding diagrams to different sections', async () => {
      // Given "spec/foundation.json" has multiple sections that can contain diagrams
      const foundationFile = join(testDir, 'spec', 'foundation.json');
      const foundationWithSections = createFoundationWithDiagrams([
        { title: 'Existing Diagram', mermaidCode: 'graph TB\n  X[Existing]' },
      ]);
      await writeFile(
        foundationFile,
        JSON.stringify(foundationWithSections, null, 2)
      );

      // When I run add-diagram to specific section
      const result = await addDiagramJsonBacked({
        title: 'Diagram 1',
        mermaidCode: 'graph TB',
        section: 'Architecture Diagrams',
        cwd: testDir,
      });

      // Then the diagram should be added to the correct section
      expect(result.success).toBe(true);
      const content = await readFile(foundationFile, 'utf-8');
      const data = JSON.parse(content);
      expect(data.architectureDiagrams).toHaveLength(2);
      expect(data.architectureDiagrams[1].title).toBe('Diagram 1');

      // And other sections should not be affected
      expect(data.architectureDiagrams[0].title).toBe('Existing Diagram');
    });
  });
});
