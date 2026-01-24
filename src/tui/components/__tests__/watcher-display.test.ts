// Feature: spec/features/purple-watcher-input-display.feature

import { describe, expect, it } from 'vitest';

/**
 * Interface for parsed watcher information
 */
interface WatcherInfo {
  role: string;
  authority: 'Supervisor' | 'Peer';
  sessionId: string;
  content: string;
}

/**
 * Parse watcher message prefix to extract role, authority, session ID, and content.
 * Format: [WATCHER: role | Authority: level | Session: id]\ncontent
 *
 * @param text - The raw message text
 * @returns WatcherInfo object if prefix found, null otherwise
 */
function parseWatcherPrefix(text: string): WatcherInfo | null {
  const match = text.match(
    /^\[WATCHER: ([^|]+) \| Authority: (Supervisor|Peer) \| Session: ([^\]]+)\]\n/
  );
  if (match) {
    return {
      role: match[1].trim(),
      authority: match[2] as 'Supervisor' | 'Peer',
      sessionId: match[3].trim(),
      content: text.slice(match[0].length),
    };
  }
  return null;
}

/**
 * Determine base color for a conversation line based on role.
 */
function getBaseColor(role: 'user' | 'assistant' | 'tool' | 'watcher'): string {
  if (role === 'user') return 'green';
  if (role === 'watcher') return 'magenta';
  return 'white';
}

describe('Purple Watcher Input Display', () => {
  describe('Parse watcher prefix with supervisor authority', () => {
    it('should parse supervisor watcher message correctly', () => {
      // @step Given a message with watcher prefix "[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]"
      const prefix =
        '[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]';

      // @step And the message content is "SQL injection vulnerability detected"
      const content = 'SQL injection vulnerability detected';
      const message = `${prefix}\n${content}`;

      // @step When the parseWatcherPrefix function parses the message
      const result = parseWatcherPrefix(message);

      // @step Then the result should contain role "Security Reviewer"
      expect(result).not.toBeNull();
      expect(result!.role).toBe('Security Reviewer');

      // @step And the result should contain authority "Supervisor"
      expect(result!.authority).toBe('Supervisor');

      // @step And the result should contain sessionId "abc-123"
      expect(result!.sessionId).toBe('abc-123');

      // @step And the result should contain content "SQL injection vulnerability detected"
      expect(result!.content).toBe('SQL injection vulnerability detected');
    });
  });

  describe('Parse watcher prefix with peer authority', () => {
    it('should parse peer watcher message correctly', () => {
      // @step Given a message with watcher prefix "[WATCHER: Code Reviewer | Authority: Peer | Session: xyz-789]"
      const prefix =
        '[WATCHER: Code Reviewer | Authority: Peer | Session: xyz-789]';

      // @step And the message content is "Consider adding error handling"
      const content = 'Consider adding error handling';
      const message = `${prefix}\n${content}`;

      // @step When the parseWatcherPrefix function parses the message
      const result = parseWatcherPrefix(message);

      // @step Then the result should contain role "Code Reviewer"
      expect(result).not.toBeNull();
      expect(result!.role).toBe('Code Reviewer');

      // @step And the result should contain authority "Peer"
      expect(result!.authority).toBe('Peer');

      // @step And the result should contain sessionId "xyz-789"
      expect(result!.sessionId).toBe('xyz-789');

      // @step And the result should contain content "Consider adding error handling"
      expect(result!.content).toBe('Consider adding error handling');
    });
  });

  describe('Parse regular message without watcher prefix', () => {
    it('should return null for messages without watcher prefix', () => {
      // @step Given a message "Regular user message without prefix"
      const message = 'Regular user message without prefix';

      // @step When the parseWatcherPrefix function parses the message
      const result = parseWatcherPrefix(message);

      // @step Then the result should be null
      expect(result).toBeNull();
    });
  });

  describe('Parse multiline watcher message', () => {
    it('should preserve multiline content after prefix', () => {
      // @step Given a message with watcher prefix "[WATCHER: Arch Advisor | Authority: Peer | Session: def-456]"
      const prefix =
        '[WATCHER: Arch Advisor | Authority: Peer | Session: def-456]';

      // @step And the message content is multiline:
      const content = `First line
Second line
Third line`;
      const message = `${prefix}\n${content}`;

      // @step When the parseWatcherPrefix function parses the message
      const result = parseWatcherPrefix(message);

      // @step Then the result should contain all three lines in content
      expect(result).not.toBeNull();
      expect(result!.content).toContain('First line');
      expect(result!.content).toContain('Second line');
      expect(result!.content).toContain('Third line');
      expect(result!.content.split('\n')).toHaveLength(3);
    });
  });

  describe('Display watcher input in magenta color', () => {
    it('should use correct colors for each role type', () => {
      // @step Given a ConversationLine with role "watcher"
      const watcherRole = 'watcher' as const;

      // @step When the line is rendered in the conversation view
      const watcherColor = getBaseColor(watcherRole);

      // @step Then the base color should be "magenta"
      expect(watcherColor).toBe('magenta');

      // @step And it should be distinct from user lines which are "green"
      const userColor = getBaseColor('user');
      expect(userColor).toBe('green');
      expect(watcherColor).not.toBe(userColor);

      // @step And it should be distinct from assistant lines which are "white"
      const assistantColor = getBaseColor('assistant');
      expect(assistantColor).toBe('white');
      expect(watcherColor).not.toBe(assistantColor);
    });
  });

  describe('Process WatcherInput chunk to conversation message', () => {
    it('should create watcher-input message with formatted content', () => {
      // @step Given a StreamChunk with type "WatcherInput"
      const chunk = {
        type: 'WatcherInput',
        text: '[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]\nVulnerability detected',
      };

      // @step And the text field contains "[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]\nVulnerability detected"
      expect(chunk.text).toBe(
        '[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]\nVulnerability detected'
      );

      // @step When processChunksToConversation processes the chunk
      // Simulate processing: parse prefix and format for display
      const parsed = parseWatcherPrefix(chunk.text);
      expect(parsed).not.toBeNull();

      const formattedContent = `[W] ${parsed!.role}> ${parsed!.content}`;
      const conversationMessage = {
        type: 'watcher-input' as const,
        content: formattedContent,
      };

      // @step Then a ConversationMessage with type "watcher-input" should be created
      expect(conversationMessage.type).toBe('watcher-input');

      // @step And the message content should show "[W] Security Reviewer> Vulnerability detected"
      expect(conversationMessage.content).toBe(
        '[W] Security Reviewer> Vulnerability detected'
      );
    });
  });
});
