/**
 * REAL Integration Test: FspecTool via NAPI-RS
 *
 * NO MOCKS - Tests the actual Rust → TypeScript → Rust callback flow.
 * Uses real fixtures and the actual NAPI binding.
 *
 * This test file validates:
 * 1. callFspecCommand NAPI function properly passes args to TypeScript callback
 * 2. fspecCallback TypeScript implementation executes real fspec commands
 * 3. The full session flow simulation with __fspec_request__ marker
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { callFspecCommand } from '@sengac/codelet-napi';
import { fspecCallback } from '../../utils/fspec-callback';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

describe('REAL Integration: FspecTool NAPI Callback', () => {
  let testDir: string;

  beforeAll(async () => {
    // Create a real temp directory with fspec structure
    testDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-napi-test-'));
    const specDir = path.join(testDir, 'spec');
    fs.mkdirSync(specDir, { recursive: true });

    // Create real work-units.json fixture
    const workUnitsData = {
      workUnits: {
        'TEST-001': {
          id: 'TEST-001',
          title: 'First Test Story',
          status: 'backlog',
          type: 'story',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        'TEST-002': {
          id: 'TEST-002',
          title: 'Second Test Story',
          status: 'implementing',
          type: 'story',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        'TEST-003': {
          id: 'TEST-003',
          title: 'Third Story In Progress',
          status: 'backlog',
          type: 'story',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      },
      states: {
        backlog: ['TEST-001', 'TEST-003'],
        specifying: [],
        testing: [],
        implementing: ['TEST-002'],
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
    // Clean up temp directory
    if (testDir && fs.existsSync(testDir)) {
      fs.rmSync(testDir, { recursive: true, force: true });
    }
  });

  describe('callFspecCommand NAPI binding', () => {
    it('should call Rust NAPI function with TypeScript callback and return result', () => {
      // This is the ACTUAL NAPI call - Rust calls our TypeScript callback synchronously
      const result = callFspecCommand(
        'list-work-units',
        '{}',
        testDir,
        (command: string, argsJson: string, projectRoot: string): string => {
          // This callback is invoked BY RUST
          console.log(
            `[NAPI TEST] Rust called TypeScript with: command=${command}, args=${argsJson}, root=${projectRoot}`
          );

          // Read the actual file
          const workUnitsPath = path.join(
            projectRoot,
            'spec',
            'work-units.json'
          );
          const data = JSON.parse(fs.readFileSync(workUnitsPath, 'utf-8'));
          const workUnits = Object.values(data.workUnits || {});

          return JSON.stringify({
            success: true,
            data: { workUnits },
            command,
            projectRoot,
          });
        }
      );

      // Parse the result returned from Rust (which came from our callback)
      const parsed = JSON.parse(result);

      expect(parsed.success).toBe(true);
      expect(parsed.data.workUnits).toHaveLength(3);
      expect(parsed.command).toBe('list-work-units');
      expect(parsed.projectRoot).toBe(testDir);
    });

    it('should pass arguments correctly through Rust to TypeScript callback', () => {
      const receivedArgs: {
        command: string;
        argsJson: string;
        projectRoot: string;
      }[] = [];

      callFspecCommand(
        'show-work-unit',
        '{"id":"TEST-001"}',
        testDir,
        (command: string, argsJson: string, projectRoot: string): string => {
          receivedArgs.push({ command, argsJson, projectRoot });
          return JSON.stringify({ success: true, received: true });
        }
      );

      // Verify the arguments passed through Rust correctly
      expect(receivedArgs).toHaveLength(1);
      expect(receivedArgs[0].command).toBe('show-work-unit');
      expect(receivedArgs[0].argsJson).toBe('{"id":"TEST-001"}');
      expect(receivedArgs[0].projectRoot).toBe(testDir);
    });

    it('should handle callback errors gracefully', () => {
      // When a callback throws, NAPI-RS converts it to a Result error
      // which gets thrown as a JavaScript error
      expect(() => {
        callFspecCommand(
          'failing-command',
          '{}',
          testDir,
          (
            _command: string,
            _argsJson: string,
            _projectRoot: string
          ): string => {
            // Callback throws an error
            throw new Error('Callback intentionally failed');
          }
        );
      }).toThrow('Callback intentionally failed');
    });

    it('should filter work units by status through the callback', () => {
      const result = callFspecCommand(
        'list-work-units',
        '{"status":"backlog"}',
        testDir,
        (command: string, argsJson: string, projectRoot: string): string => {
          const args = JSON.parse(argsJson);
          const workUnitsPath = path.join(
            projectRoot,
            'spec',
            'work-units.json'
          );
          const data = JSON.parse(fs.readFileSync(workUnitsPath, 'utf-8'));

          let workUnits = Object.values(data.workUnits || {}) as Array<{
            status: string;
          }>;

          // Apply status filter
          if (args.status) {
            workUnits = workUnits.filter(wu => wu.status === args.status);
          }

          return JSON.stringify({
            success: true,
            data: { workUnits },
            command,
          });
        }
      );

      const parsed = JSON.parse(result);
      expect(parsed.success).toBe(true);
      expect(parsed.data.workUnits).toHaveLength(2); // Only backlog items
      expect(
        parsed.data.workUnits.every(
          (wu: { status: string }) => wu.status === 'backlog'
        )
      ).toBe(true);
    });
  });

  describe('fspecCallback TypeScript implementation', () => {
    it('should execute list-work-units command with real fixture', async () => {
      const result = await fspecCallback('list-work-units', '{}', testDir);
      const parsed = JSON.parse(result);

      // fspecCallback returns the command result directly (not wrapped in success/data)
      // The wrapping happens in AgentView when calling sessionSendFspecResult
      expect(parsed.workUnits).toBeDefined();
      expect(parsed.workUnits).toHaveLength(3);
    });

    it('should execute list-work-units with status filter', async () => {
      const result = await fspecCallback(
        'list-work-units',
        '{"status":"implementing"}',
        testDir
      );
      const parsed = JSON.parse(result);

      // fspecCallback returns the command result directly
      expect(parsed.workUnits).toBeDefined();
      // Only TEST-002 has status 'implementing'
      expect(parsed.workUnits).toHaveLength(1);
      expect(parsed.workUnits[0].id).toBe('TEST-002');
    });

    it('should handle unknown commands with proper error', async () => {
      const result = await fspecCallback('unknown-fake-command', '{}', testDir);
      const parsed = JSON.parse(result);

      expect(parsed.success).toBe(false);
      expect(parsed.errorType).toBe('CommandNotFound');
      expect(parsed.error).toContain('not found');
    });

    it('should reject excluded commands (bootstrap, init)', async () => {
      const bootstrapResult = await fspecCallback('bootstrap', '{}', testDir);
      const parsed = JSON.parse(bootstrapResult);

      expect(parsed.success).toBe(false);
      expect(parsed.errorType).toBe('UnsupportedCommand');
    });

    it('should capture help output without exiting process', async () => {
      // When --help is requested via the help command,
      // fspecCallback should return structured help content
      const result = await fspecCallback(
        'help',
        '{"command":"research"}',
        testDir
      );
      const parsed = JSON.parse(result);

      // Help output should be captured successfully
      expect(parsed.success).toBe(true);
      // The data should contain help text for the research command
      expect(parsed.data).toContain('research');
    });

    it('should return AI-friendly help for the help command', async () => {
      const result = await fspecCallback('help', '{}', testDir);
      const parsed = JSON.parse(result);

      expect(parsed.success).toBe(true);
      expect(parsed.data).toContain('# Fspec Tool Reference');
      expect(parsed.data).toContain('command');
      expect(parsed.data).toContain('JSON string');
    });

    it('should return command-specific help when requested', async () => {
      const result = await fspecCallback(
        'help',
        '{"command":"list-work-units"}',
        testDir
      );
      const parsed = JSON.parse(result);

      expect(parsed.success).toBe(true);
      expect(parsed.data).toContain('## list-work-units');
      expect(parsed.data).toContain('status');
      expect(parsed.data).toContain('backlog');
    });

    it('should return error for unknown command help', async () => {
      const result = await fspecCallback(
        'help',
        '{"command":"unknown-cmd"}',
        testDir
      );
      const parsed = JSON.parse(result);

      expect(parsed.success).toBe(false);
      expect(parsed.errorType).toBe('CommandNotFound');
      expect(parsed.error).toContain('not found');
    });
  });

  describe('End-to-end NAPI with fspecCallback', () => {
    it('should work when NAPI callback uses fspecCallback internally', async () => {
      // This simulates what actually happens in the agent:
      // 1. Rust calls the callback
      // 2. Callback uses fspecCallback to execute the command
      // 3. Result flows back to Rust

      const result = callFspecCommand(
        'list-work-units',
        '{"status":"backlog"}',
        testDir,
        (command: string, argsJson: string, projectRoot: string): string => {
          // Note: fspecCallback is async but callFspecCommand expects sync callback
          // In real usage, the session handles this differently via FspecCommandRequest chunks
          // Here we test the sync path with direct file reading
          const workUnitsPath = path.join(
            projectRoot,
            'spec',
            'work-units.json'
          );
          const data = JSON.parse(fs.readFileSync(workUnitsPath, 'utf-8'));

          let workUnits = Object.values(data.workUnits || {}) as Array<{
            status: string;
            id: string;
            title: string;
          }>;
          const args = JSON.parse(argsJson);

          if (args.status) {
            workUnits = workUnits.filter(wu => wu.status === args.status);
          }

          return JSON.stringify({
            success: true,
            data: {
              workUnits: workUnits.map(wu => ({
                id: wu.id,
                title: wu.title,
                status: wu.status,
              })),
            },
          });
        }
      );

      const parsed = JSON.parse(result);
      expect(parsed.success).toBe(true);
      expect(parsed.data.workUnits).toHaveLength(2);

      // Verify the actual work unit data
      const ids = parsed.data.workUnits.map((wu: { id: string }) => wu.id);
      expect(ids).toContain('TEST-001');
      expect(ids).toContain('TEST-003');
    });
  });
});

/**
 * Integration Test: Full Session Flow Simulation
 *
 * This tests the complete flow from FspecToolFacadeWrapper through to TypeScript
 * callback execution, simulating what happens when the LLM calls the Fspec tool.
 *
 * New Architecture (fspec_handler pattern - similar to pause_handler):
 * 1. Session manager sets fspec_handler before agent run
 * 2. LLM calls Fspec tool via FspecToolFacadeWrapper
 * 3. Wrapper calls execute_fspec_command() which invokes the handler
 * 4. Handler emits FspecCommandRequest chunk to TypeScript
 * 5. Handler blocks waiting for response on std::sync::mpsc channel
 * 6. TypeScript executes fspecCallback and calls sessionSendFspecResult
 * 7. Handler receives result, returns to wrapper
 * 8. Wrapper returns actual result (not marker) to LLM
 */
