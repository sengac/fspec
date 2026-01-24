/**
 * Feature: spec/features/command-help-validation.feature
 *
 * Tests for the command help file validation script
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { execFile } from 'child_process';
import { promisify } from 'util';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../test-helpers/temp-directory';

const execFileAsync = promisify(execFile);

describe('Feature: Command Help File Validation Script', () => {
  let testDir: string;
  let commandsDir: string;
  let srcDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('validate-help');
    srcDir = join(testDir, 'src');
    commandsDir = join(srcDir, 'commands');

    await mkdir(commandsDir, { recursive: true });
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Detect missing help file for registered command', () => {
    it('should detect and report missing help file', async () => {
      // Given a command "add-diagram" is registered in the CLI
      const indexContent = `
import { registerAddDiagramCommand } from './commands/add-diagram';

program.addCommand(registerAddDiagramCommand());
`;
      await writeFile(join(srcDir, 'index.ts'), indexContent);

      // And no file "src/commands/add-diagram-help.ts" exists
      // (don't create the help file)

      // When I run the validation script
      const scriptPath = join(
        process.cwd(),
        'scripts/validate-command-help.js'
      );

      try {
        await execFileAsync('node', [scriptPath, testDir]);
        expect.fail('Script should have exited with non-zero code');
      } catch (error: unknown) {
        const execError = error as {
          code: number;
          stdout: string;
          stderr: string;
        };
        // Then the script should exit with non-zero code
        expect(execError.code).toBe(1);

        const output = (execError.stdout || '') + (execError.stderr || '');

        // And the output should contain "Missing help file for command: add-diagram"
        expect(output).toContain('Missing help file for command: add-diagram');

        // And the output should include the expected file path
        expect(output).toContain('src/commands/add-diagram-help.ts');
      }
    });
  });

  describe('Scenario: Detect orphaned help file with no command', () => {
    it('should detect help file without corresponding command', async () => {
      // Given a file "src/commands/delete-foo-help.ts" exists
      const helpContent = `
export function getDeleteFooHelp(): string {
  return 'Help content for delete-foo command';
}
`;
      await writeFile(join(commandsDir, 'delete-foo-help.ts'), helpContent);

      // And no command "delete-foo" is registered in the CLI
      const indexContent = `
import { registerAddDiagramCommand } from './commands/add-diagram';

program.addCommand(registerAddDiagramCommand());
`;
      await writeFile(join(srcDir, 'index.ts'), indexContent);

      // When I run the validation script
      const scriptPath = join(
        process.cwd(),
        'scripts/validate-command-help.js'
      );

      try {
        await execFileAsync('node', [scriptPath, testDir]);
        expect.fail('Script should have exited with non-zero code');
      } catch (error: unknown) {
        const execError = error as {
          code: number;
          stdout: string;
          stderr: string;
        };
        // Then the script should exit with non-zero code
        expect(execError.code).toBe(1);

        const output = (execError.stdout || '') + (execError.stderr || '');

        // And the output should contain "Orphaned help file: delete-foo-help.ts"
        expect(output).toContain('Orphaned help file: delete-foo-help.ts');

        // And the output should suggest removing the file or registering the command
        expect(output).toMatch(/src\/commands\/delete-foo-help.ts/);
      }
    });
  });

  describe('Scenario: Validate all commands and help files are properly linked', () => {
    it('should pass validation when all commands have help files', async () => {
      // Given all registered commands have corresponding help files
      const indexContent = `
import { registerAddDiagramCommand } from './commands/add-diagram';
import { registerValidateCommand } from './commands/validate';

program.addCommand(registerAddDiagramCommand());
program.addCommand(registerValidateCommand());
`;
      await writeFile(join(srcDir, 'index.ts'), indexContent);

      const addDiagramHelp = `export function getAddDiagramHelp(): string {
  return \`
USAGE
  fspec add-diagram <section> <name> <diagram>

EXAMPLES
  fspec add-diagram "Architecture" "System Flow" "graph TB"
\`;
}
`;
      await writeFile(join(commandsDir, 'add-diagram-help.ts'), addDiagramHelp);

      const validateHelp = `export function getValidateHelp(): string {
  return \`
USAGE
  fspec validate [file]

EXAMPLES
  fspec validate spec/features/login.feature
\`;
}
`;
      await writeFile(join(commandsDir, 'validate-help.ts'), validateHelp);

      // And all help files have corresponding registered commands
      // And all help files export the correct function signature
      // When I run the validation script
      const scriptPath = join(
        process.cwd(),
        'scripts/validate-command-help.js'
      );

      const result = await execFileAsync('node', [scriptPath, testDir]);

      // Then the script should exit with code 0
      expect(result.stderr).toBe('');

      // And the output should contain "All commands validated successfully"
      expect(result.stdout).toContain('All commands validated successfully');

      // And the output should display the total number of commands checked
      expect(result.stdout).toMatch(/checked \d+ commands/i);
    });
  });

  describe('Scenario: Detect help file with empty content', () => {
    it('should detect empty help file', async () => {
      // Given a command "validate" is registered
      const indexContent = `
import { registerValidateCommand } from './commands/validate';

program.addCommand(registerValidateCommand());
`;
      await writeFile(join(srcDir, 'index.ts'), indexContent);

      // And the file "src/commands/validate-help.ts" exists but is empty
      await writeFile(join(commandsDir, 'validate-help.ts'), '');

      // When I run the validation script
      const scriptPath = join(
        process.cwd(),
        'scripts/validate-command-help.js'
      );

      try {
        await execFileAsync('node', [scriptPath, testDir]);
        expect.fail('Script should have exited with non-zero code');
      } catch (error: unknown) {
        const execError = error as {
          code: number;
          stdout: string;
          stderr: string;
        };
        // Then the script should exit with non-zero code
        expect(execError.code).toBe(1);

        const output = (execError.stdout || '') + (execError.stderr || '');

        // And the output should contain "Help content is empty: validate-help.ts"
        expect(output).toMatch(/Help content is empty.*validate-help\.ts/);
      }
    });
  });

  describe('Scenario: Detect help file with inconsistent formatting', () => {
    it('should detect missing USAGE section', async () => {
      // Given multiple help files exist with different formatting structures
      const indexContent = `
import { registerAddRuleCommand } from './commands/add-rule';

program.addCommand(registerAddRuleCommand());
`;
      await writeFile(join(srcDir, 'index.ts'), indexContent);

      // And the file "src/commands/add-rule-help.ts" has an empty USAGE section
      const addRuleHelp = `
export function getAddRuleHelp(): string {
  return \`
EXAMPLES
  fspec add-rule WORK-001 "User must be authenticated"
\`;
}
`;
      await writeFile(join(commandsDir, 'add-rule-help.ts'), addRuleHelp);

      // When I run the validation script
      const scriptPath = join(
        process.cwd(),
        'scripts/validate-command-help.js'
      );

      try {
        await execFileAsync('node', [scriptPath, testDir]);
        expect.fail('Script should have exited with non-zero code');
      } catch (error: unknown) {
        const execError = error as {
          code: number;
          stdout: string;
          stderr: string;
        };
        // Then the script should exit with non-zero code
        expect(execError.code).toBe(1);

        const output = (execError.stdout || '') + (execError.stderr || '');

        // And the output should contain "Inconsistent help format in add-rule-help.ts: USAGE section is empty"
        expect(output).toMatch(
          /Inconsistent help format.*add-rule-help\.ts.*Missing USAGE section/
        );
      }
    });

    it('should detect missing EXAMPLES section', async () => {
      // Given a help file with missing EXAMPLES section
      const indexContent = `
import { registerAddRuleCommand } from './commands/add-rule';

program.addCommand(registerAddRuleCommand());
`;
      await writeFile(join(srcDir, 'index.ts'), indexContent);

      const addRuleHelp = `export function getAddRuleHelp(): string {
  return \`
USAGE
  fspec add-rule <work-unit-id> <rule-text>
\`;
}
`;
      await writeFile(join(commandsDir, 'add-rule-help.ts'), addRuleHelp);

      // When I run the validation script
      const scriptPath = join(
        process.cwd(),
        'scripts/validate-command-help.js'
      );

      try {
        await execFileAsync('node', [scriptPath, testDir]);
        expect.fail('Script should have exited with non-zero code');
      } catch (error: unknown) {
        const execError = error as {
          code: number;
          stdout: string;
          stderr: string;
        };
        // Then the script should exit with non-zero code
        expect(execError.code).toBe(1);

        const output = (execError.stdout || '') + (execError.stderr || '');

        // And the output should contain missing EXAMPLES section error
        expect(output).toMatch(
          /Inconsistent help format.*add-rule-help\.ts.*Missing EXAMPLES section/
        );
      }
    });

    it('should detect empty USAGE section', async () => {
      // Given a help file with empty USAGE section
      const indexContent = `
import { registerAddRuleCommand } from './commands/add-rule';

program.addCommand(registerAddRuleCommand());
`;
      await writeFile(join(srcDir, 'index.ts'), indexContent);

      const addRuleHelp = `export function getAddRuleHelp(): string {
  return \`
USAGE

EXAMPLES
  fspec add-rule WORK-001 "Rule text"
\`;
}
`;
      await writeFile(join(commandsDir, 'add-rule-help.ts'), addRuleHelp);

      // When I run the validation script
      const scriptPath = join(
        process.cwd(),
        'scripts/validate-command-help.js'
      );

      try {
        await execFileAsync('node', [scriptPath, testDir]);
        expect.fail('Script should have exited with non-zero code');
      } catch (error: unknown) {
        const execError = error as {
          code: number;
          stdout: string;
          stderr: string;
        };
        // Then the script should exit with non-zero code
        expect(execError.code).toBe(1);

        const output = (execError.stdout || '') + (execError.stderr || '');

        // And the output should contain empty USAGE section error
        expect(output).toMatch(
          /Inconsistent help format.*add-rule-help\.ts.*USAGE section is empty/
        );
      }
    });
  });

  describe('Scenario: Detect help file with incorrect export signature', () => {
    it('should detect missing export statement', async () => {
      // Given a file "src/commands/foo-help.ts" exists
      const indexContent = `
import { registerFooCommand } from './commands/foo';

program.addCommand(registerFooCommand());
`;
      await writeFile(join(srcDir, 'index.ts'), indexContent);

      // And the file does not export a function with correct signature
      const fooHelp = `
// No export statement
function getFooHelp(): string {
  return 'Help content';
}
`;
      await writeFile(join(commandsDir, 'foo-help.ts'), fooHelp);

      // When I run the validation script
      const scriptPath = join(
        process.cwd(),
        'scripts/validate-command-help.js'
      );

      try {
        await execFileAsync('node', [scriptPath, testDir]);
        expect.fail('Script should have exited with non-zero code');
      } catch (error: unknown) {
        const execError = error as {
          code: number;
          stdout: string;
          stderr: string;
        };
        // Then the script should exit with non-zero code
        expect(execError.code).toBe(1);

        const output = (execError.stdout || '') + (execError.stderr || '');

        // And the output should contain "Invalid help file export: foo-help.ts"
        expect(output).toMatch(/Invalid help file export.*foo-help\.ts/);

        // And the output should describe the expected function signature
        expect(output).toMatch(/missing export statement/i);
      }
    });
  });

  describe('Scenario: Multiple validation issues', () => {
    it('should report all issues grouped by type', async () => {
      // Given multiple commands with various issues
      const indexContent = `
import { registerAddRuleCommand } from './commands/add-rule';
import { registerValidateCommand } from './commands/validate';
import { registerMissingCommand } from './commands/missing';

program.addCommand(registerAddRuleCommand());
program.addCommand(registerValidateCommand());
program.addCommand(registerMissingCommand());
`;
      await writeFile(join(srcDir, 'index.ts'), indexContent);

      // Command with missing USAGE section
      const addRuleHelp = `export function getAddRuleHelp(): string {
  return \`
EXAMPLES
  fspec add-rule WORK-001 "Rule text"
\`;
}
`;
      await writeFile(join(commandsDir, 'add-rule-help.ts'), addRuleHelp);

      // Command with empty content
      await writeFile(join(commandsDir, 'validate-help.ts'), '');

      // missing command has no help file (don't create it)

      // Orphaned help file
      const orphanedHelp = `export function getOrphanedHelp(): string {
  return \`
USAGE
  fspec orphaned

EXAMPLES
  fspec orphaned
\`;
}
`;
      await writeFile(join(commandsDir, 'orphaned-help.ts'), orphanedHelp);

      // When I run the validation script
      const scriptPath = join(
        process.cwd(),
        'scripts/validate-command-help.js'
      );

      try {
        await execFileAsync('node', [scriptPath, testDir]);
        expect.fail('Script should have exited with non-zero code');
      } catch (error: unknown) {
        const execError = error as {
          code: number;
          stdout: string;
          stderr: string;
        };
        // Then the script should exit with non-zero code
        expect(execError.code).toBe(1);

        const output = (execError.stdout || '') + (execError.stderr || '');

        // And the output should group issues by type
        expect(output).toMatch(/MISSING HELP/);
        expect(output).toMatch(/ORPHANED HELP/);
        expect(output).toMatch(/INVALID EXPORT/);
        expect(output).toMatch(/FORMAT INCONSISTENCY/);

        // And the output should list all specific issues
        expect(output).toContain('missing-help.ts');
        expect(output).toContain('orphaned-help.ts');
        expect(output).toContain('validate-help.ts');
        expect(output).toContain('add-rule-help.ts');
      }
    });
  });

  describe('Scenario: Command with register- prefix', () => {
    it('should handle files with register- prefix', async () => {
      // Given commands with register- prefix in filename
      const indexContent = `
import { registerAddCapabilityCommand } from './commands/register-add-capability';

import { createTempTestDir, removeTempTestDir } from '../test-helpers/temp-directory';
program.addCommand(registerAddCapabilityCommand());
`;
      await writeFile(join(srcDir, 'index.ts'), indexContent);

      const helpContent = `export function getAddCapabilityHelp(): string {
  return \`
USAGE
  fspec add-capability <persona-id> <capability>

EXAMPLES
  fspec add-capability DEV-001 "Write unit tests"
\`;
}
`;
      await writeFile(join(commandsDir, 'add-capability-help.ts'), helpContent);

      // When I run the validation script
      const scriptPath = join(
        process.cwd(),
        'scripts/validate-command-help.js'
      );

      const result = await execFileAsync('node', [scriptPath, testDir]);

      // Then the script should exit with code 0
      expect(result.stderr).toBe('');

      // And the output should contain success message
      expect(result.stdout).toContain('All commands validated successfully');
    });
  });

  describe('Scenario: Large number of commands', () => {
    it('should handle validation of many commands efficiently', async () => {
      // Given 50 registered commands all with proper help files
      let imports = '';
      let commands = '';

      for (let i = 1; i <= 50; i++) {
        imports += `import { registerTest${i}Command } from './commands/test${i}';\n`;
        commands += `program.addCommand(registerTest${i}Command());\n`;

        const helpContent = `export function getTest${i}Help(): string {
  return \`
USAGE
  fspec test${i}

EXAMPLES
  fspec test${i}
\`;
}
`;
        await writeFile(join(commandsDir, `test${i}-help.ts`), helpContent);
      }

      await writeFile(join(srcDir, 'index.ts'), imports + '\n' + commands);

      // When I run the validation script
      const scriptPath = join(
        process.cwd(),
        'scripts/validate-command-help.js'
      );
      const startTime = Date.now();

      const result = await execFileAsync('node', [scriptPath, testDir]);
      const duration = Date.now() - startTime;

      // Then the script should complete quickly (under 5 seconds)
      expect(duration).toBeLessThan(5000);

      // And the output should show correct count
      expect(result.stdout).toMatch(/Found 50 registered commands/);
      expect(result.stdout).toContain('All commands validated successfully');
      expect(result.stdout).toMatch(/checked 50 commands/i);
    });
  });
});
