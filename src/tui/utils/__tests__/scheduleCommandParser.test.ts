/**
 * Feature: spec/features/schedule-tui-slash-commands.feature
 *
 * This test file validates the scheduleCommandParser — parsing /schedule slash commands
 * into structured arguments. Tests the parser in isolation.
 *
 * SCHED-008: Schedule TUI Slash Commands
 */

import { describe, it, expect } from 'vitest';
import { parseScheduleCommand } from '../scheduleCommandParser';

describe('Feature: Schedule TUI Slash Commands — Parser', () => {
  describe('Scenario: Add an agent-type schedule via slash command', () => {
    it('should parse /schedule add with agent flags', () => {
      // @step Given no schedule named "nightly-review" exists
      // (precondition — not parser's concern)

      // @step When I type "/schedule add nightly-review --cron "0 2 * * *" --tz Australia/Brisbane --role "Code reviewer" --prompt "Review all open PRs""
      const result = parseScheduleCommand(
        '/schedule add nightly-review --cron "0 2 * * *" --tz Australia/Brisbane --role "Code reviewer" --prompt "Review all open PRs"'
      );

      // @step Then the TUI should display a success message containing "nightly-review" and "added"
      expect(result.subcommand).toBe('add');
      expect(result.name).toBe('nightly-review');
      expect(result.cron).toBe('0 2 * * *');
      expect(result.timezone).toBe('Australia/Brisbane');
      expect(result.role).toBe('Code reviewer');
      expect(result.prompt).toBe('Review all open PRs');
    });
  });

  describe('Scenario: Add a shell-type schedule via slash command', () => {
    it('should parse /schedule add with shell flags', () => {
      // @step Given no schedule named "daily-sync" exists
      // (precondition — not parser's concern)

      // @step When I type "/schedule add daily-sync --cron "0 9 * * 1-5" --tz UTC --command "npm run sync""
      const result = parseScheduleCommand(
        '/schedule add daily-sync --cron "0 9 * * 1-5" --tz UTC --command "npm run sync"'
      );

      // @step Then the TUI should display a success message containing "daily-sync" and "added"
      expect(result.subcommand).toBe('add');
      expect(result.name).toBe('daily-sync');
      expect(result.cron).toBe('0 9 * * 1-5');
      expect(result.timezone).toBe('UTC');
      expect(result.command).toBe('npm run sync');
    });
  });

  describe('Scenario: List all schedules', () => {
    it('should parse /schedule list', () => {
      // @step Given a schedule named "nightly-review" exists with cron "0 2 * * *"
      // @step And a schedule named "daily-sync" exists with cron "0 9 * * 1-5"
      // (preconditions — not parser's concern)

      // @step When I type "/schedule list"
      const result = parseScheduleCommand('/schedule list');

      // @step Then the TUI should display a table containing "nightly-review" and "daily-sync"
      expect(result.subcommand).toBe('list');
    });
  });

  describe('Scenario: Pause an active schedule', () => {
    it('should parse /schedule pause with name', () => {
      // @step Given an active schedule named "nightly-review" exists
      // (precondition — not parser's concern)

      // @step When I type "/schedule pause nightly-review"
      const result = parseScheduleCommand('/schedule pause nightly-review');

      // @step Then the TUI should display a success message containing "nightly-review" and "paused"
      expect(result.subcommand).toBe('pause');
      expect(result.name).toBe('nightly-review');
    });
  });

  describe('Scenario: Resume a paused schedule', () => {
    it('should parse /schedule resume with name', () => {
      // @step Given a paused schedule named "nightly-review" exists
      // (precondition — not parser's concern)

      // @step When I type "/schedule resume nightly-review"
      const result = parseScheduleCommand('/schedule resume nightly-review');

      // @step Then the TUI should display a success message containing "nightly-review" and "resumed"
      expect(result.subcommand).toBe('resume');
      expect(result.name).toBe('nightly-review');
    });
  });

  describe('Scenario: Remove an existing schedule', () => {
    it('should parse /schedule remove with name', () => {
      // @step Given a schedule named "daily-sync" exists
      // (precondition — not parser's concern)

      // @step When I type "/schedule remove daily-sync"
      const result = parseScheduleCommand('/schedule remove daily-sync');

      // @step Then the TUI should display a success message containing "daily-sync" and "removed"
      expect(result.subcommand).toBe('remove');
      expect(result.name).toBe('daily-sync');
    });
  });

  describe('Scenario: Reject invalid cron expression', () => {
    it('should parse /schedule add with invalid cron', () => {
      // @step When I type "/schedule add bad --cron "not-a-cron" --tz UTC --command "echo hi""
      const result = parseScheduleCommand(
        '/schedule add bad --cron "not-a-cron" --tz UTC --command "echo hi"'
      );

      // @step Then the TUI should display an error message containing "Invalid cron expression"
      expect(result.subcommand).toBe('add');
      expect(result.name).toBe('bad');
      expect(result.cron).toBe('not-a-cron');
    });
  });

  describe('Scenario: Show usage help when no subcommand provided', () => {
    it('should return help subcommand for bare /schedule', () => {
      // @step When I type "/schedule"
      const result = parseScheduleCommand('/schedule');

      // @step Then the TUI should display usage help containing "add" and "list" and "pause" and "resume" and "remove"
      expect(result.subcommand).toBe('help');
    });
  });

  describe('Scenario: Reject agent schedule missing required fields', () => {
    it('should parse /schedule add without role/prompt flags', () => {
      // @step When I type "/schedule add agent-job --cron "0 9 * * *" --tz UTC"
      const result = parseScheduleCommand(
        '/schedule add agent-job --cron "0 9 * * *" --tz UTC'
      );

      // @step Then the TUI should display an error message containing "require" and "role" and "prompt"
      expect(result.subcommand).toBe('add');
      expect(result.name).toBe('agent-job');
      expect(result.role).toBeUndefined();
      expect(result.prompt).toBeUndefined();
      expect(result.command).toBeUndefined();
    });
  });

  describe('Scenario: Reject invalid timezone', () => {
    it('should parse /schedule add with invalid timezone', () => {
      // @step When I type "/schedule add test --cron "0 9 * * *" --tz Invalid/Zone --command "echo""
      const result = parseScheduleCommand(
        '/schedule add test --cron "0 9 * * *" --tz Invalid/Zone --command "echo"'
      );

      // @step Then the TUI should display an error message containing "Invalid timezone"
      expect(result.subcommand).toBe('add');
      expect(result.name).toBe('test');
      expect(result.timezone).toBe('Invalid/Zone');
    });
  });

  describe('Edge cases', () => {
    it('should handle --overlap flag', () => {
      const result = parseScheduleCommand(
        '/schedule add test --cron "*/5 * * * *" --tz UTC --command "echo" --overlap queue'
      );

      expect(result.subcommand).toBe('add');
      expect(result.overlapPolicy).toBe('queue');
    });

    it('should return help for unknown subcommand', () => {
      const result = parseScheduleCommand('/schedule unknown');
      expect(result.subcommand).toBe('help');
    });
  });
});
