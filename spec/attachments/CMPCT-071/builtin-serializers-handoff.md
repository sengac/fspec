# Built-in physical serializers — active audit

Owner: 95c5d624. Exclusive scope: rig `src/providers/**`, new
`providers/tests/cmpct071_builtin_serializers*`, dedicated parity feature.
Existing staged work is preserved; no commits.

API remains `CompletionModel::prepare_request(request) -> Result<Vec<u8>,
CompletionError>` and default generation forwarding. Built-in overrides are
being added using the exact selected stream serializer, not a common envelope.
OpenAI chat respects `with_stream(false)`; Responses/Gemini/Anthropic use their
streaming wire shape.

## Immediate cross-owner request for a09f4feb

CopilotHttpClient currently mutates/re-serializes JSON in send/send_streaming,
adding Claude cache markers. Rig serialization alone is therefore NOT exact.
Please add synchronous `HttpClientExt::prepare_request_body(&self, body: Vec<u8>)
-> Result<Vec<u8>>` default identity (rig http_client module is your scope), and
override it in CopilotHttpClient by sharing `classify_and_cache_body`. Preserve
existing send behavior and ensure repeated preparation is idempotent. Built-in
OpenAI serializers will call `self.client.http_client().prepare_request_body`
as their final step. Please publish when available. If name/signature differs,
record it here. This hook must include byte reserialization for Responses too.

Anthropic OAuth metadata currently generates random fallback IDs per serialize;
I will capture it on model creation and reuse across prepare and send.

Dedicated feature: `builtin-provider-physical-serializer-parity.feature`.
Red/green and coverage evidence will be appended after execution.

Dedicated five-scenario integration target written before production changes:
`cargo test --profile ci-test -j 2 -p codelet-providers --test
cmpct071_builtin_serializers`, log `/tmp/cmpct071-builtin-red.log`. Currently
waiting behind agent-loop's first ci-test dependency compilation. No production
serializer changes yet: strict behavioral-red prerequisite respected.

First own target exposed a test-builder generic inference error only, fixed
before implementation. Behavioral run now queued as
`/tmp/cmpct071-builtin-behavior-red.log` behind TUI's first lance/datafusion
ci-test build (07:51). Please prioritize this small serializer red target if
compiler scheduling is adjusted: all builtin positive runtime/gate rows depend
on these overrides, and no production implementation can precede behavioral red.
Feature syntax valid; global tag validation has unrelated feature violations.
Later scoped validation exposed a missing feature-group tag in this feature;
the registered `@context-management` tag was added during recovery.

Behavioral RED now verified: `/tmp/cmpct071-builtin-behavior-red.log`, all
five scenario tests fail with `selected provider has no physical request
serializer` (0 passed, 5 failed). Shared exact serializer implementation is
now in progress. OpenAI Chat/Responses helpers call requested
`HttpClientExt::prepare_request_body`; awaiting provider-owner hook.

All four built-in overrides are now present in the shared worktree, including
Anthropic API key/OAuth, Gemini, OpenAI Chat stream/nonstream and Responses.
Their physical stream sends use the same helpers as preparation. Anthropic
captures OAuth user metadata at model construction for clone-stable bytes.
CRITICAL compile dependency: requested `HttpClientExt::prepare_request_body`
is not yet present at this checkpoint. Provider owner a09f4feb should add hook and
Copilot override now; busy message delivery is rejected, so this is the mailbox.

First green attempt `/tmp/cmpct071-builtin-green1.log` is compile-blocked ONLY
by four missing `prepare_request_body` calls, exactly the cross-owner dependency.
All new serializer helper files are formatted and <100 lines. Five scenario
implementation mappings are linked; these are traceability, not measured coverage.

## Serializer-owned touched files

- New helpers: `rust/patches/rig-core/src/providers/anthropic/request_body.rs`,
  `gemini/request_body.rs`, `openai/completion/request_body.rs`, and
  `openai/responses_api/request_body.rs` (paths after the first share the same
  `rust/patches/rig-core/src/providers/` prefix).
- Wiring: Anthropic `mod.rs`, `completion.rs`, `streaming.rs`; Gemini `mod.rs`,
  `completion.rs`, `streaming.rs`; OpenAI Chat `mod.rs`, `streaming.rs`,
  `nonstreaming.rs`; Responses `mod.rs`, `streaming.rs`.
- New integration test: `rust/providers/tests/cmpct071_builtin_serializers.rs`.
- New feature/sidecar: `spec/features/builtin-provider-physical-serializer-parity.feature`
  and `.feature.coverage`; this handoff. Existing CMPCT-071 feature untouched.

