/**
 * Feature: spec/features/chunk-processor.feature
 *
 * Tests for the chunk processor utility module.
 * Validates parsing of watcher messages and tool args display.
 */

import { describe, it, expect } from 'vitest';
import {
  parseWatcherPrefix,
  formatWatcherMessage,
  extractToolArgsDisplay,
  processWatcherInputChunk,
} from '../chunkProcessor';

describe('Feature: Chunk Processor Utilities', () => {
  describe('Scenario: Parse watcher message prefix', () => {
    it('should parse supervisor watcher message correctly', () => {
      // @step Given a message with watcher prefix "[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]"
      const prefix =
        '[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]';
      const content = 'Consider adding input validation here';
      const fullMessage = `${prefix}\n${content}`;

      // @step When I parse the watcher prefix
      const result = parseWatcherPrefix(fullMessage);

      // @step Then the role should be "Security Reviewer"
      expect(result?.role).toBe('Security Reviewer');

      // @step And the authority should be "Supervisor"
      expect(result?.authority).toBe('Supervisor');

      // @step And the session ID should be "abc-123"
      expect(result?.sessionId).toBe('abc-123');

      // @step And the content should be the message body
      expect(result?.content).toBe(content);
    });

    it('should parse peer watcher message correctly', () => {
      // @step Given a message with watcher prefix "[WATCHER: Code Reviewer | Authority: Peer | Session: xyz-789]"
      const prefix =
        '[WATCHER: Code Reviewer | Authority: Peer | Session: xyz-789]';
      const content = 'This looks good to me';
      const fullMessage = `${prefix}\n${content}`;

      // @step When I parse the watcher prefix
      const result = parseWatcherPrefix(fullMessage);

      // @step Then the role should be "Code Reviewer"
      expect(result?.role).toBe('Code Reviewer');

      // @step And the authority should be "Peer"
      expect(result?.authority).toBe('Peer');

      // @step And the session ID should be "xyz-789"
      expect(result?.sessionId).toBe('xyz-789');
    });

    it('should return null for messages without watcher prefix', () => {
      // @step Given a regular message without watcher prefix
      const message = 'This is a normal user message';

      // @step When I parse the watcher prefix
      const result = parseWatcherPrefix(message);

      // @step Then the result should be null
      expect(result).toBeNull();
    });

    it('should parse bridge message correctly', () => {
      // @step Given a message from bridge "[WATCHER: bridge | Authority: Peer | Session: bridge]"
      const prefix = '[WATCHER: bridge | Authority: Peer | Session: bridge]';
      const content = 'Hello from Telegram';
      const fullMessage = `${prefix}\n${content}`;

      // @step When I parse the watcher prefix
      const result = parseWatcherPrefix(fullMessage);

      // @step Then the role should be "bridge"
      expect(result?.role).toBe('bridge');

      // @step And the content should be the message body
      expect(result?.content).toBe(content);
    });
  });

  describe('Scenario: Format watcher message for display', () => {
    it('should format watcher info as "[W] role> content"', () => {
      // @step Given parsed watcher info
      const info = {
        role: 'Security Reviewer',
        authority: 'Supervisor' as const,
        sessionId: 'abc-123',
        content: 'Check for SQL injection',
      };

      // @step When I format the watcher message
      const result = formatWatcherMessage(info);

      // @step Then the result should be "[W] Security Reviewer> Check for SQL injection"
      expect(result).toBe('[W] Security Reviewer> Check for SQL injection');
    });
  });

  describe('Scenario: Process watcher input chunk', () => {
    it('should create watcher-input message with formatted content', () => {
      // @step Given a WatcherInput chunk text
      const text =
        '[WATCHER: bridge | Authority: Peer | Session: bridge]\nHello from remote';

      // @step When I process the watcher input chunk
      const result = processWatcherInputChunk(text);

      // @step Then the message type should be "watcher-input"
      expect(result.type).toBe('watcher-input');

      // @step And the content should be formatted
      expect(result.content).toBe('[W] bridge> Hello from remote');
    });

    it('should handle messages without valid prefix', () => {
      // @step Given an invalid watcher message
      const text = 'Just some raw text';

      // @step When I process the watcher input chunk
      const result = processWatcherInputChunk(text);

      // @step Then the message type should be "watcher-input"
      expect(result.type).toBe('watcher-input');

      // @step And the content should be the raw text
      expect(result.content).toBe(text);
    });
  });

  describe('Scenario: Extract tool args display', () => {
    it('should show only file_path for Edit tool', () => {
      // @step Given an Edit tool input with file_path and content
      const input = {
        file_path: '/home/user/file.ts',
        old_string: 'foo',
        new_string: 'bar',
      };

      // @step When I extract the args display
      const result = extractToolArgsDisplay('Edit', input);

      // @step Then only the file_path should be shown
      expect(result).toBe('/home/user/file.ts');
    });

    it('should show only file_path for Write tool', () => {
      // @step Given a Write tool input with file_path and content
      const input = {
        file_path: '/home/user/new-file.ts',
        content: 'file contents here',
      };

      // @step When I extract the args display
      const result = extractToolArgsDisplay('Write', input);

      // @step Then only the file_path should be shown
      expect(result).toBe('/home/user/new-file.ts');
    });

    it('should show command first for Fspec tool', () => {
      // @step Given a Fspec tool input with command and args
      const input = {
        command: 'board',
        project_root: '.',
        args: '{}',
      };

      // @step When I extract the args display
      const result = extractToolArgsDisplay('Fspec', input);

      // @step Then the command should be shown first
      expect(result.startsWith('board')).toBe(true);
    });

    it('should show action_type first for WebSearch tool', () => {
      // @step Given a WebSearch tool input with action_type
      const input = {
        action_type: 'search',
        query: 'typescript best practices',
      };

      // @step When I extract the args display
      const result = extractToolArgsDisplay('WebSearch', input);

      // @step Then the action_type should be shown first
      expect(result.startsWith('search')).toBe(true);
    });

    it('should show all parameters for generic tools', () => {
      // @step Given a Read tool input with multiple params
      const input = {
        file_path: '/home/user/file.ts',
        offset: 100,
        limit: 50,
      };

      // @step When I extract the args display
      const result = extractToolArgsDisplay('Read', input);

      // @step Then all parameters should be shown
      expect(result).toContain('file_path');
      expect(result).toContain('offset');
      expect(result).toContain('limit');
    });

    it('should truncate long string values', () => {
      // @step Given a tool input with a very long string
      const longString = 'x'.repeat(200);
      const input = {
        pattern: longString,
      };

      // @step When I extract the args display
      const result = extractToolArgsDisplay('Grep', input);

      // @step Then the value should be truncated with ellipsis
      expect(result).toContain('...');
      expect(result.length).toBeLessThan(longString.length);
    });
  });
});
