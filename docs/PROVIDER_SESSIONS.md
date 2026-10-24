# Provider session extensions

`codelet_providers::session::ProviderSession` is the provider-neutral lifecycle
registry owned by each CLI session. Independent sessions create separate
registries; provider reconstruction within a session reuses its extension.

Implement `ProviderSessionExtension` in the provider adapter and register an
`Arc` with `ProviderSession::register`. The lifecycle forwards:

- `restore`: persisted message envelopes, including opaque extension fields.
- `set_writer`: a session-bound writer accepting provider metadata.
- `begin_turn`: the start of a user turn, not an individual model request.
- `reset`: conversation replacement, compaction, clearing, or provider switching.
- `set_observer` and `status`: optional safe user notifications and status data.

The provider's `ProviderAdapter::with_session` implementation attaches its
extension's handles to a newly constructed provider. The default implementation
does nothing, so ordinary providers need no lifecycle-specific dispatch logic.
Built-in extension selection lives in the providers crate, not the session or
persistence crates.

Persistence writers add the provider identity and an empty assistant payload.
Extension metadata must use provider-owned, noncanonical envelope keys. Core
persistence preserves those keys without interpreting their contents, including
explicit JSON `null` reset markers. Canonical fields such as `uuid`, `message`,
and `timestamp` remain owned by persistence. Blob rehydration preserves unknown
top-level fields as well.

Codex implements this interface in `providers/src/codex/session.rs`, where its
typed continuation checkpoint, cache routing, and safe metadata status remain
provider-specific. Credentials, internal routing headers, and opaque checkpoint
contents must never be included in status notifications.

Codex lineage comparison tolerates lossless history projections: adjacent
assistant text chunks, output annotations and log probabilities, and JSON tool
argument formatting. Replay still uses the original checkpoint items, not the
comparison projection. Registered raw-input and namespaced tool calls compare
against their executor projections without replacing changed arguments or call
identities. Their original wire calls and matching result types survive replay.
Streamed tool turns retain commentary in both normal
continuations and cancellation snapshots after tool execution. Changed or
missing assistant text, changed tool arguments, and missing tool outcomes still
fail validation; a lineage error never authorizes a checkpoint reset or replay
repair.

Parallel-call lineage also tolerates the streaming consumer's interleaved
call/result recording: a checkpoint batch of calls followed by outcomes compares
against the same ordered calls recorded individually with their outcomes. Only
outcomes of earlier calls in that checkpoint batch may be crossed to match its
next call. Call identities and arguments, result payloads and result order remain
validated. Replay preserves the original checkpoint order and appends unmatched
real outcomes once, including when the checkpoint still has pending calls.

## Compaction settings

Automatic compaction defaults to 80% of the configured context window for every
model. Explicit token or percentage thresholds take precedence, bounded by usable
context after output reservation. Request admission separately enforces that the
complete input plus reserved output fits the actual context capacity.

At each turn boundary, the existing session reloads saved provider, profile and
model runtime settings before dispatch, refreshing cached limits without
replacing conversation history or provider continuation state. Explicit session
token limits, compaction thresholds and continuation choices remain authoritative.
Invalid settings fail dispatch without partially applying the new configuration.

## Request admission

Context capacity and output reservations are token limits, not byte limits.
Admission tokenizes the complete serialized text with the shared token estimator,
including instructions, conversation, tool schemas and framing. A large UTF-8
request does not exceed its context window merely because its byte length is
larger than the token capacity. Estimates remain distinct from provider-reported
usage, and genuine input-plus-reserved-output overflow still blocks sending.

### Inline images

Selected request preparation and physical Codex sends use the same estimated
occupancy for Responses `input_image` parts containing inline PNG, JPEG, GIF or
WebP data. Each image reserves 10,000 estimated tokens, matching the conservative
image reservation in upstream Codex's `guardian-context/src/budget.rs`. This is
an admission estimate, not measured provider usage. The encoded data URL is not
counted as text; all remaining tokenized text and the output reservation still
participate in the budget. No image is decoded, fetched or rewritten to estimate
occupancy, and the complete original bytes are sent unchanged.

Remote image references and unsupported media still fail with typed unknown
media occupancy rather than receiving a zero-cost estimate. Tool schemas remain
fully counted as text and cannot hide media supplied in actual request content.

## Tool behavior stays shared

Provider facades translate names and arguments, then delegate execution to the
base tools. Codex's registrations in `providers/src/codex/agent.rs` use:

| Facade | Shared execution path |
| --- | --- |
| `shell_command` | `BashToolFacadeWrapper` → `BashTool` → unified exec |
| `shell`, `exec_command`, `write_stdin` | `ExecToolFacadeWrapper` → unified exec |
| `read_file`, `view_image` | `FileToolFacadeWrapper` → `ReadTool` |
| `grep_files` | `SearchToolFacadeWrapper` → `GrepTool` |
| `list_dir` | `LsToolFacadeWrapper` → `LsTool` |
| `request_user_input` | `HitlToolFacadeWrapper` → the session HITL handler |

Other registered tools, including `apply_patch`, retain their shared tool
implementations rather than duplicating execution in the provider adapter.

Quiet-process notifications belong to the process, not the originating model
turn. A model can finish its turn while a yielded command is still running.
The detector retains the session's observer so unregistering turn callbacks
does not discard the eventual clear on exit, removal, or resumed output. The
observer remains bound to the original agent session.

The TUI captures `apply_patch` changes before execution and feeds their typed
rows into the same diff codec and styling as Edit/Write. Multi-file headers
identify affected paths; collapsed cards retain the full diff for expansion.
Failed tool results discard the pending diff and preserve the error output.
