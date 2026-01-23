@watcher-management
@codelet
@WATCH-020
Feature: Autonomous Watcher Interjection with Response Parsing

  """
  Adds auto_inject field to SessionRole/WatcherConfig, exposes via NAPI for WatcherCreateView toggle
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The evaluation prompt must include explicit instructions for the AI to respond with either [INTERJECT]...[/INTERJECT] or [CONTINUE]...[/CONTINUE] blocks
  #   2. The [INTERJECT] block must contain two fields: 'urgent: true/false' (whether to interrupt parent mid-stream) and 'content:' (the message to inject)
  #   3. The watcher loop must capture the full AI response text (not just stream it) to enable parsing after the response completes
  #   4. parse_interjection() must extract the interjection from the response, returning Some(Interjection) for valid [INTERJECT] blocks and None for [CONTINUE] or malformed responses
  #   5. When a valid [INTERJECT] block is parsed, the watcher must automatically call watcher_inject() with the extracted content - no manual user action required
  #   6. When urgent: true, the injection must interrupt the parent session mid-stream; when urgent: false, the injection waits for the parent's current turn to complete
  #   7. The watcher's evaluation response must still be shown in the watcher UI (for transparency) in addition to being parsed for injection
  #   8. Malformed [INTERJECT] blocks (missing closing tag, missing fields, empty content) must be treated as [CONTINUE] - fail safe, no injection
  #   9. The evaluation prompt must include authority-aware context: Supervisor watchers are told their interjections should be followed; Peer watchers are told their interjections are suggestions
  #   10. Yes, add a configurable 'auto-inject' toggle per watcher. Default is enabled (autonomous injection). When disabled, watcher shows [INTERJECT] decision in UI but requires manual user action to inject.
  #   11. Strict format with good error feedback. Require exact block markers [INTERJECT]/[/INTERJECT] (case-sensitive), exact field names 'urgent:' and 'content:' (lowercase), exact boolean values 'true' or 'false' only. One small flexibility: allow optional whitespace after colons. Log specific parsing failures for debugging. Invest in clear prompt engineering rather than flexible parsing.
  #   12. Use a WatcherOutput wrapper struct that captures turn-specific text during streaming. WatcherOutput wraps BackgroundOutput, accumulates text chunks in a turn_text buffer, and delegates all other events to the inner handler. After run_with_provider completes, get_turn_text() retrieves the full response for parsing. This keeps watcher-specific logic separate from shared infrastructure and follows existing codebase patterns.
  #   13. Parse [INTERJECT]/[CONTINUE] blocks ONLY for observation evaluations (is_user_prompt=false). Direct user prompts to the watcher are normal conversation - no parsing, no risk of accidental injection from explanatory text. This matches the architecture intent where the structured format is specifically for evaluation decisions. If user-initiated injection is needed later, add an explicit /inject command as a separate feature.
  #
  # EXAMPLES:
  #   1. Security Reviewer watcher (Supervisor) observes parent writing SQL with string interpolation → AI evaluates at ToolResult breakpoint → responds with [INTERJECT] urgent: true content: 'SQL injection vulnerability' → watcher_inject called automatically → parent receives purple watcher message
  #   2. Architecture Advisor watcher (Peer) observes parent implementing a service → AI evaluates at Done breakpoint → responds with [CONTINUE] noting 'implementation follows good patterns' → no injection occurs → watcher UI shows the evaluation for user reference
  #   3. Test Enforcer watcher observes parent writing implementation code → AI evaluates → responds with [INTERJECT] urgent: false content: 'Tests should be written first' → injection is queued until parent's current turn completes → parent sees suggestion after finishing current response
  #   4. Watcher AI responds with malformed block '[INTERJECT] this is not properly formatted' (missing closing tag and fields) → parser returns None → no injection occurs → watcher UI shows the response but system logs warning about malformed interjection block
  #   5. Watcher with role 'Security Reviewer' and brief 'Watch for SQL injection, XSS' evaluates observations → evaluation prompt includes: role context, authority level, watching brief, formatted observations, and explicit instructions for [INTERJECT]/[CONTINUE] response format with field requirements
  #   6. Watcher AI responds with multiline content in [INTERJECT] block: 'urgent: true\ncontent: ⚠️ Security Issue\n\nThe code uses eval() which is dangerous.\nConsider using JSON.parse() instead.' → parser extracts full multiline content → injection preserves formatting
  #   7. Watcher created with auto-inject disabled → watcher evaluates observations → responds with [INTERJECT] block → parser extracts interjection → watcher UI shows 'Would inject: ...' with manual inject button → no automatic injection occurs → user can click to inject or ignore
  #   8. User talks directly to watcher: 'What issues have you seen?' → watcher responds with summary (no [INTERJECT] block) → response shown normally in watcher UI → no parsing occurs because is_user_prompt=true
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should there be a configurable 'auto-inject' toggle per watcher that allows users to disable autonomous injection and require manual review before injecting?
  #   A: Yes, add a configurable 'auto-inject' toggle per watcher. Default is enabled (autonomous injection). When disabled, watcher shows [INTERJECT] decision in UI but requires manual user action to inject.
  #
  #   Q: When parsing [INTERJECT], should we require exact field format ('urgent: true' and 'content:') or be more flexible with variations like 'urgent=true' or 'Urgent: True'?
  #   A: Strict format with good error feedback. Require exact block markers [INTERJECT]/[/INTERJECT] (case-sensitive), exact field names 'urgent:' and 'content:' (lowercase), exact boolean values 'true' or 'false' only. One small flexibility: allow optional whitespace after colons. Log specific parsing failures for debugging. Invest in clear prompt engineering rather than flexible parsing.
  #
  #   Q: Should there be rate limiting on autonomous injections to prevent a chatty watcher from flooding the parent session? (e.g., max 1 injection per 30 seconds)
  #   A: No rate limiting. Trust the AI's judgment and the user's watcher brief configuration. If a watcher is too chatty, the user can adjust the brief or disable auto-inject.
  #
  #   Q: If the parent session ends/exits while the watcher is processing an evaluation that would trigger injection, should we silently drop the injection or show an error in the watcher UI?
  #   A: Silently drop the injection. The parent is gone, nothing to inject into, just move on. No error shown in watcher UI.
  #
  #   Q: For capturing the AI response to parse: should we buffer during streaming (cleaner, captures exact turn) or reconstruct from output_buffer after completion (simpler, but may include more than current turn)?
  #   A: Use a WatcherOutput wrapper struct that captures turn-specific text during streaming. WatcherOutput wraps BackgroundOutput, accumulates text chunks in a turn_text buffer, and delegates all other events to the inner handler. After run_with_provider completes, get_turn_text() retrieves the full response for parsing. This keeps watcher-specific logic separate from shared infrastructure and follows existing codebase patterns.
  #
  #   Q: Should [INTERJECT]/[CONTINUE] parsing ONLY apply to observation evaluations (is_user_prompt=false), or should direct user prompts to the watcher also be parsed for injection blocks?
  #   A: Parse [INTERJECT]/[CONTINUE] blocks ONLY for observation evaluations (is_user_prompt=false). Direct user prompts to the watcher are normal conversation - no parsing, no risk of accidental injection from explanatory text. This matches the architecture intent where the structured format is specifically for evaluation decisions. If user-initiated injection is needed later, add an explicit /inject command as a separate feature.
  #
  # ========================================

  Background: User Story
    As a watcher session AI agent
    I want to autonomously evaluate parent session observations and inject warnings or advice when my watching brief is triggered
    So that the parent session receives real-time feedback without requiring manual intervention from the watcher's user

  Scenario: Watcher autonomously injects urgent security warning
    Given a Security Reviewer watcher with Supervisor authority and auto-inject enabled is observing a parent session
    When the parent session writes SQL code with string interpolation and emits a ToolResult breakpoint
    And the watcher AI responds with '[INTERJECT] urgent: true content: SQL injection vulnerability detected [/INTERJECT]'
    Then watcher_inject should be called automatically with the content
    And the parent session should be interrupted mid-stream
    And the parent session should receive a purple watcher message


  Scenario: Watcher continues without injection when no issues found
    Given an Architecture Advisor watcher with Peer authority is observing a parent session
    When the parent session implements a service following good patterns and emits a Done breakpoint
    And the watcher AI responds with '[CONTINUE] Implementation follows good patterns [/CONTINUE]'
    Then no injection should occur
    And the watcher UI should show the evaluation response for user reference


  Scenario: Non-urgent injection waits for parent turn completion
    Given a Test Enforcer watcher with auto-inject enabled is observing a parent session
    When the parent session writes implementation code without tests
    And the watcher AI responds with '[INTERJECT] urgent: false content: Tests should be written first [/INTERJECT]'
    Then the injection should wait until the parent's current turn completes
    And the parent should see the suggestion after finishing the current response


  Scenario: Malformed interjection block is treated as continue
    Given a watcher with auto-inject enabled is observing a parent session
    When the watcher AI responds with '[INTERJECT] this is not properly formatted'
    Then parse_interjection should return None
    And no injection should occur
    And a warning should be logged about the malformed interjection block


  Scenario: Evaluation prompt includes proper format instructions
    Given a watcher with role 'Security Reviewer' and brief 'Watch for SQL injection, XSS'
    When format_evaluation_prompt is called with accumulated observations
    Then the prompt should include role context and authority level
    And the prompt should include the watching brief
    And the prompt should include explicit [INTERJECT]/[CONTINUE] response format instructions


  Scenario: Multiline content is preserved in injection
    Given a watcher with auto-inject enabled is observing a parent session
    When the watcher AI responds with a multiline [INTERJECT] block containing "urgent: true" and multiline content
    Then parse_interjection should extract the full multiline content
    And the injection should preserve the formatting including newlines


  Scenario: Auto-inject disabled shows pending injection without injecting
    Given a watcher with auto-inject disabled is observing a parent session
    When the watcher AI responds with a valid [INTERJECT] block
    Then no automatic injection should occur
    And the watcher UI should show the pending injection for manual review


  Scenario: Direct user prompts are not parsed for injection
    Given a watcher with auto-inject enabled
    When the user sends a direct prompt 'What issues have you seen?'
    And the watcher responds with a summary
    Then the response should be shown normally in the watcher UI
    And no [INTERJECT] parsing should occur because is_user_prompt is true