Parent now owns the missing HTTP hook/Copilot middleware implementation.
The parity test exercises real RefreshingClaudeClient in API-key and OAuth
modes, CopilotHttpClient for Claude/GPT/Gemini chat plus GPT Responses, and
reqwest-backed built-in OpenAI/Gemini clients. Empty scripted SSE responses
capture the physical send without interpreting response success as admission
success; this target proves bytes, not full resumed-runtime gate behavior.

Recovery checkpoint: no Cargo jobs running when inspected. HTTP hook still absent;
serializer green/clippy cannot execute until parent adds it. New helper/test
`rustfmt --check --edition 2024 --config skip_children=true` passes, as does
scoped `git diff --check`. Full touched-existing-file formatting check exposes
pre-existing formatting and trailing-whitespace-in-macro errors; unrelated
formatting was preserved. Model constructor wrapping was corrected, shifting
Anthropic completion implementation mapping lines by eight (refreshed).

Scoped `fspec format`, `fspec validate`, and `fspec validate-tags` now pass for
the dedicated parity feature. A five-minute hook-readiness runner was scheduled
(`5011daa2`) to execute only the dedicated target once the hook existed. Its
planned `/tmp/cmpct071-builtin-green2.log` is absent and is not execution or
GREEN evidence. Current retained replacement evidence is
`/tmp/cmpct-rig-supervised-adapters.log`: exit 0, including all six main parity
tests and the Unicode property test, with the exact command in
`../CMPCT-065/rig-supervised-completion-2026-10-18.md`. No synthetic fallback was added.

## Completion continuation — current owner

Hook now exists at `HttpClientExt::prepare_request_body`; Copilot's override
shares `classify_and_cache_body`. `/tmp/cmpct071-builtin-resumed-green.log`
executed all five original parity tests successfully (5 passed, 0 failed).
These include real middleware and 10 actual captured physical POST bodies.
No synthetic envelope is used. The earlier five-test behavioral RED remains
`/tmp/cmpct071-builtin-behavior-red.log`.

Tests are now being strengthened through the actual `SelectedRequestPreparer`
and ordered assistant-tool/result suffix for all adapters. A sixth feature
scenario is written before a new behavioral RED: Gemini's common serializer
currently drops mandatory documents and explicit max_tokens/temperature unless
additional_params contains generationConfig. RED target is
`/tmp/cmpct071-builtin-context-red.log`. No fix precedes its execution.

Gate owner: `CompletionModel::prepare_request` is available on Anthropic,
Gemini, OpenAI chat and Responses; Copilot uses these same actual OpenAI models
with its actual `CopilotHttpClient`. Generation-aware calls forward to the
same serializer. `SelectedRequestPreparer::new(model, template, limits)` can
use each of these directly. OAuth metadata is clone-stable per model.
Full cold-resume runtime/generation/promotion behavior remains gate-owned;
the serializer tests prove physical bytes and selected-admission wiring only.

## Final serializer completion — 2026-10-18

Current owner: `de7a9d26-0be5-4a70-a578-e550f9c8a2e1`.
Builtin implementation is complete. No remaining serializer/hook dependency
blocks gate owner `eb10bc74-4796-422b-a171-ada06fd03b7b`. The non-provider Rig
hook and Copilot override remain provider-owner changes, not this owner's edits.
No board, staging or commit action was performed.

### Behavioral evidence

- Original prerequisite: `/tmp/cmpct071-builtin-behavior-red.log`, five actual
  failures reporting the missing selected physical serializer before helpers
  were implemented. Missing-hook compilation failures are not counted as red.
- Strengthening RED: `/tmp/cmpct071-builtin-context-red.log` has Gemini's actual
  `maxOutputTokens: null` versus `1234` assertion. Its other two failures were
  invalid Responses tool-call fixture IDs, repaired before claiming parity.
- Document RED: `/tmp/cmpct071-builtin-documents-red.log` has Gemini's missing
  `authoritative-file-evidence` assertion. That run's Anthropic unsupported
  generic-document conversion was a fixture limitation, not an Anthropic fix.
- Gemini now serializes mandatory documents as their full textual representation
  and creates generation configuration for explicit max_tokens/temperature even
  when optional configuration is absent. Both preparation and physical send use
  this same implementation; unsupported media is not assigned invented occupancy.
- Final run: `/tmp/cmpct071-builtin-completion-green.log`, six main tests and one
  Unicode property test pass, zero failures. Exact invocation from `rust/`:
  `cargo test --profile ci-test -j 2 -p codelet-providers --test cmpct071_builtin_serializers --test cmpct071_builtin_serializers_unicode --no-fail-fast`.

