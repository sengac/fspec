/**
 * Feature: spec/features/refactor-streamchunk-to-use-proper-discriminated-union-types.feature
 * NAPI-010: Refactor StreamChunk to use proper discriminated union types
 *
 * These tests verify:
 * 1. The StreamChunk type is a proper discriminated union (type structure)
 * 2. SessionStateChange is NOT added to conversation (behavior)
 * 3. UserNotification IS added to conversation (behavior)
 * 4. CompactionComplete provides structured metrics without conversation pollution
 * 5. The handler code uses discriminated union pattern (static analysis)
 */

import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';

// Import the actual processChunksToConversation function to test behavior
// We need to test that the function properly handles the new chunk types
import type { StreamChunk } from '@sengac/codelet-napi';

describe('NAPI-010: StreamChunk Discriminated Union', () => {
  // Read the actual generated TypeScript definitions
  const indexDtsPath = path.join(process.cwd(), 'codelet/napi/index.d.ts');
  const indexDts = fs.readFileSync(indexDtsPath, 'utf-8');

  // Read the AgentView source to verify implementation patterns
  const agentViewPath = path.join(process.cwd(), 'src/tui/components/AgentView.tsx');
  const agentViewSource = fs.readFileSync(agentViewPath, 'utf-8');

  describe('Scenario: SessionStateChange chunk updates state without adding to conversation', () => {
    it('should have SessionStateChange variant in discriminated union type', () => {
      // @step Given a StreamChunk handler processes incoming chunks from Rust
      // Verify the type definition includes SessionStateChange as a discriminated union variant
      expect(indexDts).toMatch(/\|\s*\{\s*type:\s*['"]SessionStateChange['"]/);
      expect(indexDts).toContain('state: SessionState');
    });

    it('should handle SessionStateChange by calling refreshRustState, not adding messages to conversation', () => {
      // @step Given a StreamChunk handler processes incoming chunks from Rust
      // @step When Rust emits a SessionStateChange chunk with state Compacting
      // @step Then the handler updates isCompacting state to true
      // @step And no message is ADDED to the conversation

      // SessionStateChange should:
      // 1. Call refreshRustState to update UI state (status indicators)
      // 2. NOT ADD messages to conversation (internal state changes are not user-visible messages)
      // 3. TUI-066: Cleared state is special - it CLEARS conversation (setConversation([])), not adds

      // Check handleStreamChunk (the main streaming handler)
      const handleStreamChunkFn = agentViewSource.match(
        /const handleStreamChunk\s*=\s*useCallback\s*\(\s*\(chunk:\s*StreamChunk\)[\s\S]*?\n\s*\},\s*\[/
      );
      expect(handleStreamChunkFn).not.toBeNull();

      const handlerCode = handleStreamChunkFn![0];

      // Extract the SessionStateChange handling block (full block until next chunk type check)
      const sessionStateChangeMatch = handlerCode.match(
        /chunk\.type\s*===\s*['"]SessionStateChange['"][\s\S]*?refreshRustState\([^)]+\);/
      );
      expect(sessionStateChangeMatch).not.toBeNull();

      const sessionStateHandler = sessionStateChangeMatch![0];

      // Verify it calls refreshRustState (which reads state including isCompacting from Rust)
      expect(sessionStateHandler).toContain('refreshRustState');

      // TUI-066: Verify it handles Cleared state by clearing conversation (not adding to it)
      // setConversation([]) clears the conversation, which is correct for Cleared state
      expect(sessionStateHandler).toContain("chunk.state === 'Cleared'");
      expect(sessionStateHandler).toContain('setConversation([])');

      // Verify it does NOT ADD to conversation (pattern: prev => [...prev, item])
      // Clearing with setConversation([]) is OK, but we should not see prev => [...prev pattern
      expect(sessionStateHandler).not.toMatch(/setConversation\s*\(\s*prev\s*=>/);

      // Also verify processChunksToConversation does NOT handle SessionStateChange
      // (it's intentionally excluded - internal state is not part of conversation history)
      const processChunksFn = agentViewSource.match(
        /const processChunksToConversation[\s\S]*?return messages;\s*\};/
      );
      expect(processChunksFn).not.toBeNull();

      // SessionStateChange should be explicitly commented as not handled
      expect(processChunksFn![0]).toContain('SessionStateChange is intentionally NOT handled');
    });
  });

  describe('Scenario: UserNotification chunk displays message in conversation', () => {
    it('should have UserNotification variant with message and severity fields', () => {
      // @step Given a StreamChunk handler processes incoming chunks from Rust
      // Verify the type definition includes UserNotification variant
      expect(indexDts).toMatch(/\|\s*\{\s*type:\s*['"]UserNotification['"]/);
      expect(indexDts).toContain('message: string');
      expect(indexDts).toContain('severity: NotificationSeverity');
    });

    it('should handle UserNotification by adding status message to conversation', () => {
      // @step Given a StreamChunk handler processes incoming chunks from Rust
      // @step When Rust emits a UserNotification chunk with message 'API rate limit exceeded' and severity Warning
      // @step Then a status message 'API rate limit exceeded' is added to the conversation

      // There are two places UserNotification is handled:
      // 1. processChunksToConversation (pure function for replay) - uses messages.push
      // 2. handleStreamChunk (React callback for streaming) - uses setConversation

      // Check that handleStreamChunk (the streaming handler) adds to conversation
      const handleStreamChunkFn = agentViewSource.match(
        /const handleStreamChunk\s*=\s*useCallback\s*\(\s*\(chunk:\s*StreamChunk\)[\s\S]*?\n\s*\},\s*\[/
      );
      expect(handleStreamChunkFn).not.toBeNull();

      // Extract just the UserNotification handling within handleStreamChunk
      const handlerCode = handleStreamChunkFn![0];
      const userNotificationMatch = handlerCode.match(
        /chunk\.type\s*===\s*['"]UserNotification['"][\s\S]*?setConversation/
      );
      expect(userNotificationMatch).not.toBeNull();

      // Verify it uses chunk.message for the content
      expect(userNotificationMatch![0]).toContain('chunk.message');

      // Also check processChunksToConversation handles it (for replay/restore)
      const processChunksFn = agentViewSource.match(
        /const processChunksToConversation[\s\S]*?return messages;\s*\};/
      );
      expect(processChunksFn).not.toBeNull();

      // It should push a status message with chunk.message
      expect(processChunksFn![0]).toContain("chunk.type === 'UserNotification'");
      expect(processChunksFn![0]).toMatch(/type:\s*['"]status['"]/);
    });
  });

  describe('Scenario: Compacting state change does not appear in conversation', () => {
    it('should NOT have old status?: string pattern that conflated state with messages', () => {
      // @step Given I have an active session with conversation history

      // The old problematic pattern was: { type: 'Status', status: 'compacting' }
      // which couldn't distinguish internal state from user messages

      // Extract the StreamChunk type definition
      const streamChunkMatch = indexDts.match(
        /export type StreamChunk\s*=[\s\S]*?(?=\nexport (?!type StreamChunk))/
      );
      expect(streamChunkMatch).not.toBeNull();

      const streamChunkDef = streamChunkMatch![0];

      // Verify it's a discriminated union (type =), not an interface
      expect(streamChunkDef).toContain('export type StreamChunk =');

      // Verify there's no old 'status?: string' or 'status: string' field
      // that would allow arbitrary strings to leak through
      expect(streamChunkDef).not.toMatch(/status\??\s*:\s*string/);
    });

    it('should emit SessionStateChange for compaction state, not string-based Status', () => {
      // @step When I run the /compact command
      // @step Then no 'compacting' message appears in the conversation area
      // @step And the compaction progress is shown only in the input area placeholder

      // Verify session_manager.rs emits SessionStateChange for state transitions
      // This is checked by verifying the Rust types don't have a Status variant
      // and the TypeScript handler doesn't expect one
      expect(indexDts).not.toMatch(/\|\s*\{\s*type:\s*['"]Status['"]/);

      // SessionStateChange is the correct way to emit state
      expect(indexDts).toMatch(/\|\s*\{\s*type:\s*['"]SessionStateChange['"]/);
    });

    it('should have CompactionComplete variant for structured compaction results', () => {
      // UX-002: CompactionComplete provides structured metrics directly
      // No string parsing needed - compressionRatio is a number field
      expect(indexDts).toMatch(/\|\s*\{\s*type:\s*['"]CompactionComplete['"]/);
      expect(indexDts).toContain('compactionResult: CompactionResult');
    });

    it('should handle CompactionComplete by extracting metrics, NOT adding to conversation', () => {
      // CompactionComplete should:
      // 1. Delegate to shared handleCompactionComplete helper
      // 2. The helper extracts compressionRatio from compactionResult
      // 3. The helper calls setCompactionReduction with the percentage
      // 4. NOT call setConversation (compaction feedback is via input area indicator)

      // Verify the shared handleCompactionComplete helper exists and handles metrics
      const handleCompactionCompleteFn = agentViewSource.match(
        /const handleCompactionComplete\s*=\s*useCallback\s*\(\s*\n?\s*\([\s\S]*?\n\s*\],?\s*\)/
      );
      expect(handleCompactionCompleteFn).not.toBeNull();

      const helperCode = handleCompactionCompleteFn![0];

      // Verify the helper accesses compressionRatio from the result
      expect(helperCode).toContain('compressionRatio');

      // Verify the helper calls setCompactionReduction (for UI indicator)
      expect(helperCode).toContain('setCompactionReduction');

      // Verify the inline CompactionComplete block delegates to the shared helper
      const handleStreamChunkFn = agentViewSource.match(
        /const handleStreamChunk\s*=\s*useCallback\s*\(\s*\(chunk:\s*StreamChunk\)[\s\S]*?\n\s*\},\s*\[/
      );
      expect(handleStreamChunkFn).not.toBeNull();

      const handlerCode = handleStreamChunkFn![0];

      // Extract the CompactionComplete handling block
      const compactionCompleteMatch = handlerCode.match(
        /chunk\.type\s*===\s*['"]CompactionComplete['"][\s\S]*?(?=}\s*else\s*if)/
      );
      expect(compactionCompleteMatch).not.toBeNull();

      const compactionCompleteHandler = compactionCompleteMatch![0];

      // Verify it delegates to handleCompactionComplete (shared helper pattern)
      expect(compactionCompleteHandler).toContain('handleCompactionComplete');
      expect(compactionCompleteHandler).toContain('chunk.compactionResult');

      // Verify it does NOT call setConversation (no conversation pollution)
      expect(compactionCompleteHandler).not.toContain('setConversation');

      // Verify NO string parsing is used (the old broken pattern)
      expect(compactionCompleteHandler).not.toMatch(/\.match\s*\(/);
      expect(compactionCompleteHandler).not.toMatch(/\.includes\s*\(/);
      expect(compactionCompleteHandler).not.toMatch(/parseInt\s*\(/);
    });
  });

  describe('Scenario: StreamChunk handler uses exhaustive switch without string parsing', () => {
    it('should define StreamChunk as a discriminated union type, not an interface', () => {
      // @step Given the StreamChunk type is defined as a discriminated union with type field

      // Discriminated unions use: export type X = | { type: 'A' } | { type: 'B' }
      const isTypeAlias = indexDts.includes('export type StreamChunk =');
      const isInterface = indexDts.includes('export interface StreamChunk {');

      expect(isTypeAlias).toBe(true);
      expect(isInterface).toBe(false);

      // Verify it has union variants with pipe syntax
      expect(indexDts).toMatch(/\|\s*\{\s*type:\s*['"]/);
    });

    it('should NOT use string parsing (.includes, .match) for chunk type discrimination', () => {
      // @step When the TypeScript handler processes any StreamChunk variant
      // @step Then it uses a switch statement on chunk.type with no string includes or substring matching

      // Find all chunk.type comparisons in the handler
      // They should use === 'Type' pattern, not .includes() or .match()

      // Extract the main stream chunk handler (handleStreamChunk function)
      const handleStreamChunkFn = agentViewSource.match(
        /const handleStreamChunk\s*=\s*useCallback\s*\(\s*\(chunk:\s*StreamChunk\)[\s\S]*?\n\s*\},\s*\[/
      );
      expect(handleStreamChunkFn).not.toBeNull();

      const handlerCode = handleStreamChunkFn![0];

      // Verify it uses direct type comparison (chunk.type === 'X'), not string methods
      // These patterns would indicate fragile string parsing:
      expect(handlerCode).not.toMatch(/chunk\.type\.includes\s*\(/);
      expect(handlerCode).not.toMatch(/chunk\.type\.match\s*\(/);
      expect(handlerCode).not.toMatch(/chunk\.type\.indexOf\s*\(/);
      expect(handlerCode).not.toMatch(/chunk\.type\.startsWith\s*\(/);

      // Also verify no parsing of chunk.status (old pattern)
      expect(handlerCode).not.toMatch(/chunk\.status\.includes\s*\(/);
      expect(handlerCode).not.toMatch(/chunk\.status\.match\s*\(/);

      // Verify it DOES use direct equality comparisons
      expect(handlerCode).toMatch(/chunk\.type\s*===\s*['"]/);
    });

    it('should handle all discriminated union variants with if/else chain', () => {
      // Extract the main stream chunk handler
      const handleStreamChunkFn = agentViewSource.match(
        /const handleStreamChunk\s*=\s*useCallback\s*\(\s*\(chunk:\s*StreamChunk\)[\s\S]*?\n\s*\},\s*\[/
      );
      expect(handleStreamChunkFn).not.toBeNull();

      const handlerCode = handleStreamChunkFn![0];

      // Verify all main chunk types are handled
      const expectedTypes = [
        'Text',
        'Thinking',
        'ToolCall',
        'ToolResult',
        'Done',
        'SessionStateChange',
        'UserNotification',
        'CompactionComplete',  // UX-002: Structured compaction result
        'Interrupted',
        'TokenUpdate',
        'ContextFillUpdate',
        'ToolProgress',
        'Error',
      ];

      for (const type of expectedTypes) {
        expect(handlerCode).toContain(`chunk.type === '${type}'`);
      }
    });
  });
});
