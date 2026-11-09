@done
@session
@rpc
@tui
@agent-view
@bug-fix
@rust
@BUG-648
Feature: Restoring a session or switching daemons or workspaces loses the live state: loading spinner, running tools and pending prompts aren't replayed
  """
  Design (2026-11-08; research in spec/attachments/BUG-648/bug648-research.md): the transcript journal is the session's event log and the TUI's live state is its fold. rpc-types: StreamChunk::is_journaled (false only for DebugStateChange, IsolationStateChange, FooterStateUpdate, which never go through BackgroundSession::handle_output). sessions: handle_output journals every is_journaled chunk (the InteractionRequestChanged and ProviderWaiting special cases go); set_status holds a status-order mutex across the swap, the status push and the journal write; set_exec_stdin_request journals inside its slot write lock. fspec-tui: transcript_push_action routes every journaled live chunk to Transcript::Live (a wake; it also runs the live-only effects: error dialog, fspec command runner, compaction badge); apply_transcript_page applies each page chunk's state through App::apply_journaled_state (record_chunk for content, token state and committed compaction, handle_stream_chunk_state_updates without the fspec runner, provider waiting, the interrupt queue). Action::ChunkReceived keeps meaning 'apply this chunk now' (state and effects). No new RPC or type; behavioural wire change: pages and get_buffered_output also carry InteractionRequestChanged and ProviderWaiting. Tests: rust/sessions/tests/bug648_journal_holds_every_state_event.rs (journal order), rust/fspec-tui/tests/bug648_attach_rebuilds_live_state.rs (a real SessionManager, embedded and WebSocket; a TUI attached from the start versus one that attaches by each path)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every attach rebuilds the session's live state to exactly what a TUI that never detached shows: the Running/Compacting/Paused status and spinner, the tool call in progress with its streamed output, a pending question, approval or human action with its AMGR-020 route, a pending exec-stdin prompt, compaction progress, token and context-fill counters, and the work-unit focus. The attach paths are /resume, the startup restore, a daemon switch, a workspace switch, a mux pane and a WebSocket reconnect, on the embedded and the WebSocket transport (user report, 2026-11-08)
  #   2. A session's transcript journal is its event log: every per-session event the daemon broadcasts through the session's output is journaled in broadcast order, including InteractionRequestChanged and ProviderWaiting. One shared classification, StreamChunk::is_journaled in rpc-types, names the journaled kinds, so a kind is either always journaled or never
  #   3. The TUI applies a journaled chunk only from the transcript pages, in journal order, through one state handler; a live journaled chunk only wakes the page fetch, so nothing is applied twice and nothing between two pages is skipped. Kinds the daemon never journals (debug, isolation, footer) are applied when they arrive
  #   4. A pending question, approval or human action is read with its route (get_hitl_request, get_pause_state, get_exec_stdin_request) when the session's replayed status is Paused; the read answers with the daemon's current request, so replaying an earlier, answered pause never shows a request that is no longer pending
  #   5. The journal order is the daemon's own state order: a status change is pushed and journaled under one status-order lock, and an exec-stdin request change is journaled while its slot is still locked
  #   6. State events missed while the WebSocket was down, or lost to a broadcast lag, come back from the next pages: the transcript cursor continues from where it was
  #   7. One-shot effects happen only when a chunk arrives live, never when a page replays it: an fspec command request runs once, the error dialog pops once, and the compaction reduction badge shows for its 10 seconds (RPC-417). A replayed page applies the state and the transcript only, including the compaction notice line
  #
  # EXAMPLES:
  #   1. Session S is Running with a Bash tool call that has streamed 'line 1', 1200 input and 300 output tokens and a context fill of 40%; a second TUI opens S with /resume; it shows the Thinking spinner, the Bash tool card with 'line 1', and the same token and context-fill badge as the TUI that was attached from the start
  #   2. S asks 'Which auth library?' and is Paused; the TUI starts again and restores S as the daemon's active session; the inline question shows with S's request id, as in the TUI that never detached
  #   3. S is Compacting with progress 'summarizing 2 of 5'; the user switches from daemon A to daemon B, which hosts S; S's tab shows the Compacting spinner and the progress
  #   4. S, in workspace W2, waits on an approval; the TUI shows workspace W1 and switches to W2; S's tab shows the approval prompt with its request id
  #   5. S has a pending exec-stdin prompt for 'npm login'; mux mode shows S in a pane; the pane shows the stdin prompt and S's status, as in the TUI that never detached
  #   6. The WebSocket is down while S goes Running, runs a tool and goes Idle, and its tokens change; after the reconnect S shows Idle, the finished tool card once, and the new token counts
  #   7. S's journal holds an approval that was answered, then Running, then Idle; an attaching TUI shows S Idle with no approval prompt
  #   8. S's journal holds an fspec command request that was already answered; an attaching TUI does not run it again
  #   9. S broadcasts an InteractionRequestChanged and a ProviderWaiting chunk; both are in S's transcript pages at the positions they were broadcast, between the chunks before and after them
  #
  # ASSUMPTIONS:
  #   1. A CompactionComplete replayed on attach sets no reduction badge: the badge is a 10-second flash armed when the event arrives (RPC-417), and pages carry no event time, so a TUI that attaches within those 10 seconds shows no badge while one that never detached still does. Alternative: show the badge for 10 seconds from the attach. Derived (one-shot effects are not replayed), reported to the supervisor for the product owner
  #
  # ========================================
  Background: User Story
    As a user of the TUI
    I want to see a running session's exact live state whenever I attach to it, by any path and over either transport
    So that I can follow and answer it as if I had never left

  Scenario: Opening a running session with /resume shows its spinner, its running tool's output and its token counts
    Given session S is Running with a Bash tool call that has streamed 'line 1'
    And S has reported 1200 input and 300 output tokens and a context fill of 40%
    And a TUI has been attached to S since before it started
    When a second TUI opens S with /resume over either transport
    Then the second TUI shows the Thinking spinner and the Bash tool card with 'line 1'
    And its view of S equals the first TUI's, token and context-fill counters included

  Scenario: Restoring the active session at startup shows its pending question
    Given session S asks 'Which auth library?' and is Paused
    And S is the daemon's active session
    When a TUI starts and restores S over either transport
    Then it shows the inline question 'Which auth library?' with S's request id
    And its view of S equals that of a TUI attached to S since before it asked

  Scenario: Switching to the daemon that hosts a compacting session shows its compaction progress
    Given session S on daemon B is Compacting with progress 'summarizing 2 of 5'
    And a TUI is connected to daemon A
    When the TUI switches to daemon B over either transport
    Then S's tab shows the Compacting spinner and the progress 'summarizing 2 of 5'
    And its view of S equals that of a TUI connected to daemon B since before S started compacting

  Scenario: Switching to the workspace of a session waiting on an approval shows the approval
    Given session S in workspace W2 waits on an approval
    And a TUI that connected after S asked shows workspace W1
    When the TUI switches to workspace W2
    Then S's tab shows the approval prompt with its request id
    And its view of S equals that of a TUI attached to S since before it asked

  Scenario: A mux pane showing a session with a pending stdin prompt shows the prompt
    Given session S has a pending exec-stdin prompt for 'npm login'
    When mux mode shows S in a pane of a TUI that connected after the prompt, over either transport
    Then the pane shows the stdin prompt for 'npm login' and S's status
    And its view of S equals that of a TUI attached to S since before the prompt

  Scenario: Events missed while the WebSocket was down come back once after the reconnect
    Given a TUI shows session S over the WebSocket
    And the WebSocket is down while S goes Running, runs a tool, goes Idle and reports new token counts
    When the WebSocket reconnects
    Then the TUI shows S Idle with the finished tool card once and the new token counts
    And its view of S equals that of a TUI that stayed connected

  Scenario: A replayed approval that was answered leaves no prompt
    Given session S's journal holds an approval that was answered, then Running, then Idle
    When a TUI attaches to S
    Then it shows S Idle with no approval prompt

  Scenario: A replayed fspec command request is not run again
    Given session S's journal holds an fspec command request that was already answered
    When a TUI attaches to S
    Then it sends no fspec result for that request

  Scenario: Request and provider-waiting events are journaled where they were broadcast
    Given a session that broadcasts a text chunk, an InteractionRequestChanged chunk, a ProviderWaiting chunk, a status change and another text chunk
    When its transcript is read page by page
    Then the pages hold every chunk it broadcast, in the order it broadcast them
    And the last status change in the pages is the session's status

  Scenario: A request answered in one chain member's transcript stays closed when another member's copy arrives later
    Given sub-agent #2 of bot #1 asks a question held by the user
    And bot #1's transcript delivers the request's Pending event twice and then its Answered event
    When sub-agent #2's transcript delivers its Pending copy of the request afterwards
    Then bot #1's view shows no relayed question and no waiting marker for it
