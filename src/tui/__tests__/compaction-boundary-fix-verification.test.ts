/**
 * Feature: Compaction Boundary Index Fix Verification
 *
 * This test verifies that the fix to the boundary index calculation works correctly.
 * The fix changed from:
 *   compaction_boundary_index = metrics.turns_kept * 2
 * To:
 *   compaction_boundary_index = metrics.turns_summarized * 2
 *
 * This test uses actual NAPI bindings to verify the fix.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { randomUUID } from 'crypto';

import {
  persistenceSetDataDirectory,
  persistenceCreateSessionWithProvider,
  persistenceStoreMessageEnvelope,
  persistenceGetSessionMessageEnvelopes,
  persistenceSetCompactionState,
  sessionManagerCreateWithId,
  sessionRestoreMessages,
  sessionGetMergedOutput,
  sessionManagerDestroy,
} from '@sengac/codelet-napi';

// Helper to create a properly formatted message envelope
function createMessageEnvelope(
  role: 'user' | 'assistant',
  text: string
): object {
  return {
    uuid: randomUUID(),
    parentUuid: null,
    timestamp: new Date().toISOString(),
    type: role,
    provider: role === 'user' ? 'user' : 'anthropic',
    message: {
      role,
      content: [{ type: 'text', text }],
    },
    requestId: role === 'assistant' ? `req-${randomUUID()}` : null,
  };
}

describe('Feature: Compaction Boundary Index Fix Verification', () => {
  let tempDir: string;
  let sessionId: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(
      path.join(os.tmpdir(), 'fspec-boundary-fix-test-')
    );
    persistenceSetDataDirectory(tempDir);
    const session = persistenceCreateSessionWithProvider(
      'Boundary Fix Test',
      tempDir,
      'anthropic/claude-sonnet-4-20250514'
    );
    sessionId = session.id;
  });

  afterEach(() => {
    try {
      sessionManagerDestroy(sessionId);
    } catch {
      // Ignore
    }
    if (tempDir && fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('Scenario: Verify correct messages loaded after compaction', () => {
    it('should load only kept turns, not summarized turns', async () => {
      // @step Given I have 10 messages (5 turns) with distinct content
      const messages = [
        // Turn 1 (will be summarized)
        createMessageEnvelope('user', 'TURN1_USER: Hello, help me with setup'),
        createMessageEnvelope(
          'assistant',
          'TURN1_ASSISTANT: I will help with setup'
        ),
        // Turn 2 (will be summarized)
        createMessageEnvelope('user', 'TURN2_USER: Configure the database'),
        createMessageEnvelope(
          'assistant',
          'TURN2_ASSISTANT: Database configured'
        ),
        // Turn 3 (will be summarized)
        createMessageEnvelope('user', 'TURN3_USER: Set up authentication'),
        createMessageEnvelope('assistant', 'TURN3_ASSISTANT: Auth is set up'),
        // Turn 4 (will be KEPT)
        createMessageEnvelope('user', 'TURN4_USER: Now implement API'),
        createMessageEnvelope(
          'assistant',
          'TURN4_ASSISTANT: Here is the API code'
        ),
        // Turn 5 (will be KEPT)
        createMessageEnvelope('user', 'TURN5_USER: Add tests'),
        createMessageEnvelope('assistant', 'TURN5_ASSISTANT: Tests added'),
      ];

      for (const msg of messages) {
        persistenceStoreMessageEnvelope(sessionId, JSON.stringify(msg));
      }

      // Verify we have 10 messages
      const before = persistenceGetSessionMessageEnvelopes(sessionId);
      expect(before.length).toBe(10);

      // @step And I set compaction state simulating:
      // - turns_summarized = 3 (turns 1-3)
      // - turns_kept = 2 (turns 4-5)
      // - With the FIX: boundary = turns_summarized * 2 = 6
      // This means skip messages 0-5 (turns 1-3) and load messages 6-9 (turns 4-5)
      const summary = 'Summarized turns 1-3: Setup, database config, and auth';
      const boundaryIndex = 6; // = turns_summarized (3) * 2
      persistenceSetCompactionState(sessionId, summary, boundaryIndex);

      // @step When I restore the session
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4-20250514',
        tempDir,
        'Test Session'
      );

      const envelopes = persistenceGetSessionMessageEnvelopes(sessionId);
      await sessionRestoreMessages(sessionId, envelopes);

      // @step Then I should have 5 items: 1 summary + 4 kept messages (turns 4-5)
      expect(envelopes.length).toBe(5);

      // @step And the merged output should contain the summary and ONLY turns 4-5
      const mergedOutput = sessionGetMergedOutput(sessionId);
      const userInputs = mergedOutput.filter(
        (c: { type: string }) => c.type === 'UserInput'
      );
      const textOutputs = mergedOutput.filter(
        (c: { type: string }) => c.type === 'Text'
      );

      // Should have: summary + TURN4_USER + TURN5_USER = 3 UserInputs
      expect(userInputs.length).toBe(3);

      // First should be the summary
      expect((userInputs[0] as { text: string }).text).toContain(
        '[Previous conversation summary]'
      );
      expect((userInputs[0] as { text: string }).text).toContain(
        'Summarized turns 1-3'
      );

      // Second should be TURN4_USER
      expect((userInputs[1] as { text: string }).text).toBe(
        'TURN4_USER: Now implement API'
      );

      // Third should be TURN5_USER
      expect((userInputs[2] as { text: string }).text).toBe(
        'TURN5_USER: Add tests'
      );

      // @step And the summarized turns (1-3) should NOT be in the output
      const allText = userInputs.map((u: { text: string }) => u.text).join(' ');
      expect(allText).not.toContain('TURN1_USER');
      expect(allText).not.toContain('TURN2_USER');
      expect(allText).not.toContain('TURN3_USER');

      // @step And the assistant responses should only be from turns 4-5
      expect(textOutputs.length).toBe(2);
      expect((textOutputs[0] as { text: string }).text).toBe(
        'TURN4_ASSISTANT: Here is the API code'
      );
      expect((textOutputs[1] as { text: string }).text).toBe(
        'TURN5_ASSISTANT: Tests added'
      );

      console.log('\n=== BOUNDARY FIX VERIFICATION PASSED ===');
      console.log('With corrected boundary index (turns_summarized * 2):');
      console.log('  - Summary loaded: ✓');
      console.log('  - Turns 4-5 (kept) loaded: ✓');
      console.log('  - Turns 1-3 (summarized) NOT loaded: ✓');
      console.log('=========================================\n');
    });
  });

  describe('Scenario: Edge case - all turns summarized except last', () => {
    it('should work when only 1 turn is kept', async () => {
      // @step Given I have 8 messages (4 turns)
      for (let i = 0; i < 8; i++) {
        const isUser = i % 2 === 0;
        const turnNum = Math.floor(i / 2) + 1;
        const msg = createMessageEnvelope(
          isUser ? 'user' : 'assistant',
          `TURN${turnNum}_${isUser ? 'USER' : 'ASSISTANT'}`
        );
        persistenceStoreMessageEnvelope(sessionId, JSON.stringify(msg));
      }

      // @step And I keep only the last turn (turn 4)
      // turns_summarized = 3, turns_kept = 1
      // boundary = 3 * 2 = 6
      persistenceSetCompactionState(sessionId, 'Summary of turns 1-3', 6);

      // @step When I restore
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4-20250514',
        tempDir,
        'Test Session'
      );

      const envelopes = persistenceGetSessionMessageEnvelopes(sessionId);
      await sessionRestoreMessages(sessionId, envelopes);

      // @step Then I should have: 1 summary + 2 messages (turn 4)
      expect(envelopes.length).toBe(3);

      const mergedOutput = sessionGetMergedOutput(sessionId);
      const userInputs = mergedOutput.filter(
        (c: { type: string }) => c.type === 'UserInput'
      );

      // Should have: summary + TURN4_USER = 2 UserInputs
      expect(userInputs.length).toBe(2);
      expect((userInputs[0] as { text: string }).text).toContain(
        '[Previous conversation summary]'
      );
      expect((userInputs[1] as { text: string }).text).toBe('TURN4_USER');
    });
  });

  describe('Scenario: Edge case - no turns summarized', () => {
    it('should work when no compaction happened', async () => {
      // @step Given I have 4 messages (2 turns)
      for (let i = 0; i < 4; i++) {
        const isUser = i % 2 === 0;
        const turnNum = Math.floor(i / 2) + 1;
        const msg = createMessageEnvelope(
          isUser ? 'user' : 'assistant',
          `TURN${turnNum}_${isUser ? 'USER' : 'ASSISTANT'}`
        );
        persistenceStoreMessageEnvelope(sessionId, JSON.stringify(msg));
      }

      // @step And NO compaction state is set (all turns kept)
      // Don't call persistenceSetCompactionState

      // @step When I restore
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4-20250514',
        tempDir,
        'Test Session'
      );

      const envelopes = persistenceGetSessionMessageEnvelopes(sessionId);
      await sessionRestoreMessages(sessionId, envelopes);

      // @step Then I should have all 4 messages (no summary)
      expect(envelopes.length).toBe(4);

      const mergedOutput = sessionGetMergedOutput(sessionId);
      const userInputs = mergedOutput.filter(
        (c: { type: string }) => c.type === 'UserInput'
      );

      // Should have: TURN1_USER + TURN2_USER = 2 UserInputs (no summary)
      expect(userInputs.length).toBe(2);
      expect((userInputs[0] as { text: string }).text).toBe('TURN1_USER');
      expect((userInputs[1] as { text: string }).text).toBe('TURN2_USER');
    });
  });
});
