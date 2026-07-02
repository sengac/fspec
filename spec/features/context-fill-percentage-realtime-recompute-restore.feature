@done
@tui
@rpc
@agent-view
@utils
@RPC-101
Feature: Context Fill Percentage Realtime Recompute Restore
  """
  TS utility side of the RPC-101 parallel fix.
  ExtractedTokenState (src/tui/utils/tokenStateUtils.ts:38-53) gained `contextThreshold: number | null` field.
  extractTokenStateFromChunks (lines 66-105) scans buffered StreamChunks for the last ContextFillUpdate, surfacing its threshold when finite & >0, else null.
  AgentView restore paths at AgentView.tsx:3661-3669 (session-switch) and 4287-4294 (service-result) seed cachedContextThresholdRef.current with extractedState.contextThreshold whenever non-null.
  Tests: src/tui/utils/__tests__/tokenStateUtils.test.ts:138-204 (3 cases: positive threshold, missing threshold, non-positive threshold).
  """

  Background: User Story
    As a developer resuming or switching sessions in the TypeScript Ink AgentView
    I want extractTokenStateFromChunks to surface the last known context-fill threshold so AgentView can prime its realtime-recompute cache on restore
    So that live TokenUpdates immediately move the [X%] badge for the restored session — without waiting for the next backend ContextFillUpdate

  Scenario: Session restore seeds the threshold cache so live TokenUpdates immediately move the badge
    Given a buffered session contains multiple ContextFillUpdate chunks, the last carrying threshold=100000 tokens
    When extractTokenStateFromChunks is called on the buffered chunks during session restore
    Then the returned ExtractedTokenState.contextThreshold MUST equal 100000
    Then AgentView MUST seed cachedContextThresholdRef.current with 100000 so the next TokenUpdate updates the badge without waiting for a backend ContextFillUpdate
    Then ContextFillUpdate without a threshold field MUST yield ExtractedTokenState.contextThreshold === null
