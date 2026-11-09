# Subordinate HITL and approvals: how they work today and what is missing

Research date: 2026-11-07. Repo: /home/rquast/projects/fspec (Rust workspace `rust/`). Read-only; nothing was edited or run.

Product-owner report: "subordinate agents need to raise their HITL and approvals to the parent agent (or bot)".

**Short answer:** fspec has no routing for this at all. Every session, subordinate or not, raises its question or approval on its own session slot and then blocks a thread until somebody answers *that session id*. The parent session is never told. The parent's `await_idle` does not notice either: it waits for `Idle`, and a session waiting on a question is `Paused`. The parent cannot answer through AgentManager, because `message` refuses any recipient that is not idle. The only person who can answer is the user, and only by opening the subordinate's own tab or mux pane. An external MCP client polling that subordinate's id can also answer. With a bot parent using `await_idle` and no timeout, nothing moves until the user notices the yellow `!` chip and switches tabs. Closing the parent does not release a blocked subordinate.

---

## 1. Today's mechanics for a single session

### 1a. HITL question (`request_user_input`)

| Step | Where |
|---|---|
| The LLM calls the tool. `RequestUserInputTool::call` runs the pre-tool hook, then `execute_hitl(session_id, request)` | `rust/tools/src/request_user_input.rs:340-369` |
| Provider facades use the same path. The Codex `request_user_input` facade goes through `HitlToolFacadeWrapper`, which calls `execute_hitl`. A cancel becomes the tool error "request_user_input was cancelled before receiving a response" | `rust/tools/src/facade/wrapper.rs:169-295` (call at :275, cancel at :290) |
| `execute_hitl` validates the questions, then looks up the per-session handler in the global `HITL_HANDLERS` map. With no handler it returns the error "request_user_input is unavailable in the current session mode" | `request_user_input.rs:174-222` |
| A handler is registered for **every** session at the start of every turn. No session kind, parent or bot is checked | `rust/agent-loop/src/agent_loop.rs:212` → `rust/agent-loop/src/agent_loop/handlers.rs:23-34`; cleared after the turn at `agent_loop.rs:308` → `handlers.rs:64-66` |
| The handler calls `set_hitl_request(Some(req))`, then `set_status(Paused)`, then `wait_for_hitl_response()`. That last call is a std `mpsc` blocking `recv` inside `block_in_place`. Afterwards it clears the request and sets `Running` | `handlers.rs:25-31`; `rust/sessions/src/background_session/interactions.rs:28-35, 108-117, 140-151` |
| The request is stored in an `InteractionSlot`. Each `set` mints a request id, and a checked reply must match that id | `rust/sessions/src/interaction_slot.rs:12-40` |
| `set_status(Paused)` sends `(id, Paused)` on the manager-wide `status_changes_tx`. It also emits `StreamChunk::SessionStateChange{Paused}` through `handle_output`, which reaches the session's own `supervisor_broadcast` and the manager-wide `chunks_tx` | `rust/sessions/src/background_session/output.rs:8-41, 44-85` |
| **The question text is never pushed.** No `StreamChunk` variant carries a HITL request or pause details (`rust/rpc-types/src/stream_chunk.rs:14-165`). Clients must pull it | |
| Pull RPCs: `get_hitl_request(session)`, `get_pause_state(session)` | `rust/rpc/src/service_implementation/supervision_interaction.rs:138-150` → `rust/sessions/src/handle_impl/handle_impl_4.rs:24-37` |
| Answer RPC: `send_hitl_response(session, {cancelled, answers})`. This goes to `session.send_hitl_response`, which is the **unchecked** variant (`respond(None, …)`) | `supervision_interaction.rs:126-136` → `handle_impl_4.rs:5-23` → `interactions.rs:119-128` |
| Wire mapping between the internal and RPC shapes | `rust/sessions/src/hitl_mapping.rs:19-67`; wire types at `rust/rpc-types/src/session_state.rs:122-174` |
| TUI receives the chunk: `SessionStateChange{Paused}` becomes `Action::PauseChunkReceived(session)` | `rust/fspec-tui/src/app/dispatch_stream_chunks.rs:60-82` |
| TUI fetches: `handle_pause_chunk` reads `get_pause_state`, `get_hitl_request` and `get_exec_stdin_request` in parallel. HITL wins over a pause. The result goes into that session's store slot via `HitlPromptFetched` or `PauseStateFetched` | `rust/fspec-tui/src/app/dispatch_pause_hitl.rs:57-126`; `rust/fspec-tui/src/app/dispatch_hitl_prompt.rs:23-31` |
| TUI renders: the inline prompt is drawn in the input area of the **view whose session id it belongs to** (the focused tab or that mux pane) | `rust/fspec-tui/src/views/agent/input_area.rs:62-92, 134, 183` |
| TUI answers: submit and Esc-cancel both go through `handle_hitl_submitted`, which calls `backend.send_hitl_response` | `dispatch_pause_hitl.rs:181-192`; `dispatch_hitl_prompt.rs` (file header: cancel always sends `{cancelled:true}`) |
| On `Running` or `Idle` the TUI clears the slots (`PauseCleared`) | `dispatch_stream_chunks.rs:84-89`; `dispatch_pause_hitl.rs:131-135` |
| Other places a session shows as paused: the agent-bar chip draws `!` in yellow | `rust/fspec-tui/src/components/menu_bar/chips.rs:15, 99`. Also `"paused"` in AgentManager `list`/`get_status` (`rust/rpc-types/src/sessions.rs:86-92`) |