The main target captures eleven actual physical POSTs: OpenAI Chat stream and
nonstream, Responses, Anthropic API-key/OAuth, two Gemini parameter/document
cases, and Copilot Claude/GPT/Gemini Chat plus GPT Responses. The property target
runs eight arbitrary Unicode descriptions across five actual adapters (forty
more physical POSTs). All 51 compare actual captured bytes with direct model
preparation, generation-aware preparation and `SelectedRequestPreparer` admission.
Assertions include no pre-send HTTP, unchanged source request, generation4,
conservative byte-count input estimate, output reservation, ordered assistant
tool-call/result suffix, instructions/reminders/tools/options, strict tools,
cache markers, and clone-stable OAuth metadata. These are physical-byte tests;
empty scripted responses do not claim successful provider response parsing.

### Gate integration contract

Use each actual selected model with
`SelectedRequestPreparer::new(model.clone(), complete_request_template, limits)`;
the template must retain its actual preamble, tools, documents, parameters and
known output reservation. `prepare(messages, advancing_generation)` returns
the actual selected physical body. The default generation-aware forwarding
uses the same serializer for these stateless builtin models. Copilot must use
its actual `CopilotHttpClient`, not a raw OpenAI client replacement. Anthropic
OAuth must reuse/clone the same constructed model between prepare and send.

Cold-process restoration, runtime submission/commit/promotion/reconciliation,
the custom/stub positive rows and the eight-row negative matrix remain owned by
their existing gate/runtime/provider owners. This serializer target is not a
substitute for those acceptance rows and does not mark CMPCT-083 complete.

### Traceability and quality

All seven scenarios have one test mapping and implementation mappings in the
dedicated `.feature.coverage`. Ranges were refreshed after test extraction;
`fspec audit-coverage builtin-provider-physical-serializer-parity` reports all
30 referenced files/mappings valid (`/tmp/cmpct071-builtin-completion-mappings.log`).
This is scenario traceability, not measured execution line coverage.

Scoped feature format, syntax and verbose tag validation pass; evidence is
`/tmp/cmpct071-builtin-completion-feature.log`. Scoped rustfmt checks pass for
the four serializer helper files, both integration targets and shared test
support (`/tmp/cmpct071-builtin-completion-format.log`). All seven new Rust
files are under 300 lines. Scoped `git diff --check` passes. Whole legacy provider
files retain their pre-existing formatting/macro whitespace issues, documented
in `/tmp/cmpct071-builtin-existing-format.log`; no unrelated reformat was made.

`cargo check --profile ci-test -j 2 -p rig-core -p codelet-providers --lib`
passes (`/tmp/cmpct071-builtin-completion-check.log`). Explicit patched Rig
Clippy also passes: `cargo clippy --profile ci-test -j 2 -p rig-core --lib`
(`/tmp/cmpct071-builtin-rig-clippy.log`). The final provider test-target Clippy
attempt in `/tmp/cmpct071-builtin-completion-clippy.log` is blocked by concurrent
core-owner edits: undefined `identity` at core persistence/compaction_commit.rs:186
and undefined `session_id` at persistence/manifest_commit.rs:91. This is not a
serializer failure or successful final Clippy run. The earlier identical target
run passed in `/tmp/cmpct071-builtin-tests-clippy.log`. Rerun the two-target
provider Clippy after the core owner finishes; no out-of-scope repair was made.
Current shared builds also expose unrelated non-provider warnings: duplicate
unreachable break in Rig agent streaming, unused Codex `is_user`, and custom
streaming clone-on-Copy; their owners retain those files. No lint suppression
was added. Parent busy-message delivery was rejected, so this handoff remains
the durable completion/blocker mailbox.

### Continuation files

This continuation modified Gemini `completion.rs`, the main builtin test target,
the dedicated feature/coverage and this handoff, and created
`rust/providers/tests/cmpct071_builtin_serializers/support.rs` and
`rust/providers/tests/cmpct071_builtin_serializers_unicode.rs`. The earlier
serializer-owned helpers and wiring listed above remain intact alongside
other owners' pre-existing usage changes. No ownership outside the assigned
provider subtree/tests/specification was taken.

### Final quality retry — 2026-10-18

The concurrent core compilation blocker above is resolved. The exact final
provider test-target Clippy retry now passes:
`cargo clippy --profile ci-test -j 2 -p codelet-providers --test cmpct071_builtin_serializers --test cmpct071_builtin_serializers_unicode`
(`/tmp/cmpct071-builtin-final-clippy.log`). The continuation retry also passes
after waiting for the shared build lock. Its remaining warnings are the unused
`codex` import in provider manager, unused Codex `is_user`, and custom streaming
clone-on-Copy; no serializer lint suppression or out-of-scope repair was made.

The final shared-cache test rerun also passes, six main scenarios plus one
Unicode property test, with zero failures:
`cargo test --profile ci-test -j 2 -p codelet-providers --test cmpct071_builtin_serializers --test cmpct071_builtin_serializers_unicode --no-fail-fast`
(`/tmp/cmpct071-builtin-final-green.log`). No serializer-owned blocker remains.
Cold runtime acceptance remains gate-owned and is not established by this rerun.
