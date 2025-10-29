@metrics
@cli
@EST-002
Feature: AI token usage tracking
  """
  Architecture notes:
  - TODO: Add key architectural decisions
  - TODO: Add dependencies and integrations
  - TODO: Add critical implementation requirements
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Token recordings MUST be cumulative (adding to existing totals, never replacing)
  #   2. Token recordings MUST track input tokens, output tokens, model used, and timestamp
  #   3. Cost calculation MUST use model-specific pricing (Sonnet $3/MTok input, $15/MTok output)
  #   4. Token history MUST be preserved (append-only array) for audit trail
  #   5. Token recording is OPTIONAL but RECOMMENDED (not enforced by workflow)
  #   6. Failed attempts and rework MUST be tracked (all token usage counts, not just successful work)
  #   7. Purely manual recording. Agent decides when to record at meaningful checkpoints (no automatic prompts).
  #   8. Support multiple AI models with configurable pricing. Start with Claude Sonnet/Opus/Haiku, extensible for GPT-4/Gemini later.
  #
  # EXAMPLES:
  #   1. Agent completes BOARD-018 using 50K input, 40K output tokens with Sonnet → records: input=50000, output=40000, model=claude-sonnet-4, cost=$0.75
  #   2. Agent records tokens twice for same story (specifying: 30K/20K, implementing: 20K/20K) → totals: 50K input, 40K output, $0.75 total
  #   3. Show work unit EST-002 displays token summary: Total: 90K tokens, Cost: $0.75, Model: Sonnet, Recordings: 2
  #   4. Token summary command shows all work units: AUTH-001: 120K tokens ($1.20), DASH-002: 80K tokens ($0.60), Total: 200K tokens ($1.80)
  #   5. Agent switches from Sonnet to Opus mid-story → two recordings with different models → cost calculated per-model then summed
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should token recording be prompted automatically during state transitions, or purely manual?
  #   A: true
  #
  #   Q: Should we support bulk import (e.g., from Claude Code session logs) or only manual recording?
  #   A: true
  #
  #   Q: Do we need to track cache hits/misses (prompt caching can reduce input token costs significantly)?
  #   A: true
  #
  #   Q: Should we support other models beyond Claude (GPT-4, Gemini) with their pricing?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Only manual recording for MVP. Bulk import from session logs is future enhancement.
  #   2. Not for MVP. Track total tokens only. Prompt cache hit/miss tracking is future enhancement.
  #
  # ========================================
  Background: User Story
    As a AI agent working on fspec stories
    I want to record token usage cumulatively across the development lifecycle
    So that organizations can track AI resource consumption and cost per work unit