### 1b. Approval (tool pause)

| Step | Where |
|---|---|
| The tool calls `pause_for_user(session_id, PauseRequest{kind, tool_name, message, details})`. **With no handler it returns `Resumed`**, and every caller treats `Resumed` as "execute" (fail-open) | `rust/tools/src/tool_pause.rs:97-120` |
| Callers: <br>• Blocklist Bash prompt (Triple) at `rust/tools/src/blocklist/middleware.rs:196-215`<br>• Blocklist Read/sensitive-file prompt (Triple) at `middleware.rs:325-345`<br>• RLCD semantic gate on fspec workflow commands (Triple) at `rust/tools/src/rlcd/gate.rs:221-250`<br>• RLCD security layer (Triple) at `rust/tools/src/rlcd/security.rs:191-221`<br>• WebSearch "interact with the page, then press Enter" (Continue) at `rust/tools/src/web_search.rs:603-624`<br>• `/goal` escalation (Continue) at `rust/cli/src/interactive/goal.rs:66-76` | |
| Per-turn handler: `PauseState::for_session` names the active tool call (BUG-291). Then it runs `set_pause_state`, `set_status(Paused)`, and the blocking `wait_for_pause_response` | `handlers.rs:7-17`; `interactions.rs:8-46` |
| Wire: `PauseState{kind: Confirm|Triple, prompt, details, tool_call_id}` and `ApprovalChoice{Approve, ApproveSession, Deny}` | `rust/rpc-types/src/session_state.rs:82-120` |
| Answer RPCs: `pause_confirm`, `pause_triple` and `pause_resume` all call the unchecked `send_pause_response` | `supervision_interaction.rs:95-124` → `rust/sessions/src/handle_impl/handle_impl_3.rs:202-230` |
| TUI: the same Paused-chunk pull as HITL. The inline triple prompt cycles with arrows; Enter maps to `ApprovalChoice`; Esc denies | `rust/fspec-tui/src/app/dispatch_pause_route.rs:103-121`; `dispatch_pause_hitl.rs:155-163` |
| "Allow for session" is **process-global**. `SESSION_ALLOWANCES` is one static `HashSet` for the whole daemon. An `ApproveSession` in any session (parent, subordinate or unrelated) silently allows that rule in every session | `rust/tools/src/blocklist/middleware.rs:22, 387-403`; the gate uses it at `rlcd/gate.rs:114-116` |

