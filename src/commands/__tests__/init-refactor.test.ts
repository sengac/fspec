/**
 * Feature: spec/features/wire-up-multi-agent-support-to-fspec-init-command.feature
 *
 * Tests for multi-agent init command wiring
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join } from 'path';

describe('Feature: Wire up multi-agent support to fspec init command', () => {
  describe('Scenario: CLI mode skips interactive selector', () => {
    it('should accept --agent flag for non-interactive mode', () => {
      // Verify that the command registration includes --agent option
      const initFilePath = join(process.cwd(), 'src', 'commands', 'init.ts');
      const initFileContent = readFileSync(initFilePath, 'utf-8');

      // Then the command should accept --agent flag
      const hasAgentOption = initFileContent.includes("'--agent <agent>'");

      expect(hasAgentOption).toBe(true);
    });
  });

  describe('Scenario: Deprecated code is removed', () => {
    it('should not have old init() function in source code', () => {
      // Given the multi-agent wiring is complete
      // When I inspect src/commands/init.ts
      const initFilePath = join(process.cwd(), 'src', 'commands', 'init.ts');
      const initFileContent = readFileSync(initFilePath, 'utf-8');

      // Then the init() function should not exist (old single-agent version)
      // This should look for the specific signature of the old init function
      const hasOldInitFunction = initFileContent.includes('export async function init(options: InitOptions)');

      expect(hasOldInitFunction).toBe(false);
    });

    it('should not have old generateTemplate() function', () => {
      const initFilePath = join(process.cwd(), 'src', 'commands', 'init.ts');
      const initFileContent = readFileSync(initFilePath, 'utf-8');

      // Then the generateTemplate() function should not exist
      const hasGenerateTemplate = initFileContent.includes('async function generateTemplate()');

      expect(hasGenerateTemplate).toBe(false);
    });

    it('should not have old copyClaudeTemplate() function', () => {
      const initFilePath = join(process.cwd(), 'src', 'commands', 'init.ts');
      const initFileContent = readFileSync(initFilePath, 'utf-8');

      // Then the copyClaudeTemplate() function should not exist
      const hasCopyClaudeTemplate = initFileContent.includes('async function copyClaudeTemplate(');

      expect(hasCopyClaudeTemplate).toBe(false);
    });

    it('should not have old --type option in registerInitCommand', () => {
      const initFilePath = join(process.cwd(), 'src', 'commands', 'init.ts');
      const initFileContent = readFileSync(initFilePath, 'utf-8');

      // Then the old --type and --path options should not exist
      const hasTypeOption = initFileContent.includes("'--type <type>'");
      const hasPathOption = initFileContent.includes("'--path <path>'");

      expect(hasTypeOption).toBe(false);
      expect(hasPathOption).toBe(false);
    });
  });
});
