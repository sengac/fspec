/**
 * Feature: sessionRestoreMessages with Compaction - NAPI Integration Tests
 *
 * These tests verify the COMPLETE flow:
 * 1. Create a session with messages
 * 2. Set compaction state
 * 3. Create a background session
 * 4. Call sessionRestoreMessages with envelopes
 * 5. Verify the rig messages contain the compaction summary
 *
 * This tests the ACTUAL sessionRestoreMessages function, not a simulation.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { randomUUID } from 'crypto';

// Import the ACTUAL NAPI bindings
import {
  persistenceSetDataDirectory,
  persistenceCreateSessionWithProvider,
  persistenceStoreMessageEnvelope,
  persistenceGetSessionMessageEnvelopes,
  persistenceSetCompactionState,
  persistenceLoadSession,
  sessionManagerCreateWithId,
  sessionRestoreMessages,
  sessionGetMergedOutput,
  sessionManagerDestroy,
} from '@sengac/codelet-napi';

// Helper to create a properly formatted message envelope
function createMessageEnvelope(
  role: 'user' | 'assistant',
  text: string,
  provider: string = role === 'user' ? 'user' : 'anthropic'
): object {
  return {
    uuid: randomUUID(),
    parentUuid: null,
    timestamp: new Date().toISOString(),
    type: role,
    provider,
    message: {
      role,
      content: [{ type: 'text', text }],
    },
    requestId: role === 'assistant' ? `req-${randomUUID()}` : null,
  };
}

describe('Feature: sessionRestoreMessages with Compaction - Full Flow', () => {
  let tempDir: string;
  let sessionId: string;

  beforeEach(() => {
    // Create a temporary directory for test data
    tempDir = fs.mkdtempSync(
      path.join(os.tmpdir(), 'fspec-restore-messages-test-')
    );

    // Set the persistence data directory to our temp dir
    persistenceSetDataDirectory(tempDir);

    // Create a test session in persistence
    const session = persistenceCreateSessionWithProvider(
      'Restore Messages Test Session',
      tempDir,
      'anthropic/claude-sonnet-4-20250514'
    );
    sessionId = session.id;
  });

  afterEach(async () => {
    // Destroy the background session if it exists
    try {
      sessionManagerDestroy(sessionId);
    } catch {
      // Session might not exist, that's ok
    }

    // Clean up temp directory
    if (tempDir && fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Restore messages WITHOUT compaction', () => {
    it('should restore all messages to output buffer', async () => {
      // @step Given I store 4 messages
      const messages = [
        createMessageEnvelope('user', 'First user message'),
        createMessageEnvelope('assistant', 'First assistant response'),
        createMessageEnvelope('user', 'Second user message'),
        createMessageEnvelope('assistant', 'Second assistant response'),
      ];

      for (const msg of messages) {
        persistenceStoreMessageEnvelope(sessionId, JSON.stringify(msg));
      }

      // @step When I create a background session and restore messages
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4-20250514',
        tempDir,
        'Test Session'
      );

      const envelopes = persistenceGetSessionMessageEnvelopes(sessionId);
      expect(envelopes.length).toBe(4);

      await sessionRestoreMessages(sessionId, envelopes);

      // @step Then the output buffer should contain the restored conversation
      const mergedOutput = sessionGetMergedOutput(sessionId);

      // Should have UserInput, Text, Done, UserInput, Text, Done
      expect(mergedOutput.length).toBeGreaterThan(0);

      // Find the user inputs
      const userInputs = mergedOutput.filter(
        (c: { type: string }) => c.type === 'UserInput'
      );
      expect(userInputs.length).toBe(2);
      expect((userInputs[0] as { text: string }).text).toBe(
        'First user message'
      );
      expect((userInputs[1] as { text: string }).text).toBe(
        'Second user message'
      );

      // Find the text outputs
      const textOutputs = mergedOutput.filter(
        (c: { type: string }) => c.type === 'Text'
      );
      expect(textOutputs.length).toBe(2);
      expect((textOutputs[0] as { text: string }).text).toBe(
        'First assistant response'
      );
      expect((textOutputs[1] as { text: string }).text).toBe(
        'Second assistant response'
      );
    });
  });

  describe('Scenario: Restore messages WITH compaction', () => {
    it('should restore compaction summary as first message in output buffer', async () => {
      // @step Given I store 6 messages (3 turns)
      for (let i = 0; i < 6; i++) {
        const isUser = i % 2 === 0;
        const msg = createMessageEnvelope(
          isUser ? 'user' : 'assistant',
          `Compaction test message ${i}`
        );
        persistenceStoreMessageEnvelope(sessionId, JSON.stringify(msg));
      }

      // @step And I set compaction state at index 4 (2 turns compacted)
      const compactionSummary =
        'Previous conversation discussed: 1) Initial setup 2) Configuration details';
      persistenceSetCompactionState(sessionId, compactionSummary, 4);

      // Verify compaction is set
      const manifest = persistenceLoadSession(sessionId);
      expect(manifest.compaction).toBeDefined();
      expect(manifest.compaction?.compactedBeforeIndex).toBe(4);

      // @step When I create a background session and restore messages
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4-20250514',
        tempDir,
        'Test Session'
      );

      // Get envelopes (should respect compaction)
      const envelopes = persistenceGetSessionMessageEnvelopes(sessionId);

      // Should have: 1 synthetic summary + 2 post-compaction messages
      expect(envelopes.length).toBe(3);

      // Verify the first envelope is the synthetic summary
      const firstEnvelope = JSON.parse(envelopes[0]);
      expect(firstEnvelope._synthetic).toBe(true);
      expect(firstEnvelope._compactionSummary).toBe(true);

      // Now restore messages
      await sessionRestoreMessages(sessionId, envelopes);

      // @step Then the output buffer should contain the compaction summary
      const mergedOutput = sessionGetMergedOutput(sessionId);

      expect(mergedOutput.length).toBeGreaterThan(0);

      // The first chunk should be the compaction summary as UserInput
      const userInputs = mergedOutput.filter(
        (c: { type: string }) => c.type === 'UserInput'
      );

      // Should have: compaction summary + 1 post-compaction user message
      expect(userInputs.length).toBe(2);

      // First user input should be the compaction summary
      const firstUserInput = userInputs[0] as { text: string };
      expect(firstUserInput.text).toContain('[Previous conversation summary]');
      expect(firstUserInput.text).toContain('Initial setup');
      expect(firstUserInput.text).toContain('Configuration details');

      // Second should be the post-compaction message
      const secondUserInput = userInputs[1] as { text: string };
      expect(secondUserInput.text).toBe('Compaction test message 4');
    });

    it('should include compaction summary in rig messages for LLM context', async () => {
      // This test verifies that the rig_messages vector in Rust contains the summary
      // We can't directly access rig_messages, but we can verify the behavior through
      // the output buffer which is populated from the same parsing logic

      // @step Given a compacted session
      for (let i = 0; i < 4; i++) {
        const isUser = i % 2 === 0;
        const msg = createMessageEnvelope(
          isUser ? 'user' : 'assistant',
          `LLM context test ${i}`
        );
        persistenceStoreMessageEnvelope(sessionId, JSON.stringify(msg));
      }

      const llmContextSummary =
        'This is the compaction summary that MUST appear in LLM context for continuity.';
      persistenceSetCompactionState(sessionId, llmContextSummary, 2);

      // @step When I restore the session
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4-20250514',
        tempDir,
        'Test Session'
      );

      const envelopes = persistenceGetSessionMessageEnvelopes(sessionId);
      await sessionRestoreMessages(sessionId, envelopes);

      // @step Then the compaction summary should be in the output buffer
      const mergedOutput = sessionGetMergedOutput(sessionId);
      const userInputs = mergedOutput.filter(
        (c: { type: string }) => c.type === 'UserInput'
      );

      // Verify the summary is present
      const summaryFound = userInputs.some((input: { text: string }) =>
        input.text.includes('MUST appear in LLM context')
      );

      expect(summaryFound).toBe(true);

      // @step And the summary should be the FIRST user input (for proper LLM context ordering)
      const firstUserInput = userInputs[0] as { text: string };
      expect(firstUserInput.text).toContain('[Previous conversation summary]');
      expect(firstUserInput.text).toContain(llmContextSummary);
    });
  });

  describe('Scenario: Debug - trace the exact compaction flow', () => {
    it('should trace each step of compaction restoration', async () => {
      // This test is for debugging - it logs each step to understand the flow

      console.log('\n=== COMPACTION RESTORATION DEBUG TRACE ===\n');

      // Step 1: Create messages
      console.log('Step 1: Creating 6 messages...');
      for (let i = 0; i < 6; i++) {
        const isUser = i % 2 === 0;
        const msg = createMessageEnvelope(
          isUser ? 'user' : 'assistant',
          `Debug message ${i}`
        );
        persistenceStoreMessageEnvelope(sessionId, JSON.stringify(msg));
      }

      // Step 2: Verify messages stored
      const beforeCompaction = persistenceGetSessionMessageEnvelopes(sessionId);
      console.log(
        `Step 2: Messages before compaction: ${beforeCompaction.length}`
      );
      expect(beforeCompaction.length).toBe(6);

      // Step 3: Set compaction state
      const summary = 'DEBUG: This is the compaction summary text.';
      console.log(
        `Step 3: Setting compaction state at index 4 with summary: "${summary}"`
      );
      persistenceSetCompactionState(sessionId, summary, 4);

      // Step 4: Verify compaction state in manifest
      const manifest = persistenceLoadSession(sessionId);
      console.log(`Step 4: Manifest compaction state:`, manifest.compaction);
      expect(manifest.compaction).toBeDefined();

      // Step 5: Get envelopes AFTER compaction
      const afterCompaction = persistenceGetSessionMessageEnvelopes(sessionId);
      console.log(
        `Step 5: Envelopes after compaction: ${afterCompaction.length}`
      );
      console.log('Step 5: First envelope (should be synthetic):');
      const firstEnv = JSON.parse(afterCompaction[0]);
      console.log(`  - uuid: ${firstEnv.uuid}`);
      console.log(`  - _synthetic: ${firstEnv._synthetic}`);
      console.log(`  - _compactionSummary: ${firstEnv._compactionSummary}`);
      console.log(`  - message.role: ${firstEnv.message.role}`);
      console.log(
        `  - message.content[0].text (first 100 chars): ${firstEnv.message.content[0].text.substring(0, 100)}`
      );

      expect(afterCompaction.length).toBe(3); // 1 synthetic + 2 post-compaction
      expect(firstEnv._synthetic).toBe(true);
      expect(firstEnv._compactionSummary).toBe(true);

      // Step 6: Create background session
      console.log('\nStep 6: Creating background session...');
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4-20250514',
        tempDir,
        'Debug Test Session'
      );

      // Step 7: Restore messages
      console.log('Step 7: Calling sessionRestoreMessages...');
      await sessionRestoreMessages(sessionId, afterCompaction);

      // Step 8: Get merged output
      console.log('Step 8: Getting merged output from session...');
      const mergedOutput = sessionGetMergedOutput(sessionId);
      console.log(`Step 8: Merged output chunks: ${mergedOutput.length}`);

      // Step 9: Analyze output
      console.log('\nStep 9: Analyzing output chunks:');
      for (let i = 0; i < mergedOutput.length; i++) {
        const chunk = mergedOutput[i] as { type: string; text?: string };
        const textPreview = chunk.text?.substring(0, 80) || 'N/A';
        console.log(`  [${i}] type=${chunk.type}, text="${textPreview}..."`);
      }

      // Step 10: Verify compaction summary is present
      const userInputChunks = mergedOutput.filter(
        (c: { type: string }) => c.type === 'UserInput'
      );
      console.log(`\nStep 10: UserInput chunks: ${userInputChunks.length}`);

      const summaryChunk = userInputChunks.find((c: { text: string }) =>
        c.text.includes('[Previous conversation summary]')
      );

      if (summaryChunk) {
        console.log('Step 10: ✓ FOUND compaction summary in output!');
        console.log(
          `  Summary text: "${(summaryChunk as { text: string }).text.substring(0, 150)}..."`
        );
      } else {
        console.log('Step 10: ✗ COMPACTION SUMMARY NOT FOUND IN OUTPUT!');
        console.log('  Available UserInput texts:');
        userInputChunks.forEach((c: { text: string }, i: number) => {
          console.log(`    [${i}] "${c.text.substring(0, 80)}..."`);
        });
      }

      expect(summaryChunk).toBeDefined();

      console.log('\n=== END DEBUG TRACE ===\n');
    });
  });
});
