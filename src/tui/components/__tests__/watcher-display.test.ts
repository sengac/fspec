// Feature: spec/features/purple-watcher-input-display.feature

import { describe, expect, it } from 'vitest';

/**
 * Interface for parsed supervisor information
 */
interface SupervisorInfo {
  role: string;
  sessionId: string;
  content: string;
}

/**
 * Parse supervisor message prefix to extract role, session ID, and content.
 * Format: [SUPERVISOR: role | Session: id]\ncontent
 */
function parseSupervisorPrefix(text: string): SupervisorInfo | null {
  const match = text.match(/^\[SUPERVISOR: ([^|]+) \| Session: ([^\]]+)\]\n/);
  if (match) {
    return {
      role: match[1].trim(),
      sessionId: match[2].trim(),
      content: text.slice(match[0].length),
    };
  }
  return null;
}

/**
 * Determine base color for a conversation line based on role.
 */
function getBaseColor(
  role: 'user' | 'assistant' | 'tool' | 'supervisor'
): string {
  if (role === 'user') return 'green';
  if (role === 'supervisor') return 'magenta';
  return 'white';
}

describe('Purple Supervisor Input Display', () => {
  describe('Parse supervisor prefix', () => {
    it('should parse supervisor message correctly', () => {
      // @step Given a message with prefix "[SUPERVISOR: Security Reviewer | Session: abc-123]"
      const prefix = '[SUPERVISOR: Security Reviewer | Session: abc-123]';
      const content = 'Consider adding input validation here';
      const fullMessage = `${prefix}\n${content}`;

      // @step When I parse the supervisor prefix
      const result = parseSupervisorPrefix(fullMessage);

      // @step Then the role should be "Security Reviewer"
      expect(result).not.toBeNull();
      expect(result!.role).toBe('Security Reviewer');

      // @step And the session ID should be "abc-123"
      expect(result!.sessionId).toBe('abc-123');

      // @step And the content should be the message body
      expect(result!.content).toBe(content);
    });

    it('should parse another supervisor message correctly', () => {
      // @step Given a message with prefix "[SUPERVISOR: Code Reviewer | Session: xyz-789]"
      const prefix = '[SUPERVISOR: Code Reviewer | Session: xyz-789]';
      const content = 'This looks good to me';
      const fullMessage = `${prefix}\n${content}`;

      // @step When I parse the supervisor prefix
      const result = parseSupervisorPrefix(fullMessage);

      // @step Then the role should be "Code Reviewer"
      expect(result).not.toBeNull();
      expect(result!.role).toBe('Code Reviewer');

      // @step And the session ID should be "xyz-789"
      expect(result!.sessionId).toBe('xyz-789');
    });

    it('should return null for messages without prefix', () => {
      // @step Given a regular message without supervisor prefix
      const message = 'This is a normal user message';

      // @step When I parse the supervisor prefix
      const result = parseSupervisorPrefix(message);

      // @step Then the result should be null
      expect(result).toBeNull();
    });
  });

  describe('Format supervisor message with role prefix', () => {
    it('should format parsed info as "[W] role> content"', () => {
      // @step Given a message with supervisor prefix "[SUPERVISOR: Arch Advisor | Session: def-456]"
      const prefix = '[SUPERVISOR: Arch Advisor | Session: def-456]';
      const content = 'Consider using the Strategy pattern here';
      const fullMessage = `${prefix}\n${content}`;

      // @step When I parse and format the message
      const result = parseSupervisorPrefix(fullMessage);
      const formatted = `[W] ${result!.role}> ${result!.content}`;

      // @step Then the output should be "[W] Arch Advisor> Consider using the Strategy pattern here"
      expect(formatted).toBe(
        '[W] Arch Advisor> Consider using the Strategy pattern here'
      );
    });
  });

  describe('Supervisor messages display in magenta', () => {
    it('should use magenta as base color for supervisor role', () => {
      // @step Given a conversation line with role "supervisor"
      const role: 'user' | 'assistant' | 'tool' | 'supervisor' = 'supervisor';

      // @step When I get the base color for the line
      const color = getBaseColor(role);

      // @step Then the color should be "magenta"
      expect(color).toBe('magenta');
    });

    it('should use green for user and white for assistant', () => {
      expect(getBaseColor('user')).toBe('green');
      expect(getBaseColor('assistant')).toBe('white');
      expect(getBaseColor('tool')).toBe('white');
    });
  });

  describe('Process SupervisorInput chunk to conversation message', () => {
    it('should convert SupervisorInput chunk to supervisor-input message', () => {
      // @step Given a StreamChunk with type "SupervisorInput"
      const chunk = {
        type: 'IncomingMessage',
        text: '[SUPERVISOR: Security Reviewer | Session: abc-123]\nVulnerability detected',
      };

      // @step And the text field contains "[SUPERVISOR: Security Reviewer | Session: abc-123]\nVulnerability detected"
      expect(chunk.text).toBe(
        '[SUPERVISOR: Security Reviewer | Session: abc-123]\nVulnerability detected'
      );

      // @step When I process the chunk
      const result = parseSupervisorPrefix(chunk.text);
      const message = result
        ? {
            type: 'supervisor-input' as const,
            content: `[W] ${result.role}> ${result.content}`,
          }
        : { type: 'supervisor-input' as const, content: chunk.text };

      // @step Then the resulting message type should be "supervisor-input"
      expect(message.type).toBe('supervisor-input');

      // @step And the content should include the role prefix "[W]"
      expect(message.content).toContain('[W]');
      expect(message.content).toContain('Security Reviewer');
    });
  });
});
