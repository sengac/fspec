/**
 * Feature: spec/features/chunk-processor.feature
 *
 * Tests for the chunk processor utility module.
 * Validates parsing of supervisor messages and tool args display.
 */

import { describe, it, expect } from 'vitest';
import {
  parseSupervisorPrefix,
  formatSupervisorMessage,
  extractToolArgsDisplay,
  processSupervisorInputChunk,
} from '../chunkProcessor';

describe('Feature: Chunk Processor Utilities', () => {
  describe('Scenario: Parse supervisor message prefix', () => {
    it('should parse supervisor message correctly', () => {
      // @step Given a message with prefix "[SUPERVISOR: Security Reviewer | Session: abc-123]"
      const prefix = '[SUPERVISOR: Security Reviewer | Session: abc-123]';
      const content = 'Consider adding input validation here';
      const fullMessage = `${prefix}\n${content}`;

      // @step When I parse the supervisor prefix
      const result = parseSupervisorPrefix(fullMessage);

      // @step Then the role should be "Security Reviewer"
      expect(result?.role).toBe('Security Reviewer');

      // @step And the session ID should be "abc-123"
      expect(result?.sessionId).toBe('abc-123');

      // @step And the content should be the message body
      expect(result?.content).toBe(content);
    });

    it('should parse another supervisor message correctly', () => {
      // @step Given a message with prefix "[SUPERVISOR: Code Reviewer | Session: xyz-789]"
      const prefix = '[SUPERVISOR: Code Reviewer | Session: xyz-789]';
      const content = 'This looks good to me';
      const fullMessage = `${prefix}\n${content}`;

      // @step When I parse the supervisor prefix
      const result = parseSupervisorPrefix(fullMessage);

      // @step Then the role should be "Code Reviewer"
      expect(result?.role).toBe('Code Reviewer');

      // @step And the session ID should be "xyz-789"
      expect(result?.sessionId).toBe('xyz-789');
    });

    it('should return null for messages without supervisor prefix', () => {
      // @step Given a regular message without supervisor prefix
      const message = 'This is a normal user message';

      // @step When I parse the supervisor prefix
      const result = parseSupervisorPrefix(message);

      // @step Then the result should be null
      expect(result).toBeNull();
    });

    it('should parse bridge message correctly', () => {
      // @step Given a message from bridge "[SUPERVISOR: bridge | Session: bridge]"
      const prefix = '[SUPERVISOR: bridge | Session: bridge]';
      const content = 'Hello from Telegram';
      const fullMessage = `${prefix}\n${content}`;

      // @step When I parse the supervisor prefix
      const result = parseSupervisorPrefix(fullMessage);

      // @step Then the role should be "bridge"
      expect(result?.role).toBe('bridge');

      // @step And the content should be the message body
      expect(result?.content).toBe(content);
    });
  });

  describe('Scenario: Format supervisor message for display', () => {
    it('should format supervisor info as "[W] role> content"', () => {
      // @step Given parsed supervisor info
      const info = {
        role: 'Security Reviewer',
        sessionId: 'abc-123',
        content: 'Check for SQL injection',
      };

      // @step When I format the supervisor message
      const result = formatSupervisorMessage(info);

      // @step Then the result should be "[W] Security Reviewer> Check for SQL injection"
      expect(result).toBe('[W] Security Reviewer> Check for SQL injection');
    });
  });

  describe('Scenario: Process supervisor input chunk', () => {
    it('should create supervisor-input message with formatted content', () => {
      // @step Given a SupervisorInput chunk text
      const text = '[SUPERVISOR: bridge | Session: bridge]\nHello from remote';

      // @step When I process the supervisor input chunk
      const result = processSupervisorInputChunk(text);

      // @step Then the message type should be "supervisor-input"
      expect(result.type).toBe('supervisor-input');

      // @step And the content should be formatted
      expect(result.content).toBe('[W] bridge> Hello from remote');
    });
  });

  describe('Scenario: Extract tool args display', () => {
    it('should format key-value pairs from tool input', () => {
      const input = {
        file_path: '/src/main.ts',
        pattern: 'function $NAME',
        language: 'typescript',
      };

      const result = extractToolArgsDisplay('Grep', input);

      expect(result).toContain('/src/main.ts');
      expect(result).toContain('function $NAME');
      expect(result).toContain('typescript');
    });

    it('should handle empty input', () => {
      const input = {};
      const result = extractToolArgsDisplay('Read', input);
      expect(result).toBe('');
    });

    it('should only show file_path for Write tool', () => {
      const input = {
        file_path: '/src/main.ts',
        content: 'const x = 1;\nconst y = 2;\n',
      };

      const result = extractToolArgsDisplay('Write', input);

      expect(result).toContain('/src/main.ts');
      expect(result).not.toContain('const x = 1');
    });

    it('should only show file_path for Edit tool', () => {
      const input = {
        file_path: '/src/utils.ts',
        old_string: 'old code',
        new_string: 'new code',
      };

      const result = extractToolArgsDisplay('Edit', input);

      expect(result).toContain('/src/utils.ts');
      expect(result).not.toContain('old code');
      expect(result).not.toContain('new code');
    });
  });
});