### 1c. Other answer channels that exist

- **Client MCP** (an external agent driving the daemon). The catalog exposes `get_hitl_request`, `send_hitl_response`, `pause_triple`, `get_interaction_snapshot` and `reply_interaction_checked` (`rust/rpc/src/mcp_dispatch/catalog.rs:85-88, 126-127`). `reply_interaction_checked` is the only id-checked answer path: it uses the `InteractionSlot` request id (`handle_impl_3.rs:161-201`; `rust/rpc/src/service_implementation/interaction_sessions.rs:5-30`; `spec/features/client-mcp-stale-approval-replies.feature`). It is poll-only and per session id. It does not know about chains.
- **Bridge relay** (web dashboard / Telegram style remote). Every session registers a per-session `control_handler`, and its `pause_response` action answers that session's pause (`rust/agent-loop/src/agent_loop/bridge.rs:76-107`; routing at `rust/tools/src/bridge_relay.rs:467-489`). Subordinate chunks, including `SessionStateChange{Paused}`, are forwarded to the **root** parent's relay, tagged with `_relay_session_id` (`rust/agent-loop/src/agent_manager_handler.rs:214-276`; `rust/sessions/src/subordinate_forwarding.rs:8-102`). A remote client can therefore see that a subordinate paused and send `pause_response` for it. **There is no bridge action that answers a HITL question**, and the bridge never receives the question or approval text.
- Facades (Codex, Claude, Gemini, Z.AI in `rust/tools/src/facade/`) are tool-schema adapters only. They raise no approval or question events of their own; everything goes through `execute_hitl` or `pause_for_user`. A repo-wide grep found no `AskUserQuestion` or `PermissionRequest` events.

---

## 2. What happens when a subordinate raises one

Setup: AgentManager `spawn` creates the session, registers `add_supervisor(sub, spawner)` in `ChainOfCommand`, sets the role, and starts relay forwarding to the root parent (`agent_manager_handler.rs:118-225`). The TUI adds a tab for the new session (RPC-385, `spec/features/agentview-spawned-subordinate-session-registration.feature`). Per TUI-125 that tab currently steals focus.

Trace when subordinate S (parent P) calls `request_user_input`, or hits a blocklist or RLCD prompt:

1. S's own handler stores the request on **S's** slot, sets S to `Paused`, and blocks S's agent-loop thread (`handlers.rs:25-28`). Nothing consults the `ChainOfCommand`, so **it is not routed to P**.
2. The Paused chunk reaches every TUI client through `chunks_tx`, plus the root's bridge relay. The TUI fetches the request and stores it in **S's** slot. It is drawn **only when the S tab or pane is on screen** (`input_area.rs:72, 81`). Elsewhere the user sees only a yellow `!` on S's chip in the agent bar. There is no notice in P's or the bot's scrollback, no toast, and nothing says whose question it is.
3. **P's AgentManager view:**
   - `await_idle` subscribes to S's broadcast and returns only on `SessionStateChange{Idle}`, on the channel closing (`Destroyed`), on timeout, or on P being interrupted (`rust/agent-loop/src/agent_manager_wait.rs:17-38`; `agent_manager_handler.rs:857-1019`). `Paused` is ignored, and `AwaitOutcome` has no "needs input" or "blocked" variant (`rust/tools/src/agent_manager/types.rs:253-265`).
   - The `timeout` defaults to **wait indefinitely** (`rust/tools/src/agent_manager/mod.rs:184-186`; `spec/features/agent-manager-await-idle.feature:105-108`). P therefore hangs until the user answers inside S, or until the user presses Esc on P.
   - If P polls `get_status` or `list`, it sees `status: "paused"` (`agent_manager_handler.rs:354-395`) but not the question or approval text.
   - P cannot answer. `message` goes to `receive_agent_message`, which rejects every non-`Idle` recipient with `session_busy` (`rust/sessions/src/background_session/agent_message.rs:42-53`; `spec/features/agent-manager-busy-message-guard.feature:18` names "paused"). AgentManager has no answer or approve action.
