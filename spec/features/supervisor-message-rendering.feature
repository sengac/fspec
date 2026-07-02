@done
@bug-fix
@ts-parity
@parser
@rust
@agent-view
@tui
@RPC-387
Feature: Supervisor message rendering in the subordinate view
  """
  Backend wire format stays space-separated; only the TUI parser changes (parity with TS chunkProcessor and NAPI assertions)
  Fix lives in parse_supervisor_envelope (session_context.rs): split on header closing bracket, trim leading space/newline from body
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The incoming-message envelope parser must extract the message body when the backend separates the header and body with a SPACE (the real format_incoming_message output)
  #   2. The parser must still extract the body when the header and body are separated by a NEWLINE (replay/legacy form), preserving existing behaviour
  #   3. When the raw text has no recognizable [ header, the parser must fall back to role 'supervisor' and treat the whole string as the body
  #   4. The rendered chunk for an incoming supervisor message must be '[W] <role>> <body>' with the body present and colored magenta
  #
  # EXAMPLES:
  #   1. Parsing '[SUPERVISOR: reviewer | Session: s-2] please check this' (space) yields role 'reviewer' and body 'please check this'
  #   2. Parsing '[SUPERVISOR: reviewer | Session: s-2]\nplease check this' (newline) yields role 'reviewer' and body 'please check this'
  #   3. Parsing 'raw body without header' (no bracket) yields role 'supervisor' and body 'raw body without header'
  #   4. A StreamChunk::IncomingMessage carrying the space-separated backend envelope renders scrollback text '[W] reviewer> please check this' in magenta
  #
  # ========================================
  Background: User Story
    As a subordinate agent user
    I want to see the full text of a message my supervisor sends me in my session view
    So that I can act on the supervisor's instructions instead of seeing an empty prompt

  Scenario: Parse a space-separated supervisor envelope from the real backend
    Given a supervisor envelope "[SUPERVISOR: reviewer | Session: s-2] please check this" separated by a space
    When the envelope is parsed
    Then the extracted role is "reviewer"
    And the extracted body is "please check this"

  Scenario: Parse a newline-separated supervisor envelope from the replay path
    Given a supervisor envelope "[SUPERVISOR: reviewer | Session: s-2]\nplease check this" separated by a newline
    When the envelope is parsed
    Then the extracted role is "reviewer"
    And the extracted body is "please check this"

  Scenario: Fall back to the default role when no header is present
    Given a raw message "raw body without header" with no envelope header
    When the envelope is parsed
    Then the extracted role is "supervisor"
    And the extracted body is "raw body without header"

  Scenario: Render a space-separated incoming supervisor message in the scrollback
    Given a subordinate session with an empty scrollback
    When a StreamChunk::IncomingMessage carrying "[SUPERVISOR: reviewer | Session: s-2] please check this" is recorded
    Then the scrollback shows a single rendered chunk with text "[W] reviewer> please check this"
    And the rendered chunk is colored magenta