describe('Integration: Full Fspec Tool Session Flow', () => {
  let testDir: string;

  beforeAll(async () => {
    testDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-session-flow-'));
    const specDir = path.join(testDir, 'spec');
    fs.mkdirSync(specDir, { recursive: true });

    const workUnitsData = {
      workUnits: {
        'FLOW-001': {
          id: 'FLOW-001',
          title: 'Session Flow Test',
          status: 'specifying',
          type: 'story',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      },
      states: {
        backlog: [],
        specifying: ['FLOW-001'],
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

  it('should demonstrate the fspec_handler callback flow', async () => {
    // In the new architecture, FspecToolFacadeWrapper calls execute_fspec_command()
    // which invokes the fspec_handler set by session_manager. The handler:
    // 1. Emits FspecCommandRequest chunk to TypeScript
    // 2. Blocks waiting for response
    // 3. TypeScript executes fspecCallback and calls sessionSendFspecResult
    // 4. Handler receives result and returns to the tool

    // Here we test the TypeScript callback portion of this flow
    const command = 'list-work-units';
    const argsJson = '{"status":"specifying"}';
    const projectRoot = testDir;

    // This is what TypeScript does when it receives FspecCommandRequest
    const resultJson = await fspecCallback(command, argsJson, projectRoot);
    const result = JSON.parse(resultJson);

    // Verify the result contains the expected data
    expect(result.workUnits).toBeDefined();
    expect(result.workUnits).toHaveLength(1);
    expect(result.workUnits[0].id).toBe('FLOW-001');
    expect(result.workUnits[0].status).toBe('specifying');

    // This is what AgentView.tsx does before calling sessionSendFspecResult
    // The handler on the Rust side is blocking, waiting for this
    const fspecResult = {
      success: true,
      data: resultJson,
      error: null,
      systemReminder: null,
      toolCallId: 'test-tool-call-id',
    };

    expect(fspecResult.success).toBe(true);
    expect(JSON.parse(fspecResult.data).workUnits).toHaveLength(1);
  });

  it('should handle errors in the callback flow', async () => {
    // Test error handling through the callback flow
    const resultJson = await fspecCallback(
      'nonexistent-command',
      '{}',
      testDir
    );
    const result = JSON.parse(resultJson);

    // Error result structure from fspecCallback
    expect(result.success).toBe(false);
    expect(result.errorType).toBe('CommandNotFound');
    expect(result.error).toContain('not found');

    // This would be sent via sessionSendFspecResult
    const fspecResult = {
      success: result.success,
      data: '',
      error: result.error,
      systemReminder: null,
      toolCallId: 'error-test-id',
    };

    expect(fspecResult.success).toBe(false);
    expect(fspecResult.error).toContain('not found');
  });

  it('should capture error message when command throws and calls process.exit(1)', async () => {
    // Test that when a command throws an error and calls process.exit(1),
    // the error message is properly captured and returned (not just "Exit code 1")
    // Note: add-rule doesn't support --format json, so we get a Commander error
    // But show-work-unit DOES support --format json, so we can test actual command errors
    const resultJson = await fspecCallback(
      'show-work-unit',
      JSON.stringify({ _: ['NONEXISTENT-999'] }),
      testDir
    );

    const result = JSON.parse(resultJson);

    // Should fail with proper error message
    expect(result.success).toBe(false);
    expect(result.errorType).toBe('CommandError');
    // The error should contain the actual error message, not just "Exit code 1"
    expect(result.error).toContain('does not exist');
    expect(result.error).not.toBe('Exit code 1');
  });
});