4. **Not auto-denied.** Every session has a handler, so the `Resumed` fail-open never fires for subordinates. Nothing has a timeout either: the question or approval waits until the user answers it in S, an MCP client calls `send_hitl_response`/`pause_triple`/`reply_interaction_checked` for S, or a bridge client sends `pause_response` (approvals only).
5. **Interrupting P** (Esc on P) wakes P's `await_idle` (`Interrupted`) but does not cascade to S. S stays blocked.
6. **Closing or destroying S while it is blocked.** This happens when a bot is closed (the bot close closes its sub-agents, `rust/fspec-tui/src/app/dispatch_agent_close.rs:121-127`) or P calls AgentManager `close`. `destroy_session` removes S from the map and calls `session.interrupt()` (`rust/sessions/src/session_manager/manager_impl_6.rs:20-80`). `interrupt()` only sets flags, wakes the `Notify` and aborts bash (`rust/sessions/src/background_session/messaging.rs:122-136`). It **does not send on `hitl_response_tx` or `pause_response_tx`**. The blocked `recv` returns only when the sender drops, and the sender is a field of the session, while the session `Arc` is still held by the registered handler closure (`handlers.rs:23-34`, cleared only after the turn). From reading the code, the subordinate's turn thread stays blocked forever: a leaked blocked worker plus a session object that is gone from the map, and nothing can answer it any more (`get_session` fails). *Not confirmed by running anything; this is a code-reading finding.*
7. **Closing an agent (non-bot) parent P.** S keeps running (`close_facts.rs:212-216`, "Its N sub-agents keep running"). `destroy_session(P)` calls `remove_supervisor(P)` (`manager_impl_6.rs:28-29`; `rust/sessions/src/chain_of_command.rs` `remove_supervisor`), so S becomes a root with no parent. Its pending question stays on S's slot and is answerable only from S's tab.
8. **Daemon restart.** Supervisor links are in memory only (BUG-587), so any future routing has nowhere to go after a restart.

**Bot parent vs agent parent.** The daemon treats them the same. "Bot" is only a role overlay (`rust/sessions/src/bot.rs:9-32`; the TUI decides via `is_bot_role`). The differences in practice:
- The bot role tells the model to delegate everything, to use `await_idle`, and to "Use AgentManager message to steer or **answer** a sub-agent" (`bot.rs:13-27`). Answering a sub-agent's question by message is impossible while the sub-agent is paused (`session_busy`). Bots also lean on `await_idle` the most, so a **bot parent is the likeliest to hang indefinitely**.
- Closing a bot closes its sub-agents, which leads to the blocked-thread leak in item 6. Closing an agent orphans its sub-agents, which leads to item 7.

---

## 3. Existing partial support and related cards

- **No feature file or card covers escalating to a parent.** I grepped `spec/features/*.feature` for subordinate, parent or escalation near hitl, question, approval, pause or request_user_input, and searched `spec/work-units.json` titles and descriptions. Nothing matched.
- Mechanisms that look related but do not do this:
  - `subordinate_forwarding` / `bridge_relay` (SESS-015, BUG-217): mirrors chunks to the root's remote relay connection, for display only. Pause answering is per session id and there is no HITL answering.
  - `attribute_runaway_subordinate_forwarding` (`rust/tools/src/profile/diagnostics_tests/attribute_runaway_subordinate_forwarding.rs`): only an AMGR-017 profiling test of the forwarding loop. Unrelated.
  - `supervision_interaction.rs`: the per-session pause and HITL RPCs. No chain awareness.
  - Client-MCP `get_interaction_snapshot` / `reply_interaction_checked`: checked replies keyed by request id. These can be reused for "answer by id".
  - `InteractionSlot` request ids (`interaction_slot.rs`): the identity primitive to reuse.
  - RPC-061 supervisor links (`rust/fspec-tui/src/app/dispatch_supervisor_links.rs`, `spec/features/rpc061-supervisor-links.feature`): the TUI knows each session's supervisors, so it can attribute a question to the right parent.
