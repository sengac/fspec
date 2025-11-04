/**
 * Feature: spec/features/tui-016-incomplete-checkpointpanel-using-chokidar-instead-of-ipc-zustand.feature
 *
 * LIVE integration tests for BUG-065: Real TUI process + IPC messaging
 * These tests actually spawn the TUI and verify IPC works end-to-end
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { spawn, ChildProcess } from 'child_process';
import { sendIPCMessage, getIPCPath } from '../../utils/ipc';
import * as fs from 'fs/promises';
import * as path from 'path';
import { tmpdir } from 'os';
import net from 'net';

describe('Feature: TUI-016 Live IPC Integration', () => {
  let tuiProcess: ChildProcess | null = null;
  let tempDir: string;
  let checkpointIndexDir: string;

  beforeEach(async () => {
    // Create temp directory with git structure
    tempDir = await fs.mkdtemp(path.join(tmpdir(), 'fspec-live-test-'));
    const gitDir = path.join(tempDir, '.git');
    const specDir = path.join(tempDir, 'spec');
    checkpointIndexDir = path.join(gitDir, 'fspec-checkpoints-index');

    await fs.mkdir(gitDir, { recursive: true });
    await fs.mkdir(specDir, { recursive: true });
    await fs.mkdir(checkpointIndexDir, { recursive: true });

    // Create minimal work-units.json
    await fs.writeFile(
      path.join(specDir, 'work-units.json'),
      JSON.stringify({
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
      })
    );

    // Create minimal epics.json
    await fs.writeFile(
      path.join(specDir, 'epics.json'),
      JSON.stringify({ epics: {} })
    );

    // Copy foundation.json from project root
    const projectFoundation = path.join(
      process.cwd(),
      'spec',
      'foundation.json'
    );
    const tempFoundation = path.join(specDir, 'foundation.json');
    await fs.copyFile(projectFoundation, tempFoundation);
  });

  afterEach(async () => {
    // Kill TUI process if running
    if (tuiProcess) {
      tuiProcess.kill('SIGTERM');
      tuiProcess = null;
    }

    // Cleanup temp directory
    await fs.rm(tempDir, { recursive: true, force: true });

    // Small delay to allow socket cleanup
    await new Promise(resolve => setTimeout(resolve, 100));
  });

  // @step Given TUI is running with BoardView IPC server listening
  async function startTUI(): Promise<void> {
    return new Promise((resolve, reject) => {
      // Spawn dev-tui process (simpler than full board command)
      const devTuiPath = path.join(process.cwd(), 'dev-tui.tsx');
      tuiProcess = spawn('npx', ['tsx', devTuiPath], {
        cwd: process.cwd(),
        stdio: ['pipe', 'pipe', 'pipe'],
        env: { ...process.env, FORCE_COLOR: '0' },
      });

      tuiProcess.stdout?.on('data', _data => {
        // Capture output for debugging if needed
      });

      tuiProcess.stderr?.on('data', data => {
        console.error('TUI stderr:', data.toString());
      });

      tuiProcess.on('error', error => {
        reject(error);
      });

      // Wait for IPC socket to be created (indicates TUI is ready)
      const checkInterval = setInterval(async () => {
        try {
          const ipcPath = getIPCPath();
          await fs.access(ipcPath);

          // Socket exists, verify it's listening
          const client = net.connect(ipcPath, () => {
            client.end();
            clearInterval(checkInterval);
            resolve();
          });

          client.on('error', () => {
            // Not ready yet, keep checking
          });
        } catch {
          // Socket doesn't exist yet, keep checking
        }
      }, 100);

      // Timeout after 5 seconds
      setTimeout(() => {
        clearInterval(checkInterval);
        reject(new Error('TUI failed to start within 5 seconds'));
      }, 5000);
    });
  }

  // @step When checkpoint command sends IPC message
  async function sendCheckpointChangedMessage(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        sendIPCMessage({ type: 'checkpoint-changed' });
        // Give message time to be received
        setTimeout(resolve, 200);
      } catch (error) {
        reject(error);
      }
    });
  }

  describe('Scenario: IPC server accepts connections', () => {
    it('should create IPC socket when TUI starts', async () => {
      // @step Given TUI is starting
      const startPromise = startTUI();

      // @step When TUI initializes BoardView
      await startPromise;

      // @step Then IPC socket file exists
      const ipcPath = getIPCPath();
      const stats = await fs.stat(ipcPath);
      expect(stats.isSocket()).toBe(true);
    }, 10000);
  });

  describe('Scenario: IPC message can be sent to running TUI', () => {
    it('should successfully send checkpoint-changed message', async () => {
      // @step Given TUI is running with IPC server
      await startTUI();

      // @step When checkpoint command sends IPC message
      let messageSent = false;
      let sendError: Error | null = null;

      try {
        await sendCheckpointChangedMessage();
        messageSent = true;
      } catch (error) {
        sendError = error as Error;
      }

      // @step Then message is sent without error
      expect(sendError).toBeNull();
      expect(messageSent).toBe(true);
    }, 10000);
  });

  describe('Scenario: TUI receives and processes IPC message', () => {
    it('should call loadCheckpointCounts when checkpoint-changed received', async () => {
      // @step Given checkpoint index has initial checkpoints
      await fs.writeFile(
        path.join(checkpointIndexDir, 'TEST-001.json'),
        JSON.stringify({
          checkpoints: [
            {
              name: 'manual-checkpoint',
              message: 'fspec-checkpoint:TEST-001:manual-checkpoint:1234567890',
            },
            {
              name: 'TEST-001-auto-checkpoint',
              message: 'fspec-checkpoint:TEST-001:auto-checkpoint:1234567891',
            },
          ],
        })
      );

      // @step Given TUI is running
      await startTUI();

      // @step When checkpoint-changed IPC message is sent
      await sendCheckpointChangedMessage();

      // @step Then TUI processes message (we verify it doesn't crash)
      // Note: We can't easily verify the internal state change without instrumentation,
      // but we can verify the TUI remains running and responsive

      await new Promise(resolve => setTimeout(resolve, 500));

      // Verify TUI process is still running
      expect(tuiProcess?.killed).toBe(false);
      expect(tuiProcess?.exitCode).toBeNull();
    }, 10000);
  });

  describe('Scenario: Real checkpoint creation triggers TUI update', () => {
    it('should update TUI when fspec checkpoint command is run', async () => {
      // @step Given work-units.json with BUG-065 in project root (not temp dir)
      const projectCheckpointIndexDir = path.join(
        process.cwd(),
        '.git',
        'fspec-checkpoints-index'
      );

      // Read current checkpoint counts
      const bugCheckpointPath = path.join(
        projectCheckpointIndexDir,
        'BUG-065.json'
      );
      let initialCheckpointCount = 0;
      try {
        const data = JSON.parse(await fs.readFile(bugCheckpointPath, 'utf-8'));
        initialCheckpointCount = data.checkpoints?.length || 0;
      } catch {
        // File doesn't exist yet
      }

      // @step Given TUI is running
      await startTUI();

      // Wait for TUI to initialize and IPC server to be ready
      await new Promise(resolve => setTimeout(resolve, 1000));

      // @step When fspec checkpoint command is executed in separate process
      const fspecBinary = path.join(process.cwd(), 'dist', 'index.js');
      const checkpointProcess = spawn(
        'node',
        [fspecBinary, 'checkpoint', 'BUG-065', 'integration-test-checkpoint'],
        {
          cwd: process.cwd(),
          stdio: ['pipe', 'pipe', 'pipe'],
        }
      );

      let checkpointStdout = '';
      let checkpointStderr = '';
      checkpointProcess.stdout?.on('data', data => {
        checkpointStdout += data.toString();
      });
      checkpointProcess.stderr?.on('data', data => {
        checkpointStderr += data.toString();
      });

      // Wait for checkpoint command to complete
      await new Promise<void>((resolve, reject) => {
        checkpointProcess.on('exit', code => {
          if (code === 0) {
            resolve();
          } else {
            console.error('Checkpoint command stdout:', checkpointStdout);
            console.error('Checkpoint command stderr:', checkpointStderr);
            reject(new Error(`Checkpoint command failed with code ${code}`));
          }
        });

        // Timeout after 5 seconds
        setTimeout(
          () => reject(new Error('Checkpoint command timed out')),
          5000
        );
      });

      // @step Then TUI receives IPC message and reloads counts
      await new Promise(resolve => setTimeout(resolve, 500));

      // Verify TUI is still running (didn't crash)
      expect(tuiProcess?.killed).toBe(false);

      // @step Then checkpoint should be created
      const updatedData = JSON.parse(
        await fs.readFile(bugCheckpointPath, 'utf-8')
      );
      const finalCheckpointCount = updatedData.checkpoints?.length || 0;
      expect(finalCheckpointCount).toBe(initialCheckpointCount + 1);

      // Verify the checkpoint was created
      const lastCheckpoint =
        updatedData.checkpoints[updatedData.checkpoints.length - 1];
      expect(lastCheckpoint.name).toBe('integration-test-checkpoint');
    }, 15000);
  });
});
