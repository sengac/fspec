@done
@PROV-041
@providers
@provider-abstraction
Feature: Thinking token exhaustion recovery — detect budget depletion and preserve context across all providers
  """
  VTCode's 6-level ReasoningEffortLevel enum (None/Minimal/Low/Medium/High/XHigh) maps to provider-specific values: Anthropic → explicit token counts (1024/4096/8192/16384/32768), OpenAI → effort strings (low/medium/high/xhigh), Gemini → thinking_level strings (minimal/low/medium/high). Our implementation should mirror this abstraction.
  Detection heuristic: FinishReason::Length + (reasoning_tokens > 0 && output_tokens < threshold). Threshold should be configurable but default to ~50 tokens. The key insight is that thinking exhaustion produces a response where the model 'thought a lot but said almost nothing'.
  Recovery logic should live in the stream loop (provider-agnostic layer) — same location as PROV-040's truncated tool call recovery. The thinking exhaustion counter and session-level reasoning effort tracker are turn-level and session-level state respectively.
  VTCode uses budget clamping as primary prevention: effective_budget = min(budget, max_tokens - 100). This should be our first line of defense too, but unlike VTCode, we add active recovery as a second line when prevention fails.
  For context preservation: VTCode's SessionMemoryEnvelope pattern (grounded facts + task summary + touched files + history artifact path) is the gold standard. On thinking exhaustion near context limits, persist a memory envelope BEFORE retrying, ensuring recovery via SessionSearch even if the retry fails.
  Anthropic Opus 4.6 uses ThinkingConfig::Adaptive (model self-manages budget). For Adaptive models, the retry strategy should NOT try to set explicit budgets — instead, inject a system hint like 'Keep your thinking concise for this request' and rely on the model's own budget management.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a response terminates with FinishReason::Length AND the response has reasoning/thinking content but empty or near-empty output content, the system MUST detect this as 'thinking exhaustion' (distinct from regular output truncation)
  #   2. On thinking exhaustion detection, the system MUST preserve the captured thinking content (reasoning field from LLMResponse) as context for the retry — not discard it
  #   3. On retry after thinking exhaustion, the system MUST reduce the thinking budget (e.g., halve it or drop one ReasoningEffortLevel) and increase max_tokens for output — the ratio must shift toward output
  #   4. A thinking exhaustion retry budget (max 2 consecutive thinking-exhaustion retries per turn) MUST prevent infinite retry loops — after budget exhausted, use the best partial response available or disable thinking for the turn
  #   5. Detection and recovery MUST work for all providers: Anthropic (ThinkingDelta + max_tokens stop_reason), OpenAI (reasoning_content + length finish_reason), Gemini (TagStreamSanitizer <think> extraction + MAX_TOKENS), and local models (TagStreamSanitizer + length)
  #   6. When context window utilization exceeds 90%, the system MUST persist conversation state via session archive BEFORE any compaction or retry — ensuring context can be recovered even if the session is interrupted
  #   7. Normal completions (FinishReason::Stop, FinishReason::ToolCalls) MUST be completely unaffected — zero performance overhead on the happy path
  #   8. If thinking exhaustion happens repeatedly across turns (not just retries), the system MUST progressively downgrade reasoning effort for subsequent turns: XHigh→High→Medium→Low→Minimal→None
  #
  # EXAMPLES:
  #   1. Anthropic model with thinking_budget=8192 and max_tokens=16000 spends all 8192 tokens on thinking → response has 7800 thinking tokens, 12 output tokens → detected as thinking exhaustion → retry with thinking_budget=4096, max_tokens=16000 → model produces full response successfully
  #   2. OpenAI model with reasoning_effort=High hits length finish_reason → response has reasoning_content but empty choices[0].message.content → detected as thinking exhaustion → retry with reasoning_effort=Medium → model produces complete response
  #   3. Gemini model generates long <think>...</think> block consuming most of MAX_TOKENS → regular content after </think> is truncated → detected as thinking exhaustion → retry with thinking_level downgraded from 'high' to 'medium' → model produces useful response
  #   4. Model hits thinking exhaustion → retry 1 with reduced budget → hits thinking exhaustion again → retry 2 with further reduced budget → still exhausts → retry budget exhausted → thinking disabled entirely for this turn → model produces response without reasoning → warning shown to user
  #   5. Model completes normally with thinking (FinishReason::Stop, has both reasoning and content) → no thinking exhaustion detection fires → zero overhead → behavior identical to pre-PROV-041 baseline
  #   6. Context window at 92% utilization when thinking exhaustion occurs → system persists full conversation to session archive BEFORE retrying → thinking-reduced retry triggers compaction → even if session crashes during compaction, the pre-compaction state is recoverable via SessionSearch
  #   7. Thinking exhaustion happens 3 times across different turns (not retries) → session-level reasoning effort auto-downgrades from High to Medium → user notified 'Reasoning effort automatically reduced to Medium due to repeated thinking budget exhaustion' → subsequent turns use Medium reasoning
  #   8. Regular output truncation (model has useful content but hit token limit, no thinking content) → NOT classified as thinking exhaustion → handled by existing PROV-039/PROV-040 truncation recovery instead
  #
  # ========================================
  Background: User Story
    As a AI agent user
    I want to have thinking token exhaustion detected and recovered from automatically
    So that my agent doesn't lose context or produce empty responses when the model spends too many tokens reasoning

  @thinking-exhaustion
  @anthropic
  Scenario: Anthropic thinking exhaustion detected and recovered with reduced budget
    Given the agent is streaming a response from the Anthropic provider
    And the model has thinking_budget set to 8192 and max_tokens set to 16000
    When the model spends all tokens on thinking and produces near-empty output
    And the response terminates with FinishReason Length
    And the response has reasoning_tokens greater than 0 and output_tokens less than the exhaustion threshold
    Then the system detects this as thinking exhaustion rather than regular output truncation
    And the system retries with a reduced thinking_budget of 4096
    And the retry produces a complete response with both reasoning and output content

  @thinking-exhaustion
  @openai
  Scenario: OpenAI thinking exhaustion detected and recovered with lower reasoning effort
    Given the agent is streaming a response from the OpenAI provider
    And the model has reasoning_effort set to High
    When the model produces reasoning_content but empty output content
    And the response terminates with finish_reason length
    Then the system detects this as thinking exhaustion
    And the system retries with reasoning_effort downgraded to Medium
    And the retry produces a complete response

  @thinking-exhaustion
  @gemini
  Scenario: Gemini thinking exhaustion detected and recovered with lower thinking level
    Given the agent is streaming a response from the Gemini provider
    And the model has thinking_level set to high
    When the model generates a long think block consuming most of MAX_TOKENS
    And the regular content after the think block is truncated
    And the response terminates with FinishReason MaxTokens
    Then the system detects this as thinking exhaustion
    And the system retries with thinking_level downgraded from high to medium
    And the retry produces a useful response with complete output

  @thinking-exhaustion
  @thinking-preservation
  Scenario: Thinking content from exhausted attempt is preserved as context for retry
    Given the agent is streaming a response from any provider
    When thinking exhaustion is detected
    And the response contains reasoning content from the exhausted attempt
    Then the system preserves the captured thinking content
    And the retry request includes the preserved thinking content as context
    And the thinking content is not silently discarded

  @thinking-exhaustion
  @retry-budget
  Scenario: Retry budget prevents infinite thinking exhaustion retry loops
    Given the agent is streaming a response from any provider
    And the thinking exhaustion retry budget is set to 2
    When the model hits thinking exhaustion on the first attempt
    And the first retry with reduced budget also hits thinking exhaustion
    And the second retry with further reduced budget also hits thinking exhaustion
    Then the retry budget is exhausted
    And the system disables thinking entirely for this turn
    And the model produces a response without reasoning
    And a warning is shown to the user indicating thinking was disabled

  @thinking-exhaustion
  @happy-path
  Scenario: Normal completion with thinking is unaffected by exhaustion detection
    Given the agent is streaming a response from any provider
    And the model completes normally with FinishReason Stop
    And the response contains both reasoning content and output content
    When the stream completes
    Then no thinking exhaustion detection fires
    And no retry is triggered
    And the thinking exhaustion counter remains at zero
    And the behavior is identical to pre-PROV-041 baseline

  @thinking-exhaustion
  @context-preservation
  Scenario: Context preserved via session archive before retry near context limits
    Given the agent is streaming a response from any provider
    And the context window utilization exceeds 90 percent
    When thinking exhaustion is detected
    Then the system persists the full conversation state to session archive before retrying
    And the thinking-reduced retry proceeds after archival
    And the pre-compaction state is recoverable via SessionSearch even if the retry fails

  @thinking-exhaustion
  @progressive-degradation
  Scenario: Session-level reasoning effort auto-downgrades on repeated thinking exhaustion across turns
    Given the agent has a session-level reasoning effort set to High
    When thinking exhaustion occurs 3 times across different turns
    Then the session-level reasoning effort is automatically downgraded from High to Medium
    And the user is notified that reasoning effort was automatically reduced
    And subsequent turns use the downgraded Medium reasoning level

  @thinking-exhaustion
  @boundary
  Scenario: Regular output truncation is not classified as thinking exhaustion
    Given the agent is streaming a response from any provider
    And the model produces useful output content that exceeds the token limit
    And the response has no reasoning or thinking content
    When the response terminates with FinishReason Length
    Then the system does not classify this as thinking exhaustion
    And the existing PROV-039 truncation warning is displayed instead
    And any tool call truncation is handled by PROV-040 recovery instead
