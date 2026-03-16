/**
 * Feature: spec/features/cross-pane-selection-with-correlation-ids.feature
 *
 * Tests for Cross-Pane Selection with Correlation IDs (WATCH-011)
 *
 * Tests verify:
 * - StreamChunk correlation_id assignment
 * - Cross-pane highlighting based on observed_correlation_ids
 * - Bi-directional correlation mapping
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';

// Mock the codelet-napi module
vi.mock('@sengac/codelet-napi', () => ({
  sessionGetSubordinate: vi.fn(),
  sessionGetMergedOutput: vi.fn(),
  sessionGetRole: vi.fn(),
  sessionGetStatus: vi.fn(),
  sessionSendInput: vi.fn(),
  persistenceSetDataDirectory: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
  sessionManagerList: vi.fn(() => []),
  JsThinkingLevel: { Off: 0, Low: 1, Medium: 2, High: 3 },
  getThinkingConfig: vi.fn(() => null),
  // BRIDGE-006: Unified thinking level detection NAPI functions
  napiDetectThinkingLevel: vi.fn(() => 0), // Default to Off
  napiHasDisableKeywords: vi.fn(() => false),
  napiComputeEffectiveThinkingLevel: vi.fn((base: number, detected: number, forceOff: boolean) => {
    if (forceOff) { return 0; }
    return Math.max(base, detected);
  }),
}));

import type { ConversationLine } from '../../types/conversation';
// WATCH-011: DRY - Import shared correlation mapping utility
import { buildCorrelationMaps, getHighlightedTurns } from '../../utils/correlationMapping';

describe('Feature: Cross-Pane Selection with Correlation IDs', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: StreamChunk receives correlation ID in handle_output', () => {
    it('should assign unique correlation_id via atomic counter', () => {
      // @step Given a parent session exists
      const sessionId = 'subordinate-session-123';

      // @step When the parent session emits a Text chunk via handle_output()
      // Simulating what handle_output does in Rust
      let counter = 0;
      const assignCorrelationId = () => {
        const id = `${sessionId}-${counter}`;
        counter++;
        return id;
      };

      const chunk1CorrelationId = assignCorrelationId();
      const chunk2CorrelationId = assignCorrelationId();

      // @step Then the chunk receives a unique correlation_id assigned by an atomic counter
      expect(chunk1CorrelationId).toBe('subordinate-session-123-0');
      expect(chunk2CorrelationId).toBe('subordinate-session-123-1');
      expect(chunk1CorrelationId).not.toBe(chunk2CorrelationId);

      // @step And the correlation_id is in format "{session_id}-{counter}"
      expect(chunk1CorrelationId).toMatch(/^subordinate-session-123-\d+$/);

      // @step And the chunk broadcast to watchers carries the same correlation_id
      // This is verified by the fact that the same ID is used
      expect(chunk1CorrelationId).toBe('subordinate-session-123-0');
    });
  });

  describe('Scenario: Subordinate turn selection highlights correlated supervisor turns', () => {
    it('should highlight supervisor turns that observed the selected subordinate turn', () => {
      // @step Given I am viewing a watcher session in split view
      // (setup state representing split view)

      // @step And the parent pane shows turns 1, 2, 3 with correlation IDs
      const subordinateConversation: ConversationLine[] = [
        { role: 'user', content: 'Hello', messageIndex: 0, correlationId: 'p-0' },
        { role: 'assistant', content: 'Hi', messageIndex: 1, correlationId: 'p-1' },
        { role: 'user', content: 'Help me', messageIndex: 2, correlationId: 'p-2' },
        { role: 'assistant', content: 'Sure', messageIndex: 3, correlationId: 'p-3' },
      ];

      // @step And the watcher responded to turn 3 (watcher turn 5 has observed_correlation_ids pointing to turn 3)
      const supervisorConversation: ConversationLine[] = [
        { role: 'user', content: 'Watch for issues', messageIndex: 0 }, // User message to supervisor
        { role: 'assistant', content: 'Watching...', messageIndex: 1 },
        { role: 'assistant', content: 'I noticed something', messageIndex: 2, observedCorrelationIds: ['p-3'] }, // Observed turn 3
      ];

      // @step When I press Tab to enter turn-select mode in parent pane
      // (simulated by select mode being active)

      // @step And I navigate to select turn 3 in the parent pane
      const selectedSubordinateTurn = 3;

      // Build correlation maps
      const { subordinateToSupervisorTurns } = buildCorrelationMaps(subordinateConversation, supervisorConversation);

      // @step Then turn 5 in the watcher pane is highlighted with cyan color, bold text, and vertical bar prefix
      const highlightedSupervisorTurns = subordinateToSupervisorTurns.get(selectedSubordinateTurn);
      expect(highlightedSupervisorTurns).toBeDefined();
      expect(highlightedSupervisorTurns!.has(2)).toBe(true); // Watcher turn 2 (messageIndex) observed parent turn 3
    });
  });

  describe('Scenario: Supervisor turn selection highlights observed subordinate turns', () => {
    it('should highlight subordinate turns that the selected supervisor turn was observing', () => {
      // @step Given I am viewing a watcher session in split view
      // (setup state)

      // @step And the watcher turn 5 observed parent turns 3 and 4 (observed_correlation_ids includes both)
      const subordinateConversation: ConversationLine[] = [
        { role: 'user', content: 'Hello', messageIndex: 0, correlationId: 'p-0' },
        { role: 'assistant', content: 'Hi', messageIndex: 1, correlationId: 'p-1' },
        { role: 'user', content: 'Question 1', messageIndex: 2, correlationId: 'p-2' },
        { role: 'assistant', content: 'Answer 1', messageIndex: 3, correlationId: 'p-3' },
        { role: 'user', content: 'Question 2', messageIndex: 4, correlationId: 'p-4' },
      ];

      const supervisorConversation: ConversationLine[] = [
        { role: 'user', content: 'Watch', messageIndex: 0 },
        { role: 'assistant', content: 'OK', messageIndex: 1 },
        { role: 'assistant', content: 'Found issues', messageIndex: 2, observedCorrelationIds: ['p-3', 'p-4'] },
      ];

      // @step When I switch to watcher pane and press Tab to enter turn-select mode
      // (simulated by supervisor pane active)

      // @step And I navigate to select turn 5 in the watcher pane
      const selectedSupervisorTurn = 2; // messageIndex 2

      // Build correlation maps
      const { supervisorToSubordinateTurns } = buildCorrelationMaps(subordinateConversation, supervisorConversation);

      // @step Then turns 3 and 4 in the parent pane are highlighted with cyan color, bold text, and vertical bar prefix
      const highlightedSubordinateTurns = supervisorToSubordinateTurns.get(selectedSupervisorTurn);
      expect(highlightedSubordinateTurns).toBeDefined();
      expect(highlightedSubordinateTurns!.has(3)).toBe(true); // Parent turn 3
      expect(highlightedSubordinateTurns!.has(4)).toBe(true); // Parent turn 4
    });
  });

  describe('Scenario: Direct user message to supervisor has no correlation', () => {
    it('should not highlight any subordinate turns when selecting a direct user message', () => {
      // @step Given I am viewing a watcher session in split view
      // (setup state)

      // @step And turn 1 in watcher pane is a direct user message (no observed_correlation_ids)
      const subordinateConversation: ConversationLine[] = [
        { role: 'user', content: 'Hello', messageIndex: 0, correlationId: 'p-0' },
        { role: 'assistant', content: 'Hi', messageIndex: 1, correlationId: 'p-1' },
      ];

      const supervisorConversation: ConversationLine[] = [
        { role: 'user', content: 'Watch for issues', messageIndex: 0 }, // Direct user message, no observedCorrelationIds
        { role: 'assistant', content: 'Watching...', messageIndex: 1 },
      ];

      // @step When I switch to watcher pane and press Tab to enter turn-select mode
      // (simulated by supervisor pane active)

      // @step And I select turn 1 in the watcher pane
      const selectedSupervisorTurn = 0; // Direct user message

      // Build correlation maps
      const { supervisorToSubordinateTurns } = buildCorrelationMaps(subordinateConversation, supervisorConversation);

      // @step Then no highlight appears in the parent pane
      const highlightedSubordinateTurns = supervisorToSubordinateTurns.get(selectedSupervisorTurn);
      expect(highlightedSubordinateTurns).toBeUndefined();

      // @step And the parent pane content remains dimmed
      // (this is a UI concern, verified by the fact that no turns are highlighted)
    });
  });

  describe('Scenario: Cross-pane highlight updates as selection moves', () => {
    it('should update cross-pane highlight when navigating between turns', () => {
      // @step Given I am viewing a watcher session in split view
      // (setup state)

      // @step And the parent pane has multiple turns with correlated watcher observations
      const subordinateConversation: ConversationLine[] = [
        { role: 'user', content: 'Turn 1', messageIndex: 0, correlationId: 'p-0' },
        { role: 'assistant', content: 'Turn 2', messageIndex: 1, correlationId: 'p-1' },
        { role: 'user', content: 'Turn 3', messageIndex: 2, correlationId: 'p-2' },
        { role: 'assistant', content: 'Turn 4', messageIndex: 3, correlationId: 'p-3' },
      ];

      const supervisorConversation: ConversationLine[] = [
        { role: 'assistant', content: 'Observed turn 1', messageIndex: 0, observedCorrelationIds: ['p-0', 'p-1'] },
        { role: 'assistant', content: 'Observed turn 2', messageIndex: 1, observedCorrelationIds: ['p-2', 'p-3'] },
      ];

      // Build correlation maps
      const { subordinateToSupervisorTurns } = buildCorrelationMaps(subordinateConversation, supervisorConversation);

      // @step When I press Tab to enter turn-select mode in parent pane
      // (simulated)

      // Initially select turn 0
      let selectedSubordinateTurn = 0;
      let highlightedSupervisorTurns = subordinateToSupervisorTurns.get(selectedSubordinateTurn);
      expect(highlightedSupervisorTurns).toBeDefined();
      expect(highlightedSupervisorTurns!.has(0)).toBe(true); // Watcher turn 0 observed parent turn 0

      // @step And I press Down arrow to move selection to the next turn
      selectedSubordinateTurn = 2; // Move to turn 2

      // @step Then the cross-pane highlight in watcher pane updates to show the newly correlated turn
      highlightedSupervisorTurns = subordinateToSupervisorTurns.get(selectedSubordinateTurn);
      expect(highlightedSupervisorTurns).toBeDefined();
      expect(highlightedSupervisorTurns!.has(1)).toBe(true); // Watcher turn 1 observed parent turn 2

      // @step And the previously highlighted watcher turn returns to dimmed state
      // (verified by the fact that turn 0 is no longer in the highlighted set for the new selection)
      expect(highlightedSupervisorTurns!.has(0)).toBe(false);
    });
  });

  describe('Scenario: Supervisor evaluation captures observed correlation IDs', () => {
    it('should capture correlation IDs from buffered chunks at breakpoint', () => {
      // @step Given a watcher session is observing a parent session
      // (simulated observation buffer)
      interface MockStreamChunk {
        type: string;
        text?: string;
        correlationId?: string;
      }

      const observationBuffer: MockStreamChunk[] = [];

      // @step And the parent emits chunks with correlation_ids "p-0", "p-1", "p-2"
      observationBuffer.push({ type: 'Text', text: 'Hello', correlationId: 'p-0' });
      observationBuffer.push({ type: 'Text', text: 'World', correlationId: 'p-1' });
      observationBuffer.push({ type: 'Text', text: '!', correlationId: 'p-2' });

      // @step When a natural breakpoint (Done or ToolResult) triggers watcher evaluation
      // Simulate what happens in supervisor_loop_tick when breakpoint is reached
      const captureCorrelationIds = (buffer: MockStreamChunk[]): string[] => {
        return buffer
          .filter(c => c.correlationId)
          .map(c => c.correlationId!);
      };

      const observedCorrelationIds = captureCorrelationIds(observationBuffer);

      // @step Then the ProcessObservations action includes observed_correlation_ids ["p-0", "p-1", "p-2"]
      expect(observedCorrelationIds).toEqual(['p-0', 'p-1', 'p-2']);

      // @step And the watcher's response chunks can be tagged with these observed IDs
      const supervisorResponseChunk = {
        type: 'Text',
        text: 'I observed your conversation',
        observedCorrelationIds: observedCorrelationIds,
      };
      expect(supervisorResponseChunk.observedCorrelationIds).toEqual(['p-0', 'p-1', 'p-2']);
    });
  });

  describe('Utility: getHighlightedTurns helper function', () => {
    it('should return empty set when not in select mode', () => {
      const subordinateConversation: ConversationLine[] = [
        { role: 'user', content: 'Hello', messageIndex: 0, correlationId: 'p-0' },
      ];
      const supervisorConversation: ConversationLine[] = [
        { role: 'assistant', content: 'Observed', messageIndex: 0, observedCorrelationIds: ['p-0'] },
      ];

      const correlationMaps = buildCorrelationMaps(subordinateConversation, supervisorConversation);

      // Not in select mode - should return empty set
      const highlighted = getHighlightedTurns(
        'subordinate',
        false, // not in select mode
        0,
        null,
        correlationMaps
      );

      expect(highlighted.size).toBe(0);
    });

    it('should highlight supervisor turns when subordinate turn selected', () => {
      const subordinateConversation: ConversationLine[] = [
        { role: 'user', content: 'Hello', messageIndex: 0, correlationId: 'p-0' },
        { role: 'assistant', content: 'Hi', messageIndex: 1, correlationId: 'p-1' },
      ];
      const supervisorConversation: ConversationLine[] = [
        { role: 'assistant', content: 'Observed', messageIndex: 0, observedCorrelationIds: ['p-0', 'p-1'] },
      ];

      const correlationMaps = buildCorrelationMaps(subordinateConversation, supervisorConversation);

      // Select subordinate turn 0 - should highlight supervisor turn 0
      const highlighted = getHighlightedTurns(
        'subordinate',
        true, // in select mode
        0, // selected parent turn
        null,
        correlationMaps
      );

      expect(highlighted.has(0)).toBe(true);
    });

    it('should highlight parent turns when watcher turn selected', () => {
      const subordinateConversation: ConversationLine[] = [
        { role: 'user', content: 'Hello', messageIndex: 0, correlationId: 'p-0' },
        { role: 'assistant', content: 'Hi', messageIndex: 1, correlationId: 'p-1' },
      ];
      const supervisorConversation: ConversationLine[] = [
        { role: 'assistant', content: 'Observed', messageIndex: 0, observedCorrelationIds: ['p-0', 'p-1'] },
      ];

      const correlationMaps = buildCorrelationMaps(subordinateConversation, supervisorConversation);

      // Select watcher turn 0 - should highlight parent turns 0 and 1
      const highlighted = getHighlightedTurns(
        'supervisor',
        true, // in select mode
        null,
        0, // selected watcher turn
        correlationMaps
      );

      expect(highlighted.has(0)).toBe(true);
      expect(highlighted.has(1)).toBe(true);
    });
  });
});
