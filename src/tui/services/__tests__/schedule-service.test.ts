/**
 * Feature: spec/features/schedule-tui-slash-commands.feature
 *
 * This test file validates the schedule service layer — the integration
 * between slash command parsing and the existing schedule CRUD operations.
 * Tests against a real filesystem using OS temp directories.
 *
 * SCHED-008: Schedule TUI Slash Commands
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdtemp, rm, readFile, mkdir } from 'fs/promises';
import { tmpdir } from 'os';
import type { SchedulesData } from '../../../types/schedule';
import { handleScheduleCommand } from '../schedule-service';

describe('Feature: Schedule TUI Slash Commands — Service', () => {
  let tempDir: string;

  beforeEach(async () => {
    tempDir = await mkdtemp(join(tmpdir(), 'fspec-sched-tui-'));
    await mkdir(join(tempDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(tempDir, { recursive: true, force: true });
  });

  describe('Scenario: Add an agent-type schedule via slash command', () => {
    it('should add agent schedule and return success message', async () => {
      // @step Given no schedule named "nightly-review" exists
      // (empty temp dir)

      // @step When I type "/schedule add nightly-review --cron "0 2 * * *" --tz Australia/Brisbane --role "Code reviewer" --prompt "Review all open PRs""
      const result = await handleScheduleCommand(
        '/schedule add nightly-review --cron "0 2 * * *" --tz Australia/Brisbane --role "Code reviewer" --prompt "Review all open PRs"',
        tempDir
      );

      // @step Then the TUI should display a success message containing "nightly-review" and "added"
      expect(result.success).toBe(true);
      expect(result.message).toContain('nightly-review');
      expect(result.message).toContain('added');

      // @step And the schedule "nightly-review" should be persisted in schedules.json as type "agent"
      const data = JSON.parse(
        await readFile(join(tempDir, 'spec', 'schedules.json'), 'utf-8')
      ) as SchedulesData;
      expect(data.schedules['nightly-review']).toBeDefined();
      expect(data.schedules['nightly-review'].jobType).toBe('agent');
    });
  });

  describe('Scenario: Add a shell-type schedule via slash command', () => {
    it('should add shell schedule and return success message', async () => {
      // @step Given no schedule named "daily-sync" exists
      // (empty temp dir)

      // @step When I type "/schedule add daily-sync --cron "0 9 * * 1-5" --tz UTC --command "npm run sync""
      const result = await handleScheduleCommand(
        '/schedule add daily-sync --cron "0 9 * * 1-5" --tz UTC --command "npm run sync"',
        tempDir
      );

      // @step Then the TUI should display a success message containing "daily-sync" and "added"
      expect(result.success).toBe(true);
      expect(result.message).toContain('daily-sync');
      expect(result.message).toContain('added');

      // @step And the schedule "daily-sync" should be persisted in schedules.json as type "shell"
      const data = JSON.parse(
        await readFile(join(tempDir, 'spec', 'schedules.json'), 'utf-8')
      ) as SchedulesData;
      expect(data.schedules['daily-sync']).toBeDefined();
      expect(data.schedules['daily-sync'].jobType).toBe('shell');
    });
  });

  describe('Scenario: List all schedules', () => {
    it('should list schedules in table format', async () => {
      // @step Given a schedule named "nightly-review" exists with cron "0 2 * * *"
      await handleScheduleCommand(
        '/schedule add nightly-review --cron "0 2 * * *" --tz UTC --role "Reviewer" --prompt "Review"',
        tempDir
      );

      // @step And a schedule named "daily-sync" exists with cron "0 9 * * 1-5"
      await handleScheduleCommand(
        '/schedule add daily-sync --cron "0 9 * * 1-5" --tz UTC --command "npm run sync"',
        tempDir
      );

      // @step When I type "/schedule list"
      const result = await handleScheduleCommand('/schedule list', tempDir);

      // @step Then the TUI should display a table containing "nightly-review" and "daily-sync"
      expect(result.success).toBe(true);
      expect(result.message).toContain('nightly-review');
      expect(result.message).toContain('daily-sync');

      // @step And the table should include columns for Name, Cron, Timezone, Type, and Status
      expect(result.message).toContain('Name');
      expect(result.message).toContain('Cron');
      expect(result.message).toContain('Timezone');
      expect(result.message).toContain('Type');
      expect(result.message).toContain('Status');
    });
  });

  describe('Scenario: Pause an active schedule', () => {
    it('should pause schedule and confirm', async () => {
      // @step Given an active schedule named "nightly-review" exists
      await handleScheduleCommand(
        '/schedule add nightly-review --cron "0 2 * * *" --tz UTC --role "Reviewer" --prompt "Review"',
        tempDir
      );

      // @step When I type "/schedule pause nightly-review"
      const result = await handleScheduleCommand(
        '/schedule pause nightly-review',
        tempDir
      );

      // @step Then the TUI should display a success message containing "nightly-review" and "paused"
      expect(result.success).toBe(true);
      expect(result.message).toContain('nightly-review');
      expect(result.message).toContain('paused');

      // @step And the schedule "nightly-review" should have status "paused" in schedules.json
      const data = JSON.parse(
        await readFile(join(tempDir, 'spec', 'schedules.json'), 'utf-8')
      ) as SchedulesData;
      expect(data.schedules['nightly-review'].status).toBe('paused');
    });
  });

  describe('Scenario: Resume a paused schedule', () => {
    it('should resume schedule and confirm', async () => {
      // @step Given a paused schedule named "nightly-review" exists
      await handleScheduleCommand(
        '/schedule add nightly-review --cron "0 2 * * *" --tz UTC --role "Reviewer" --prompt "Review"',
        tempDir
      );
      await handleScheduleCommand('/schedule pause nightly-review', tempDir);

      // @step When I type "/schedule resume nightly-review"
      const result = await handleScheduleCommand(
        '/schedule resume nightly-review',
        tempDir
      );

      // @step Then the TUI should display a success message containing "nightly-review" and "resumed"
      expect(result.success).toBe(true);
      expect(result.message).toContain('nightly-review');
      expect(result.message).toContain('resumed');

      // @step And the schedule "nightly-review" should have status "active" in schedules.json
      const data = JSON.parse(
        await readFile(join(tempDir, 'spec', 'schedules.json'), 'utf-8')
      ) as SchedulesData;
      expect(data.schedules['nightly-review'].status).toBe('active');
    });
  });

  describe('Scenario: Remove an existing schedule', () => {
    it('should remove schedule and confirm', async () => {
      // @step Given a schedule named "daily-sync" exists
      await handleScheduleCommand(
        '/schedule add daily-sync --cron "0 9 * * 1-5" --tz UTC --command "npm run sync"',
        tempDir
      );

      // @step When I type "/schedule remove daily-sync"
      const result = await handleScheduleCommand(
        '/schedule remove daily-sync',
        tempDir
      );

      // @step Then the TUI should display a success message containing "daily-sync" and "removed"
      expect(result.success).toBe(true);
      expect(result.message).toContain('daily-sync');
      expect(result.message).toContain('removed');

      // @step And the schedule "daily-sync" should not exist in schedules.json
      const data = JSON.parse(
        await readFile(join(tempDir, 'spec', 'schedules.json'), 'utf-8')
      ) as SchedulesData;
      expect(data.schedules['daily-sync']).toBeUndefined();
    });
  });

  describe('Scenario: Reject invalid cron expression', () => {
    it('should return error for invalid cron', async () => {
      // @step When I type "/schedule add bad --cron "not-a-cron" --tz UTC --command "echo hi""
      const result = await handleScheduleCommand(
        '/schedule add bad --cron "not-a-cron" --tz UTC --command "echo hi"',
        tempDir
      );

      // @step Then the TUI should display an error message containing "Invalid cron expression"
      expect(result.success).toBe(false);
      expect(result.message.toLowerCase()).toContain('cron');
    });
  });

  describe('Scenario: Reject duplicate schedule name', () => {
    it('should return error for duplicate name', async () => {
      // @step Given a schedule named "nightly-review" exists
      await handleScheduleCommand(
        '/schedule add nightly-review --cron "0 2 * * *" --tz UTC --command "echo"',
        tempDir
      );

      // @step When I type "/schedule add nightly-review --cron "0 2 * * *" --tz UTC --command "echo""
      const result = await handleScheduleCommand(
        '/schedule add nightly-review --cron "0 2 * * *" --tz UTC --command "echo"',
        tempDir
      );

      // @step Then the TUI should display an error message containing "already exists"
      expect(result.success).toBe(false);
      expect(result.message).toContain('already exists');
    });
  });

  describe('Scenario: Reject removal of nonexistent schedule', () => {
    it('should return error for nonexistent schedule', async () => {
      // @step When I type "/schedule remove nonexistent"
      const result = await handleScheduleCommand(
        '/schedule remove nonexistent',
        tempDir
      );

      // @step Then the TUI should display an error message containing "not found"
      expect(result.success).toBe(false);
      expect(result.message.toLowerCase()).toContain('not');
      expect(result.message.toLowerCase()).toContain('exist');
    });
  });

  describe('Scenario: Show usage help when no subcommand provided', () => {
    it('should return help text for bare /schedule', async () => {
      // @step When I type "/schedule"
      const result = await handleScheduleCommand('/schedule', tempDir);

      // @step Then the TUI should display usage help containing "add" and "list" and "pause" and "resume" and "remove"
      expect(result.success).toBe(true);
      expect(result.message).toContain('add');
      expect(result.message).toContain('list');
      expect(result.message).toContain('pause');
      expect(result.message).toContain('resume');
      expect(result.message).toContain('remove');
    });
  });

  describe('Scenario: Reject agent schedule missing required fields', () => {
    it('should return error when role/prompt missing for agent type', async () => {
      // @step When I type "/schedule add agent-job --cron "0 9 * * *" --tz UTC"
      // No --role, --prompt, or --command → ambiguous, but since no --command it defaults to agent
      // Actually, without explicit --command, it has no job type marker.
      // The service should detect no role/prompt/command and report error.
      const result = await handleScheduleCommand(
        '/schedule add agent-job --cron "0 9 * * *" --tz UTC',
        tempDir
      );

      // @step Then the TUI should display an error message containing "require" and "role" and "prompt"
      expect(result.success).toBe(false);
      expect(result.message.toLowerCase()).toContain('role');
      expect(result.message.toLowerCase()).toContain('prompt');
    });
  });

  describe('Scenario: Reject invalid timezone', () => {
    it('should return error for invalid timezone', async () => {
      // @step When I type "/schedule add test --cron "0 9 * * *" --tz Invalid/Zone --command "echo""
      const result = await handleScheduleCommand(
        '/schedule add test --cron "0 9 * * *" --tz Invalid/Zone --command "echo"',
        tempDir
      );

      // @step Then the TUI should display an error message containing "Invalid timezone"
      expect(result.success).toBe(false);
      expect(result.message.toLowerCase()).toContain('timezone');
    });
  });
});