- Related open cards:
  - **TUI-125** (backlog): a session spawned by AgentManager steals focus. Fixing it makes this problem *worse*, because the user will be even less likely to be on the subordinate's tab.
  - **BUG-587** (backlog): supervisor links are lost on daemon restart. Escalation needs these links.
  - **BUG-584** (backlog): deleting an open agent from /resume leaves it running.
  - **BUG-586** (backlog): the Exit dialog does not say what happens to running agents.
- Done features to stay consistent with:
  - `inline-hitl-prompt.feature` (RPC-411)
  - `inline-tool-approval-pause-prompt.feature` (RPC-406)
  - `pause-and-hitl-dialogs.feature` (RPC-053)
  - `paused-chunk-delivery-during-blocking-waits.feature` (RPC-409)
  - `pause-state-tool-call-identity.feature` (BUG-291)
  - `agent-manager-await-idle.feature` (AMGR-015)
  - `agent-manager-busy-message-guard.feature`
  - `client-mcp-stale-approval-replies.feature`
  - `tui-new-bot.feature` (TUI-114)

---

## 4. Failure scenarios (what the user experiences)

1. **Bot looks hung.** The user asks a bot for work. The bot spawns sub-agent A and calls `await_idle` with no timeout. A asks "Which auth library?" or hits a blocklist prompt on `git push`. The bot's view shows a running spinner indefinitely. Only a small yellow `!` on A's chip hints at the cause. The user has to work out that they must switch to A's tab to answer.
2. **Parent cannot help.** An agent parent with a timeout sees A as `timed_out` or `paused`. It tries `message` to A to supply the answer and gets `session_busy`. It can only spin on `get_status` or give up, and it never learns the question.
3. **The answer lands without context.** The user answers in A's tab, but the parent never learns what was asked or decided, so its later reasoning lacks that information.
4. **Approvals bypass the orchestrator.** A dangerous Bash call in A asks for approval in A's tab. The bot, which holds the overall plan, has no say and no visibility.
5. **"Allow for session" leaks.** Choosing Approve-for-session on A's prompt allows that rule for the bot and every other session in the daemon. Today this is global, not per session.
6. **Close while blocked.** The user closes the bot while A is waiting on a question. The tabs disappear, but A's turn thread stays blocked in the daemon (a leak), and the question can never be answered or cancelled.
7. **Orphan question.** The user closes an agent parent. A keeps running, its question stays pending, and A is now a parentless session that must be found and answered by hand.
8. **Nested chains.** Bot → A → B. B's question is invisible to both A and the bot. Both `await_idle` calls hang.
9. **Things a parent cannot do.** WebSearch "interact with the page, then press Enter" and `/goal` escalations need a human. Even with routing, a parent must not be allowed to "Continue" them on the user's behalf.

---

## 5. Proposed design

Principles: one routing path for every session, with no legacy branches. A root session (no supervisor) is the degenerate case: the chain is empty, so the request goes to the user exactly as today. Behaviour is deterministic and language-agnostic (the wire carries ids and structured payloads; nothing is inferred from text).

### 5.1 One interaction model

- Add a daemon-owned **Interaction** record, created when `execute_hitl` or `pause_for_user` is entered:
  - `interaction_id` (reuse the `InteractionSlot` request id)
  - `origin_session_id`
  - `kind`: `question` | `approval` | `human_action`. `human_action` covers `PauseKind::Continue`, i.e. WebSearch press-Enter and `/goal`.
  - `payload`: `HitlRequest` | `PauseState` (also carry the missing `tool_name`)
  - `chain`: the supervisor path at creation time, origin → … → root
  - `holder`: the session currently responsible for it, or `user`
  - `created_at`
