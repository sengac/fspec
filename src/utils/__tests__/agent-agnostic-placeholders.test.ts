/**
 * Feature: spec/features/replace-hardcoded-claude-md-references-with-agent-agnostic-placeholders.feature
 *
 * Tests verify that template files use {{DOC_TEMPLATE}} placeholder instead of hardcoded 'CLAUDE.md'
 */

import { describe, it, expect } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';

const projectRoot = join(__dirname, '../../..');

describe('Feature: Replace hardcoded CLAUDE.md references with agent-agnostic placeholders', () => {
  describe('Scenario: Replace hardcoded CLAUDE.md in bootstrapFoundation.ts with placeholder', () => {
    it('should use {{DOC_TEMPLATE}} placeholder instead of hardcoded CLAUDE.md', async () => {
      // @step Given the file "src/utils/slashCommandSections/bootstrapFoundation.ts" exists
      const filePath = join(
        projectRoot,
        'src/utils/slashCommandSections/bootstrapFoundation.ts'
      );

      // @step When I read the file content
      const content = await readFile(filePath, 'utf-8');

      // @step Then the file should contain "spec/{{DOC_TEMPLATE}}" placeholder
      expect(content).toContain('spec/{{DOC_TEMPLATE}}');

      // @step And the file should NOT contain hardcoded "spec/CLAUDE.md"
      expect(content).not.toContain('spec/CLAUDE.md');
    });
  });

  describe('Scenario: Replace hardcoded CLAUDE.md in fileStructure.ts with placeholder', () => {
    it('should use {{DOC_TEMPLATE}} placeholder in file structure example', async () => {
      // @step Given the file "src/utils/projectManagementSections/fileStructure.ts" exists
      const filePath = join(
        projectRoot,
        'src/utils/projectManagementSections/fileStructure.ts'
      );

      // @step When I read the file content
      const content = await readFile(filePath, 'utf-8');

      // @step Then the file should contain "{{DOC_TEMPLATE}}" placeholder in tree structure
      expect(content).toContain('├── {{DOC_TEMPLATE}}');

      // @step And the file should NOT contain hardcoded "├── CLAUDE.md"
      expect(content).not.toContain('├── CLAUDE.md');
    });
  });

  describe('Scenario: Replace hardcoded CLAUDE.md in enforcement.ts with placeholder', () => {
    it('should use {{DOC_TEMPLATE}} placeholder in exception list', async () => {
      // @step Given the file "src/utils/projectManagementSections/enforcement.ts" exists
      const filePath = join(
        projectRoot,
        'src/utils/projectManagementSections/enforcement.ts'
      );

      // @step When I read the file content
      const content = await readFile(filePath, 'utf-8');

      // @step Then the file should contain "{{DOC_TEMPLATE}}" placeholder
      expect(content).toContain('{{DOC_TEMPLATE}}');

      // @step And the file should NOT contain hardcoded "CLAUDE.md" in exception list
      // Note: We need to check the specific line about exceptions
      const lines = content.split('\n');
      const exceptionLine = lines.find(
        line => line.includes('Exception') && line.includes('FOUNDATION.md')
      );

      if (exceptionLine) {
        expect(exceptionLine).not.toContain('CLAUDE.md');
        expect(exceptionLine).toContain('{{DOC_TEMPLATE}}');
      }
    });
  });

  describe('Scenario: Verify Cursor agent sees correct references after fix', () => {
    it('should generate Cursor-specific references when templates use placeholders', async () => {
      // @step Given the template files use {{DOC_TEMPLATE}} placeholder
      const bootstrapPath = join(
        projectRoot,
        'src/utils/slashCommandSections/bootstrapFoundation.ts'
      );
      const fileStructurePath = join(
        projectRoot,
        'src/utils/projectManagementSections/fileStructure.ts'
      );
      const enforcementPath = join(
        projectRoot,
        'src/utils/projectManagementSections/enforcement.ts'
      );

      const bootstrapContent = await readFile(bootstrapPath, 'utf-8');
      const fileStructureContent = await readFile(fileStructurePath, 'utf-8');
      const enforcementContent = await readFile(enforcementPath, 'utf-8');

      // @step Then all template files should use placeholders
      expect(bootstrapContent).not.toContain('spec/CLAUDE.md');
      expect(fileStructureContent).not.toContain('├── CLAUDE.md');
      expect(enforcementContent).not.toContain(
        'FOUNDATION.md, TAGS.md, and CLAUDE.md'
      );

      // @step And placeholders should be present for replacement
      expect(bootstrapContent).toContain('{{DOC_TEMPLATE}}');
      expect(fileStructureContent).toContain('{{DOC_TEMPLATE}}');
      expect(enforcementContent).toContain('{{DOC_TEMPLATE}}');
    });
  });

  describe('Scenario: Verify Aider agent sees correct references after fix', () => {
    it('should ensure all agents benefit from placeholder-based templates', async () => {
      // @step Given all template files have been updated to use {{DOC_TEMPLATE}}
      const templateFiles = [
        'src/utils/slashCommandSections/bootstrapFoundation.ts',
        'src/utils/projectManagementSections/fileStructure.ts',
        'src/utils/projectManagementSections/enforcement.ts',
      ];

      // @step When I check all template files
      for (const file of templateFiles) {
        const filePath = join(projectRoot, file);
        const content = await readFile(filePath, 'utf-8');

        // @step Then no file should contain hardcoded "CLAUDE.md"
        expect(content).not.toContain('CLAUDE.md');

        // @step And all files should use {{DOC_TEMPLATE}} placeholder
        expect(content).toContain('{{DOC_TEMPLATE}}');
      }
    });
  });
});
