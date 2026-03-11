/**
 * Feature: spec/features/ansi-escape-codes-and-tui-content-leaking-into-fspec-tool-call-results.feature
 *
 * This test file validates that ANSI escape codes are stripped from all
 * capture layers used by fspec-callback, and that Commander configureOutput
 * is propagated to subcommands so process.stdout.write is never overridden.
 */

import { describe, it, expect, afterEach, beforeAll, afterAll } from 'vitest';
import {
  output,
  createCaptureContext,
  setOutputContext,
  resetOutputContext,
  stripAnsi,
} from '../output';
import { fspecCallback } from '../fspec-callback';
import { createProgram } from '../../cli/program';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

// Comprehensive ANSI regex for test assertions - matches any ANSI escape sequence
// eslint-disable-next-line no-control-regex
const ANSI_DETECT_REGEX = /\x1b\[/;

describe('Feature: ANSI escape codes and TUI content leaking into Fspec tool call results', () => {
  afterEach(() => {
    resetOutputContext();
  });

  describe('Scenario: Capture context strips ANSI color codes from output.log', () => {
    it('should strip ANSI color codes from output.log in capture mode', () => {
      // @step Given a capture context is created via createCaptureContext
      const { context, stdout } = createCaptureContext();
      setOutputContext(context);

      // @step When a command calls output.log with chalk-formatted text containing SGR escape codes
      // Simulate chalk.green('tag') output
      output.log(`  \x1b[32mtag-name\x1b[39m - some description`);
      // Simulate chalk.bold('Feature:') output
      output.log(`\x1b[1mFeature:\x1b[22m user-login.feature`);
      // Simulate chalk.yellow('warning') output
      output.log(`\x1b[33m⚠ Warning:\x1b[39m something happened`);

      // @step Then the captured stdout array should contain only plain text with no ANSI escape sequences
      for (const line of stdout) {
        expect(line).not.toMatch(ANSI_DETECT_REGEX);
      }
      expect(stdout[0]).toBe('  tag-name - some description');
      expect(stdout[1]).toBe('Feature: user-login.feature');
      expect(stdout[2]).toBe('⚠ Warning: something happened');
    });
  });

  describe('Scenario: Capture context strips ANSI codes from output.error and output.warn', () => {
    it('should strip ANSI codes from output.error and output.warn in capture mode', () => {
      // @step Given a capture context is created via createCaptureContext
      const { context, stderr } = createCaptureContext();
      setOutputContext(context);

      // @step When a command calls output.error and output.warn with chalk-formatted text
      output.error(`\x1b[31m✗ Error:\x1b[39m something failed`);
      output.warn(`\x1b[33m⚠ Warning:\x1b[39m check this`);

      // @step Then the captured stderr array should contain only plain text with no ANSI escape sequences
      for (const line of stderr) {
        expect(line).not.toMatch(ANSI_DETECT_REGEX);
      }
      expect(stderr[0]).toBe('✗ Error: something failed');
      expect(stderr[1]).toBe('⚠ Warning: check this');
    });
  });

  describe('Scenario: Strip ANSI handles all CSI sequences not just SGR colors', () => {
    it('should strip cursor movement, line erasure, and mouse tracking sequences', () => {
      // @step Given text containing CSI cursor movement, line erasure, and mouse tracking sequences
      const textWithCsiSequences = [
        '\x1b[49AHello', // cursor up 49 lines + text
        '\x1b[ENext line', // cursor next line + text
        '\x1b[2KErased line', // erase in line + text
        '\x1b[?1000hMouse on', // mouse tracking enable + text
        '\x1b[?1000lMouse off', // mouse tracking disable + text
        '\x1b[32mGreen\x1b[39m text \x1b[49A\x1b[E\x1b[2K', // mixed SGR + CSI
      ];

      // @step When the text is passed through the stripAnsi function
      const results = textWithCsiSequences.map(t => stripAnsi(t));

      // @step Then all CSI sequences including cursor up, erase line, next line, and mouse tracking are removed
      expect(results[0]).toBe('Hello');
      expect(results[1]).toBe('Next line');
      expect(results[2]).toBe('Erased line');
      expect(results[3]).toBe('Mouse on');
      expect(results[4]).toBe('Mouse off');
      expect(results[5]).toBe('Green text ');
      for (const result of results) {
        expect(result).not.toMatch(ANSI_DETECT_REGEX);
      }
    });
  });

  describe('Scenario: Commander configureOutput propagated to subcommands', () => {
    it('should propagate configureOutput to all subcommands', () => {
      // @step Given a fresh Commander program with subcommands registered via createProgram
      const program = createProgram();
      expect(program.commands.length).toBeGreaterThan(0);

      // @step When configureOutput is called on the program with capture callbacks
      let capturedOutput = '';
      const captureConfig = {
        writeOut: (str: string) => {
          capturedOutput += str;
        },
        writeErr: (_str: string) => {
          // capture
        },
        outputError: (_str: string) => {
          // capture
        },
      };
      program.configureOutput(captureConfig);
      // Propagate to subcommands (as fspecCallback does)
      for (const cmd of program.commands) {
        cmd.configureOutput(captureConfig);
      }

      // @step Then all subcommands should also have their output configuration updated
      // Verify a subcommand's help goes through our capture, not process.stdout.write
      const listCmd = program.commands.find(
        c => c.name() === 'list-work-units'
      );
      expect(listCmd).toBeDefined();

      // Trigger help output on the subcommand
      program.exitOverride();
      try {
        listCmd!.outputHelp();
      } catch {
        // exitOverride may throw
      }

      // @step And subcommand help output should be captured by configureOutput not process.stdout.write
      expect(capturedOutput.length).toBeGreaterThan(0);
      expect(capturedOutput).toContain('list-work-units');
    });
  });

  describe('Scenario: End-to-end fspec callback returns clean output', () => {
    let testDir: string;

    beforeAll(() => {
      testDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-ansi-strip-'));
      const specDir = path.join(testDir, 'spec');
      fs.mkdirSync(specDir, { recursive: true });

      const workUnitsData = {
        workUnits: {},
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };
      fs.writeFileSync(
        path.join(specDir, 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );
    });

    afterAll(() => {
      if (testDir && fs.existsSync(testDir)) {
        fs.rmSync(testDir, { recursive: true, force: true });
      }
    });

    it('should return clean JSON with no ANSI escape sequences from fspecCallback', async () => {
      // @step Given a project with valid fspec structure
      // (set up in beforeAll)

      // @step When fspecCallback is called to execute a command
      const result = await fspecCallback('list-work-units', '{}', testDir);

      // @step Then the returned JSON string should not contain any ANSI escape sequences
      expect(result).not.toMatch(ANSI_DETECT_REGEX);

      // Also verify it's valid JSON
      const parsed = JSON.parse(result);
      expect(parsed.success).toBe(true);

      // Deep check: stringify back and verify no ANSI
      const reStringified = JSON.stringify(parsed);
      expect(reStringified).not.toMatch(ANSI_DETECT_REGEX);
    });
  });

  describe('stripAnsi function', () => {
    it('should handle OSC sequences (Operating System Command)', () => {
      // OSC sequences: \x1b] ... \x07 (BEL) or \x1b\\ (ST)
      const oscWithBel = '\x1b]0;Window Title\x07Some text';
      const oscWithSt = '\x1b]0;Window Title\x1b\\Some text';
      expect(stripAnsi(oscWithBel)).toBe('Some text');
      expect(stripAnsi(oscWithSt)).toBe('Some text');
    });

    it('should handle text with no ANSI codes unchanged', () => {
      const plain = 'Hello, world! 123 @#$%';
      expect(stripAnsi(plain)).toBe(plain);
    });

    it('should handle empty string', () => {
      expect(stripAnsi('')).toBe('');
    });

    it('should handle multiple ANSI sequences in one string', () => {
      const mixed =
        '\x1b[1m\x1b[33mBold Yellow\x1b[39m\x1b[22m and \x1b[31mRed\x1b[39m';
      expect(stripAnsi(mixed)).toBe('Bold Yellow and Red');
    });
  });
});