- The origin session stays `Paused` and blocked exactly as today. Only *who is asked* changes.
- Every answer is **checked by `interaction_id`**. The unchecked `send_hitl_response` / `pause_*` RPCs (`handle_impl_4.rs:5`, `handle_impl_3.rs:202-230`) are replaced by the checked form. With no fallback, one path exists, as `reply_interaction_checked` already shows. The first valid answer wins; later answers get "stale or already answered".

### 5.2 Routing

- **Question:** the holder is the origin's immediate supervisor. If there is none, the user.
- **Approval:** see open decision D1. The recommended default is that the holder is the user, with the request also *announced* to every session in the chain. Guardrail prompts exist to get human consent, so an LLM should not approve its own sub-agent's risky command by default.
- **`human_action`:** always the user. It is announced to the chain.
- A parent holder gets the interaction as an **event**:
  - If the parent is inside `await_idle` on the origin (or on any ancestor of the origin in its own subtree), `await_idle` returns at once for that session with a new `AwaitOutcome::NeedsInput { interaction_id, origin_session_id, kind, payload }`. Other awaited sessions keep their normal outcomes, or `pending` if not yet done (see D5).
  - If the parent is `Idle`, the interaction is delivered as a turn-starting input, the same admission path as incoming messages (`receive_agent_message` → `retain_submission`), with origin `Subordinate interaction`.
  - If the parent is `Running` (not awaiting), the interaction is queued and attached to its next AgentManager result or turn boundary. `get_status` and `list` also show `pending_interactions: [...]` for that subordinate.
- The parent decides through new AgentManager actions:
  - `answer { interaction_id, answers }` for a question.
  - `approve { interaction_id, choice: approve|approve_session|deny }`, only if D1 allows parents to approve.
  - `escalate { interaction_id, note? }`: moves the holder to the parent's own supervisor, or to the user at the root. Escalation repeats up the chain and records each hop in `chain`. A parent that is itself a subordinate needs no special case.
- The parent may also call `request_user_input` itself. That is a *new* interaction of its own, not a forward of the subordinate's.

### 5.3 What the user sees

- When the holder is `user`, the prompt is drawn in the **root session's view**, the one the user is watching (bot or agent). It carries an attribution header such as `Question from Agent 1a2b3c4d (role: implement AUTH-002) via Bot …`, or `Approval requested by …: Bash git push`. The same prompt is also drawn in the origin's own tab. Answering in either place sends one checked reply.
- See D3 on whether to keep the root-only behaviour or to also show the prompt in intermediate views.
- The chip for the origin shows `?` (question) or `!` (approval / human action). The root's chip shows a "needs you" marker while any interaction in its tree has holder `user`.
- When a parent answers or escalates, a one-line notice is written to the origin's and the parent's scrollback, for example `Agent X answered: …` or `escalated to you`.
- The board and agent bar also list how many interactions are waiting on the user.

### 5.4 Timeouts and cancel

