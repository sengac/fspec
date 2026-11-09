# MODEL-015: Claude effort that changes between turns

Date: 2026-11-06. Sources:
- The Claude API reference bundled with Claude Code's `claude-api` skill (cached 2026-10-06): `shared/model-migration.md`, the "Per-message effort" section under "Migrating to Claude Fable 5.1 from Claude Fable 5"; `shared/prompt-caching.md`, "Invalidation hierarchy"; `shared/platform-availability.md`.
- fspec code, `rust/<path>:<line>`.

## 1. The API

- Effort is GA as `output_config.effort` (`low|medium|high|xhigh|max`). Changing the **top-level** `effort` between requests invalidates the messages cache, as a `thinking` change does, on every model.
- **Per-message effort** uses the beta header `anthropic-beta: mid-conversation-output-config-2026-07-01`. A `{"role":"system","content":[],"output_config":{"effort":"low"}}` entry inside `messages` changes effort for later turns **without** invalidating the cache.
  - Placed directly after a `user` turn with new input, it sets the level for the reply to that turn. Placed anywhere else, it takes effect from the next user turn with new input; a user turn of only `tool_result` blocks doesn't count.
  - The level holds until a later effort message.
  - An effort-only message (empty content) can sit anywhere in `messages`.
  - Lowering is reliable; raising works best for large jumps.
- **Models:** Claude Fable 5.1, Mythos 5.1, Opus 5.5, Opus 5, Sonnet 5.5 and Haiku 5.5, with thinking on only.
  - Unsupported models (Fable 5 included) return 400: "output_config.effort requires a model that supports per-turn effort; this model does not".
  - With thinking off (`disabled`, or Sonnet 5.5's `between_tools`), a per-message effort that differs from the level in effect returns 400.
- **Platforms:** Claude API and Google Cloud. Bedrock, Foundry and Claude Platform on AWS are unconfirmed.
- **Append-only:** an edited history invalidates thinking blocks (preserved thinking) on Fable 5.1, Mythos 5.1, Opus 5.5, Sonnet 5.5 and Haiku 5.5. Effort messages must therefore stay in the history at their position on every later request, never be removed or moved.
- **Compaction:** a per-message effort level set inside summarized messages is not carried over. Until a later message sets a level, turns run at the request's top-level effort.

## 2. fspec today

- The turn level reaches Claude as `{"thinking":{"type":"adaptive","display":"summarized"},"output_config":{"effort":name}}` (`providers/src/thinking_wire/forms.rs:82-86`). It is merged at the top level of the body (`providers/src/claude.rs:590-606`; `patches/rig-core/src/providers/anthropic/request_body.rs:79-81`; shallow `merge_inplace`, `patches/rig-core/src/json_utils.rs:24-30`).
- So a level changed between turns changes the top-level effort, and the messages cache is lost on that request.
- **Beta headers** are fixed when the client is built (`providers/src/claude.rs:59-112`, `build_beta_headers`; rig `anthropic_betas`, `patches/rig-core/src/providers/anthropic/client.rs:64,131-135,184-196`). There is no per-request header.
- **Message roles:** rig's anthropic `Role` and the generic `rig::message::Message` have only User and Assistant (`patches/rig-core/src/providers/anthropic/completion.rs:195-198`, `patches/rig-core/src/completion/message.rs:36-45`). `content` is `OneOrMany`, so it can't be empty. Session restore accepts only user and assistant (`sessions/src/replay_content.rs:95-97`).
- **History:** `Session::messages: Vec<rig::message::Message>` (`cli/src/session/mod.rs:47`). It is sent each turn through `core/src/rig_agent.rs:198-235` and rebuilt by compaction (`cli/src/session/compaction.rs:160-208`). Stored as `StoredMessage {role, metadata}` (`core/src/persistence/messages.rs:59-77`).
- **Levels:** only the session's current level is kept (`sessions/src/background_session.rs:140`); per-turn levels aren't recorded.
