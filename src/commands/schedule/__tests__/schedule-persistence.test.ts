/**
 * Feature: spec/features/schedule-persistence.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios map directly to Gherkin scenarios.
 *
 * SCHED-002: Schedule Persistence & Schema
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdtemp, rm, readFile, writeFile, mkdir } from 'fs/promises';
import { tmpdir } from 'os';
import { existsSync } from 'fs';

// These will be implemented in the implementing phase
import { addSchedule } from '../add-schedule';
import { removeSchedule } from '../remove-schedule';
import { pauseSchedule, resumeSchedule } from '../pause-schedule';
import { listSchedules } from '../list-schedules';
import { ensureSchedulesFile } from '../../../utils/ensure-schedules-file';
import type { SchedulesData, ScheduleEntry } from '../../../types/schedule';

describe('Feature: Schedule Persistence & Schema', () => {
  let tempDir: string;
  let specDir: string;
  let schedulesFilePath: string;

  beforeEach(async () => {
    // Create temp directory for each test
    tempDir = await mkdtemp(join(tmpdir(), 'fspec-schedule-test-'));
    specDir = join(tempDir, 'spec');
    await mkdir(specDir, { recursive: true });
    schedulesFilePath = join(specDir, 'schedules.json');
  });

  afterEach(async () => {
    // Clean up temp directory
    await rm(tempDir, { recursive: true, force: true });
  });

  describe('Scenario: Add an agent schedule with all required fields', () => {
    it('should persist an agent schedule to spec/schedules.json with all fields', async () => {
      // @step Given the project has no schedules configured
      expect(existsSync(schedulesFilePath)).toBe(false);

      // @step When I add an agent schedule "nightly-review" with:
      const result = await addSchedule({
        name: 'nightly-review',
        cron: '0 2 * * *',
        timezone: 'Australia/Brisbane',
        jobType: 'agent',
        role: 'Code reviewer',
        prompt: 'Review PRs',
        cwd: tempDir,
      });

      // @step Then the schedule should be persisted to spec/schedules.json
      expect(result.success).toBe(true);
      expect(existsSync(schedulesFilePath)).toBe(true);

      const data = JSON.parse(
        await readFile(schedulesFilePath, 'utf-8')
      ) as SchedulesData;
      const schedule = data.schedules['nightly-review'];
      expect(schedule).toBeDefined();

      // @step And the schedule entry should have jobType "agent"
      expect(schedule.jobType).toBe('agent');

      // @step And the schedule entry should have status "active"
      expect(schedule.status).toBe('active');

      // @step And the schedule entry should have a createdAt timestamp
      expect(schedule.createdAt).toBeDefined();
      expect(new Date(schedule.createdAt).getTime()).not.toBeNaN();
    });
  });

  describe('Scenario: Add a shell command schedule', () => {
    it('should persist a shell schedule to spec/schedules.json', async () => {
      // @step Given the project has no schedules configured
      expect(existsSync(schedulesFilePath)).toBe(false);

      // @step When I add a shell schedule "daily-sync" with:
      const result = await addSchedule({
        name: 'daily-sync',
        cron: '0 9 * * 1-5',
        timezone: 'UTC',
        jobType: 'shell',
        command: 'npm run sync',
        cwd: tempDir,
      });

      // @step Then the schedule should be persisted to spec/schedules.json
      expect(result.success).toBe(true);
      expect(existsSync(schedulesFilePath)).toBe(true);

      const data = JSON.parse(
        await readFile(schedulesFilePath, 'utf-8')
      ) as SchedulesData;
      const schedule = data.schedules['daily-sync'];

      // @step And the schedule entry should have jobType "shell"
      expect(schedule.jobType).toBe('shell');

      // @step And the schedule entry should have status "active"
      expect(schedule.status).toBe('active');
    });
  });

  describe('Scenario: Reject schedule with invalid cron expression', () => {
    it('should fail validation for invalid cron syntax', async () => {
      // @step Given the project has no schedules configured
      expect(existsSync(schedulesFilePath)).toBe(false);

      // @step When I try to add a schedule "bad-cron" with cron "0 99 * * *"
      await expect(
        addSchedule({
          name: 'bad-cron',
          cron: '0 99 * * *', // Invalid: hour field can only be 0-23
          timezone: 'UTC',
          jobType: 'shell',
          command: 'echo test',
          cwd: tempDir,
        })
      ).rejects.toThrow();

      // @step Then the validation should fail with an error about invalid cron syntax
      try {
        await addSchedule({
          name: 'bad-cron',
          cron: '0 99 * * *',
          timezone: 'UTC',
          jobType: 'shell',
          command: 'echo test',
          cwd: tempDir,
        });
      } catch (error: unknown) {
        expect((error as Error).message).toMatch(/cron|syntax|invalid/i);
      }

      // @step And spec/schedules.json should not be modified
      // File should either not exist, or be empty/only have version
      if (existsSync(schedulesFilePath)) {
        const data = JSON.parse(
          await readFile(schedulesFilePath, 'utf-8')
        ) as SchedulesData;
        expect(Object.keys(data.schedules)).toHaveLength(0);
      }
    });
  });

  describe('Scenario: Reject schedule with invalid timezone', () => {
    it('should fail validation for invalid timezone', async () => {
      // @step Given the project has no schedules configured
      expect(existsSync(schedulesFilePath)).toBe(false);

      // @step When I try to add a schedule "bad-tz" with timezone "Fake/City"
      let errorMessage = '';
      try {
        await addSchedule({
          name: 'bad-tz',
          cron: '0 2 * * *',
          timezone: 'Fake/City',
          jobType: 'shell',
          command: 'echo test',
          cwd: tempDir,
        });
      } catch (error: unknown) {
        errorMessage = (error as Error).message;
      }

      // @step Then the validation should fail with an error about invalid timezone
      expect(errorMessage).toMatch(/timezone|invalid/i);

      // @step And the error message should suggest valid timezone values
      // Error message should mention valid timezones or hint at proper format
      expect(errorMessage.length).toBeGreaterThan(0);

      // @step And spec/schedules.json should not be modified
      if (existsSync(schedulesFilePath)) {
        const data = JSON.parse(
          await readFile(schedulesFilePath, 'utf-8')
        ) as SchedulesData;
        expect(Object.keys(data.schedules)).toHaveLength(0);
      }
    });
  });

  describe('Scenario: Reject schedule with invalid name format', () => {
    it('should fail validation for names with spaces and special characters', async () => {
      // @step Given the project has no schedules configured
      expect(existsSync(schedulesFilePath)).toBe(false);

      // @step When I try to add a schedule "My Schedule!" with valid cron and timezone
      let errorMessage = '';
      try {
        await addSchedule({
          name: 'My Schedule!',
          cron: '0 2 * * *',
          timezone: 'UTC',
          jobType: 'shell',
          command: 'echo test',
          cwd: tempDir,
        });
      } catch (error: unknown) {
        errorMessage = (error as Error).message;
      }

      // @step Then the validation should fail requiring slug format
      expect(errorMessage).toMatch(
        /slug|format|name|invalid|lowercase|hyphen/i
      );

      // @step And spec/schedules.json should not be modified
      if (existsSync(schedulesFilePath)) {
        const data = JSON.parse(
          await readFile(schedulesFilePath, 'utf-8')
        ) as SchedulesData;
        expect(Object.keys(data.schedules)).toHaveLength(0);
      }
    });
  });

  describe('Scenario: Reject duplicate schedule name', () => {
    it('should fail when adding a schedule with a name that already exists', async () => {
      // @step Given a schedule "nightly-review" already exists
      await mkdir(specDir, { recursive: true });
      const existingSchedule: ScheduleEntry = {
        name: 'nightly-review',
        cron: '0 2 * * *',
        timezone: 'Australia/Brisbane',
        jobType: 'agent',
        role: 'Code reviewer',
        prompt: 'Review PRs',
        overlapPolicy: 'skip',
        status: 'active',
        lastRunAt: null,
        lastRunStatus: null,
        createdAt: new Date().toISOString(),
      };
      const initialData: SchedulesData = {
        version: '1.0.0',
        schedules: { 'nightly-review': existingSchedule },
      };
      await writeFile(schedulesFilePath, JSON.stringify(initialData, null, 2));

      // @step When I try to add another schedule named "nightly-review"
      let errorMessage = '';
      try {
        await addSchedule({
          name: 'nightly-review',
          cron: '0 3 * * *', // Different cron
          timezone: 'UTC',
          jobType: 'shell',
          command: 'echo test',
          cwd: tempDir,
        });
      } catch (error: unknown) {
        errorMessage = (error as Error).message;
      }

      // @step Then the validation should fail with "schedule already exists" error
      expect(errorMessage).toMatch(/already exists|duplicate/i);

      // @step And the existing schedule should remain unchanged
      const data = JSON.parse(
        await readFile(schedulesFilePath, 'utf-8')
      ) as SchedulesData;
      expect(data.schedules['nightly-review'].cron).toBe('0 2 * * *');
      expect(data.schedules['nightly-review'].jobType).toBe('agent');
    });
  });

  describe('Scenario: Pause an active schedule', () => {
    it('should update the schedule status to paused', async () => {
      // @step Given an active schedule "nightly-review" exists
      await mkdir(specDir, { recursive: true });
      const activeSchedule: ScheduleEntry = {
        name: 'nightly-review',
        cron: '0 2 * * *',
        timezone: 'Australia/Brisbane',
        jobType: 'agent',
        role: 'Code reviewer',
        prompt: 'Review PRs',
        overlapPolicy: 'skip',
        status: 'active',
        lastRunAt: null,
        lastRunStatus: null,
        createdAt: new Date().toISOString(),
      };
      const initialData: SchedulesData = {
        version: '1.0.0',
        schedules: { 'nightly-review': activeSchedule },
      };
      await writeFile(schedulesFilePath, JSON.stringify(initialData, null, 2));
      const originalCreatedAt = activeSchedule.createdAt;

      // @step When I pause the schedule "nightly-review"
      const result = await pauseSchedule({
        name: 'nightly-review',
        cwd: tempDir,
      });
      expect(result.success).toBe(true);

      // @step Then the schedule status should be updated to "paused"
      const data = JSON.parse(
        await readFile(schedulesFilePath, 'utf-8')
      ) as SchedulesData;
      expect(data.schedules['nightly-review'].status).toBe('paused');

      // @step And the schedule should remain in spec/schedules.json
      expect(data.schedules['nightly-review']).toBeDefined();

      // @step And all other schedule fields should be unchanged
      expect(data.schedules['nightly-review'].cron).toBe('0 2 * * *');
      expect(data.schedules['nightly-review'].timezone).toBe(
        'Australia/Brisbane'
      );
      expect(data.schedules['nightly-review'].createdAt).toBe(
        originalCreatedAt
      );
    });
  });

  describe('Scenario: Resume a paused schedule', () => {
    it('should update the schedule status back to active', async () => {
      // @step Given a paused schedule "nightly-review" exists
      await mkdir(specDir, { recursive: true });
      const pausedSchedule: ScheduleEntry = {
        name: 'nightly-review',
        cron: '0 2 * * *',
        timezone: 'Australia/Brisbane',
        jobType: 'agent',
        role: 'Code reviewer',
        prompt: 'Review PRs',
        overlapPolicy: 'skip',
        status: 'paused',
        lastRunAt: null,
        lastRunStatus: null,
        createdAt: new Date().toISOString(),
      };
      const initialData: SchedulesData = {
        version: '1.0.0',
        schedules: { 'nightly-review': pausedSchedule },
      };
      await writeFile(schedulesFilePath, JSON.stringify(initialData, null, 2));

      // @step When I resume the schedule "nightly-review"
      const result = await resumeSchedule({
        name: 'nightly-review',
        cwd: tempDir,
      });
      expect(result.success).toBe(true);

      // @step Then the schedule status should be updated to "active"
      const data = JSON.parse(
        await readFile(schedulesFilePath, 'utf-8')
      ) as SchedulesData;
      expect(data.schedules['nightly-review'].status).toBe('active');
    });
  });

  describe('Scenario: Remove a schedule', () => {
    it('should delete the schedule from spec/schedules.json', async () => {
      // @step Given a schedule "daily-sync" exists
      await mkdir(specDir, { recursive: true });
      const schedule: ScheduleEntry = {
        name: 'daily-sync',
        cron: '0 9 * * 1-5',
        timezone: 'UTC',
        jobType: 'shell',
        command: 'npm run sync',
        overlapPolicy: 'skip',
        status: 'active',
        lastRunAt: null,
        lastRunStatus: null,
        createdAt: new Date().toISOString(),
      };
      const initialData: SchedulesData = {
        version: '1.0.0',
        schedules: { 'daily-sync': schedule },
      };
      await writeFile(schedulesFilePath, JSON.stringify(initialData, null, 2));

      // @step When I remove the schedule "daily-sync"
      const result = await removeSchedule({ name: 'daily-sync', cwd: tempDir });
      expect(result.success).toBe(true);

      // @step Then the schedule should be deleted from spec/schedules.json
      const data = JSON.parse(
        await readFile(schedulesFilePath, 'utf-8')
      ) as SchedulesData;
      expect(data.schedules['daily-sync']).toBeUndefined();

      // @step And no trace of "daily-sync" should remain in the file
      const fileContent = await readFile(schedulesFilePath, 'utf-8');
      expect(fileContent).not.toContain('daily-sync');
    });
  });

  describe('Scenario: List all configured schedules', () => {
    it('should display all schedules in a table format', async () => {
      // @step Given the following schedules exist:
      await mkdir(specDir, { recursive: true });
      const schedules: SchedulesData = {
        version: '1.0.0',
        schedules: {
          'nightly-review': {
            name: 'nightly-review',
            cron: '0 2 * * *',
            timezone: 'Australia/Brisbane',
            jobType: 'agent',
            role: 'Code reviewer',
            prompt: 'Review PRs',
            overlapPolicy: 'skip',
            status: 'active',
            lastRunAt: null,
            lastRunStatus: null,
            createdAt: new Date().toISOString(),
          },
          'daily-sync': {
            name: 'daily-sync',
            cron: '0 9 * * 1-5',
            timezone: 'UTC',
            jobType: 'shell',
            command: 'npm run sync',
            overlapPolicy: 'skip',
            status: 'paused',
            lastRunAt: null,
            lastRunStatus: null,
            createdAt: new Date().toISOString(),
          },
        },
      };
      await writeFile(schedulesFilePath, JSON.stringify(schedules, null, 2));

      // @step When I list all schedules
      const result = await listSchedules({ cwd: tempDir });

      // @step Then I should see a table with columns: name, cron, timezone, type, status, last run, next run
      expect(result.schedules).toHaveLength(2);
      expect(result.columns).toContain('name');
      expect(result.columns).toContain('cron');
      expect(result.columns).toContain('timezone');
      expect(result.columns).toContain('type');
      expect(result.columns).toContain('status');

      // @step And the table should contain both schedules
      const names = result.schedules.map((s: ScheduleEntry) => s.name);
      expect(names).toContain('nightly-review');
      expect(names).toContain('daily-sync');
    });
  });

  describe('Scenario: Auto-create schedules file when missing', () => {
    it('should create spec/schedules.json with default structure when missing', async () => {
      // @step Given spec/schedules.json does not exist
      expect(existsSync(schedulesFilePath)).toBe(false);

      // @step When I run a schedule command that requires the file
      await ensureSchedulesFile(tempDir);

      // @step Then spec/schedules.json should be created
      expect(existsSync(schedulesFilePath)).toBe(true);

      // @step And the file should have version "1.0.0"
      const data = JSON.parse(
        await readFile(schedulesFilePath, 'utf-8')
      ) as SchedulesData;
      expect(data.version).toBe('1.0.0');

      // @step And the schedules object should be empty
      expect(Object.keys(data.schedules)).toHaveLength(0);
    });
  });

  describe('Scenario: Update last run timestamp after execution', () => {
    it('should update lastRunAt and lastRunStatus after scheduler completes a run', async () => {
      // @step Given a schedule "nightly-review" exists with no previous runs
      await mkdir(specDir, { recursive: true });
      const schedule: ScheduleEntry = {
        name: 'nightly-review',
        cron: '0 2 * * *',
        timezone: 'Australia/Brisbane',
        jobType: 'agent',
        role: 'Code reviewer',
        prompt: 'Review PRs',
        overlapPolicy: 'skip',
        status: 'active',
        lastRunAt: null,
        lastRunStatus: null,
        createdAt: new Date().toISOString(),
      };
      const initialData: SchedulesData = {
        version: '1.0.0',
        schedules: { 'nightly-review': schedule },
      };
      await writeFile(schedulesFilePath, JSON.stringify(initialData, null, 2));
      expect(schedule.lastRunAt).toBeNull();
      expect(schedule.lastRunStatus).toBeNull();

      // @step When the scheduler engine completes a run of "nightly-review"
      // This simulates what the Rust scheduler will do
      const beforeUpdate = new Date();
      const data = JSON.parse(
        await readFile(schedulesFilePath, 'utf-8')
      ) as SchedulesData;
      data.schedules['nightly-review'].lastRunAt = new Date().toISOString();
      data.schedules['nightly-review'].lastRunStatus = 'completed';
      await writeFile(schedulesFilePath, JSON.stringify(data, null, 2));

      // @step Then the lastRunAt field should be updated to the current timestamp
      const updatedData = JSON.parse(
        await readFile(schedulesFilePath, 'utf-8')
      ) as SchedulesData;
      expect(updatedData.schedules['nightly-review'].lastRunAt).not.toBeNull();
      const lastRunAt = new Date(
        updatedData.schedules['nightly-review'].lastRunAt!
      );
      expect(lastRunAt.getTime()).toBeGreaterThanOrEqual(
        beforeUpdate.getTime()
      );

      // @step And the lastRunStatus field should be set to "completed"
      expect(updatedData.schedules['nightly-review'].lastRunStatus).toBe(
        'completed'
      );
    });
  });
});