- With no timeout configured, interactions wait (today's behaviour, explicit). See D4 for an optional per-interaction deadline.
- When an `await_idle` timeout expires while an interaction is pending, the outcome is `NeedsInput`, never `TimedOut`, so the parent always learns why.
- The user can cancel from the prompt: Esc sends `cancelled:true` for a question and `Deny` for an approval, as today. An `AgentManager answer` with `cancelled:true` does the same.
- Interrupting the **origin** answers its pending interaction: a question gets `Cancelled` and an approval gets `Interrupted` (= deny). This fixes the blocked thread on interrupt.
- Interrupting the **parent** does not cancel the subordinate's interaction. The holder moves to the parent's supervisor or the user (see D6).

### 5.5 Parent closed or gone

- Destroying the **origin** must first answer its own pending interaction (Cancelled or Interrupted) so the blocked thread finishes. This fixes the leak in §2 item 6. All holders get an "interaction withdrawn" event.
- Destroying a **holder** parent moves each interaction it holds to the next live session up its `chain`. With none left, the holder becomes the user. The prompt then appears in the origin's view, now itself a root (agent close), or nowhere if the origin was closed too (bot close: the sub-agents are destroyed first and their interactions withdrawn).
- Daemon restart: blocked turns do not survive a restart, so this concern is about restoring links and showing the history honestly. This depends on BUG-587.

### 5.6 Approve-for-session scope

`ApproveSession` must apply to the origin session only, not to the whole process (`middleware.rs:22`). Otherwise a parent's, or the user's, "allow for session" on a sub-agent's prompt silently widens permissions everywhere. This can be done in the same card or as a separate bug (D7).

### 5.7 Open decisions for the product owner (either/or)

- **D1:** May a parent agent or bot **approve** a sub-agent's tool approval (blocklist, RLCD security, RLCD gate), or do approvals always go to the user, with the parent only notified?
- **D2:** For **questions**, does the parent get the first chance to answer and escalate only when it chooses to, or do questions go straight to the user with the parent only notified (pure pass-through)?
- **D3:** When an interaction reaches the user, is the prompt shown only in the **root (bot) view** plus the origin's tab, or in **every view up the chain**?
- **D4:** Interactions **wait indefinitely** until answered (as today), or have a **fixed deadline** after which a question is cancelled and an approval denied (and if so, what default)?
- **D5:** On `NeedsInput`, does `await_idle` **return at once** (other sessions reported as `pending`), or **keep waiting** for the rest and report `needs_input` alongside them?
- **D6:** When the parent is interrupted (Esc) while holding a sub-agent's question, does the question **escalate to the user** or get **cancelled**?
- **D7:** Is approve-for-session scoped to **the asking session**, or to **the whole chain under the root bot**? Fix it in **this card** or in **a separate bug**?
- **D8:** Does a bot parent get **different defaults** from an agent parent (for example, bots always pass approvals to the user) or **identical rules**?

---

## 6. Proposed cards

### Card A: story. "A sub-agent's questions and approvals go to the agent or bot that started it"

**Description:** Today a session spawned by AgentManager raises `request_user_input` and tool approvals on its own session only. Its parent is never told: `await_idle` hangs, since `Paused` is never `Idle`, and `message` returns `session_busy`. The user has to find the sub-agent's tab to answer. Route each subordinate interaction to its supervisor as an event the supervisor can answer or escalate, recursively up to the user at the root. Show the user whose question it is, in the view they are watching. One checked, id-based answer path for every session.

**Rules (draft):**
1. Every question or approval raised by a session becomes an interaction with an id, an origin session, a kind and a chain.
2. A session with no supervisor sends its interactions to the user, unchanged from today.
3. A subordinate's question is held by its immediate supervisor (subject to D2).
4. A subordinate's approval is held by the user and announced to the chain (subject to D1).
5. Human-action pauses (press Enter, goal escalation) are always held by the user.
6. `await_idle` returns `needs_input` with the interaction for an awaited session that has an interaction held by the caller.
7. AgentManager `answer` answers an interaction the caller holds; `escalate` moves it to the caller's supervisor, or to the user at the root.
8. A reply carries the interaction id; a reply to a changed or answered interaction is rejected.
9. An interaction held by the user is drawn in the root session's view with the origin's name and role, and in the origin's own view.
10. `list` and `get_status` show each session's pending interactions and holders.
11. Closing a holder moves its interactions up the chain; closing or interrupting the origin cancels its interactions (question: cancelled; approval: denied).

**Example scenarios:**
```gherkin
Scenario: A bot answers its sub-agent's question
  Given a bot that spawned sub-agent "A" and is awaiting it with no timeout
  When "A" asks "Which auth library?" with options "jose" and "jsonwebtoken"
  Then the bot's await_idle returns needs_input for "A" with that question and an interaction id
  When the bot calls AgentManager answer with that id selecting "jose"
  Then "A" receives the answer "jose" and continues running
  And "A"'s view shows "Bot answered: jose"

Scenario: A parent escalates a question to the user
  Given a bot that spawned sub-agent "A"
  And "A" asked a question that the bot holds
  When the bot calls AgentManager escalate with that interaction id
  Then the bot's view shows the question headed "Question from A (role: …)"
  When the user selects an option and submits
  Then "A" receives the user's answer
  And the bot is told the question was answered by the user

Scenario: Nested escalation reaches the user at the root
  Given bot "R" spawned agent "A" and "A" spawned agent "B"
  When "B" asks a question and "A" escalates it
  Then "R" holds the question
  When "R" escalates it
  Then the user sees it in "R"'s view headed "Question from B via A"

Scenario: A sub-agent's approval is shown to the user in the bot's view
  Given a bot that spawned sub-agent "A"
  When "A" runs "git push --force" and the blocklist asks for approval
  Then the bot's view shows "Approval requested by A: Bash git push --force"
  And the bot's await_idle returns needs_input for "A" with kind approval held by the user
  When the user chooses Deny in the bot's view
  Then "A"'s command is blocked with "User denied access"

Scenario: A stale answer is rejected
  Given "A" has a pending question with id "q1"
  And the user answered "q1"
  When the bot calls AgentManager answer for "q1"
  Then the answer is rejected with "Stale or already answered interaction"

Scenario: Closing a sub-agent with a pending question ends its blocked turn
  Given sub-agent "A" is waiting on a question
  When the bot closes "A"
  Then "A"'s question is cancelled and its turn finishes
  And no prompt for "A" remains in any view

Scenario: Closing the holder moves the question up the chain
  Given bot "R" spawned agent "A", "A" spawned "B", and "A" holds "B"'s question
  When "A" is closed and "B" keeps running
  Then "R" holds "B"'s question

Scenario: A root session's question is unchanged
  Given a session with no supervisor
  When it asks a question
  Then the user sees the inline prompt in that session's view exactly as today
```

### Card B: bug. "Destroying or interrupting a session blocked on a question or approval leaves its turn blocked forever"

**Description:** `destroy_session` and `interrupt()` never answer the pending interaction. `wait_for_hitl_response` / `wait_for_pause_response` block on a std `mpsc` whose sender is owned by the session, and the session is kept alive by the registered handler. The turn thread stays blocked after the session is removed. Files: `manager_impl_6.rs:20-80`, `messaging.rs:122-136`, `interactions.rs:37-44, 108-117`, `handlers.rs:23-34`. It can be folded into Card A rule 11 if preferred.

```gherkin
Scenario: Interrupting a session waiting on a question cancels the question
  Given a session waiting on a question
  When the session is interrupted
  Then the tool receives a cancelled response and the turn ends Interrupted

Scenario: Closing a session waiting on an approval denies it and ends the turn
  Given a session waiting on a blocklist approval
  When the session is closed
  Then the approval is answered Interrupted and the session's turn finishes
```

### Card C: bug. "Approve for session applies to every session in the daemon" (if D7 = separate)

**Description:** `SESSION_ALLOWANCES` (`rust/tools/src/blocklist/middleware.rs:22, 387-403`) and the RLCD gate allowances (`rlcd/gate.rs:114-116`) are a single process-global set. Approve-for-session in one session allows the rule in all sessions, including other users' bots and their sub-agents.

```gherkin
Scenario: Approve for session is scoped to the asking session
  Given sessions "A" and "B" in one daemon
  When "A" approves blocklist rule "git-push-force" for the session
  Then "A" is not asked again for "git-push-force"
  And "B" is asked for approval of "git-push-force"
```

Related cards to link: TUI-125 (focus steal; makes the gap worse), BUG-587 (supervisor links lost on restart; routing depends on them), BUG-584, BUG-586.
