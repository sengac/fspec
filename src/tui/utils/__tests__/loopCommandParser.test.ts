/**
 * Feature: spec/features/loop-shorthand-natural-language-schedule-creation.feature
 *
 * This test file validates the loop command parser — the deterministic interval
 * extraction logic that converts natural language into cron expressions.
 *
 * SCHED-011: Loop Shorthand — Natural Language Schedule Creation
 */

import { describe, it, expect } from 'vitest';
import { parseLoopCommand } from '../loopCommandParser';

describe('Feature: Loop Shorthand — Parser', () => {
  describe('Scenario: Create loop with leading interval token (minutes)', () => {
    it('should parse leading interval and extract prompt', () => {
      // @step Given the fspec TUI is running
      // (parser is stateless, no setup needed)

      // @step When I type "/loop 5m check deployment status"
      const result = parseLoopCommand('/loop 5m check deployment status');

      // @step Then the interval should be parsed as 300 seconds (5 minutes)
      expect(result.intervalSeconds).toBe(300);

      // @step And the cron expression should be "*/5 * * * *"
      expect(result.cron).toBe('*/5 * * * *');

      // @step And the prompt should be "check deployment status"
      expect(result.prompt).toBe('check deployment status');

      // @step And the TUI should confirm "Scheduled every 5 minutes" with an 8-character job ID
      expect(result.subcommand).toBe('add');
    });
  });

  describe('Scenario: Create loop with leading interval token (seconds)', () => {
    it('should parse seconds interval correctly', () => {
      // @step When I type "/loop 5s check health"
      const result = parseLoopCommand('/loop 5s check health');

      // @step Then the interval should be parsed as 5 seconds
      expect(result.intervalSeconds).toBe(5);

      // @step And the cron expression should fall back to every 1 minute (cron minimum)
      expect(result.cron).toBe('*/1 * * * *');

      // @step And the prompt should be "check health"
      expect(result.prompt).toBe('check health');
      expect(result.subcommand).toBe('add');
    });
  });

  describe('Scenario: Create loop with leading interval token (hours)', () => {
    it('should parse hours interval correctly', () => {
      // @step When I type "/loop 2h check status"
      const result = parseLoopCommand('/loop 2h check status');

      // @step Then the interval should be parsed as 7200 seconds (2 hours)
      expect(result.intervalSeconds).toBe(7200);

      // @step And the cron expression should be "0 */2 * * *"
      expect(result.cron).toBe('0 */2 * * *');

      // @step And the prompt should be "check status"
      expect(result.prompt).toBe('check status');
    });
  });

  describe('Scenario: Create loop with leading interval token (days)', () => {
    it('should parse days interval correctly', () => {
      // @step When I type "/loop 1d run report"
      const result = parseLoopCommand('/loop 1d run report');

      // @step Then the interval should be parsed as 86400 seconds (1 day)
      expect(result.intervalSeconds).toBe(86400);

      // @step And the cron expression should be "0 0 */1 * *"
      expect(result.cron).toBe('0 0 */1 * *');

      // @step And the prompt should be "run report"
      expect(result.prompt).toBe('run report');
    });
  });

  describe('Scenario: Create loop with default interval when none specified', () => {
    it('should default to 600 seconds (10 minutes) when no interval is given', () => {
      // @step Given the fspec TUI is running
      // (parser is stateless)

      // @step When I type "/loop check the build"
      const result = parseLoopCommand('/loop check the build');

      // @step Then the interval should default to 600 seconds (10 minutes)
      expect(result.intervalSeconds).toBe(600);

      // @step And the cron expression should be "*/10 * * * *"
      expect(result.cron).toBe('*/10 * * * *');

      // @step And the prompt should be "check the build"
      expect(result.prompt).toBe('check the build');
    });
  });

  describe('Scenario: Create loop with trailing interval clause', () => {
    it('should parse trailing "every N unit" clause', () => {
      // @step Given the fspec TUI is running

      // @step When I type "/loop check build status every 2 hours"
      const result = parseLoopCommand('/loop check build status every 2 hours');

      // @step Then the trailing "every 2 hours" should be parsed as 7200 seconds
      expect(result.intervalSeconds).toBe(7200);

      // @step And the cron expression should be "0 */2 * * *"
      expect(result.cron).toBe('0 */2 * * *');

      // @step And the prompt should be "check build status"
      expect(result.prompt).toBe('check build status');
    });
  });

  describe('Scenario: Trailing interval clause with seconds', () => {
    it('should parse trailing "every 30 seconds" clause', () => {
      // @step When I type "/loop check health every 30 seconds"
      const result = parseLoopCommand('/loop check health every 30 seconds');

      // @step Then the interval should be 30 seconds
      expect(result.intervalSeconds).toBe(30);

      // @step And the cron expression should be "*/1 * * * *" (cron minimum)
      expect(result.cron).toBe('*/1 * * * *');

      // @step And the prompt should be "check health"
      expect(result.prompt).toBe('check health');
    });
  });

  describe('Scenario: Chain slash commands as loop prompts', () => {
    it('should preserve slash command as the prompt', () => {
      // @step Given the fspec TUI is running

      // @step When I type "/loop 20m /review-pr 1234"
      const result = parseLoopCommand('/loop 20m /review-pr 1234');

      // @step Then the interval should be parsed as 1200 seconds (20 minutes)
      expect(result.intervalSeconds).toBe(1200);

      // @step And the prompt should be "/review-pr 1234"
      expect(result.prompt).toBe('/review-pr 1234');

      // @step And the scheduler should send the prompt to a subordinate agent session
      expect(result.subcommand).toBe('add');
    });
  });

  describe('Scenario: Cancel an active loop by job ID', () => {
    it('should parse cancel subcommand with job ID', () => {
      // @step Given a loop is running with job ID "a1b2c3d4"
      // (parser doesn't need state, just validates the parse)

      // @step When I type "/loop cancel a1b2c3d4"
      const result = parseLoopCommand('/loop cancel a1b2c3d4');

      // @step Then the session-scoped schedule should be removed
      expect(result.subcommand).toBe('cancel');
      expect(result.jobId).toBe('a1b2c3d4');

      // @step And the TUI should confirm "Cancelled loop a1b2c3d4"
      // (TUI rendering tested in service layer)
    });
  });

  describe('Scenario: List all active loops', () => {
    it('should parse list subcommand', () => {
      // @step Given loops are running with IDs "abc12345" and "def67890"
      // (parser stateless)

      // @step When I type "/loop list"
      const result = parseLoopCommand('/loop list');

      // @step Then the TUI should display a table of active loops
      expect(result.subcommand).toBe('list');

      // @step And the table should include columns for ID, Prompt, Interval, Next Fire, and Expires
      // (column rendering tested in service layer)
    });
  });

  describe('Scenario: Show usage help when no arguments provided', () => {
    it('should parse bare /loop as help', () => {
      // @step Given the fspec TUI is running

      // @step When I type "/loop" with no arguments
      const result = parseLoopCommand('/loop');

      // @step Then the TUI should display usage help for the /loop command
      expect(result.subcommand).toBe('help');
    });
  });

  describe('Scenario: Hourly interval generates correct cron', () => {
    it('should generate hourly cron for hour intervals', () => {
      const result = parseLoopCommand('/loop 1h check logs');
      expect(result.intervalSeconds).toBe(3600);
      expect(result.cron).toBe('0 */1 * * *');
      expect(result.prompt).toBe('check logs');
    });
  });

  describe('Scenario: 30-second interval is preserved (not rounded)', () => {
    it('should keep 30s as 30 seconds', () => {
      const result = parseLoopCommand('/loop 30s run health check');
      expect(result.intervalSeconds).toBe(30);
      expect(result.cron).toBe('*/1 * * * *');
      expect(result.prompt).toBe('run health check');
      expect(result.subcommand).toBe('add');
    });
  });
});
