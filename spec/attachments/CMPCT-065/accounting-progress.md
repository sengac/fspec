# Accounting completion progress — CMPCT-065/066/069

Coordination from 074 owner 9721cd0e: I own Codex streaming.rs and latest-context
usage separate from aggregate final. Plan additive `GetTokenUsage` default
`context_token_usage()` and optional explicit latest usage metadata on Responses
final (aggregate billing untouched); hook consumes context method, not billing.
Please preserve this additive method when normalizing Responses usage. Need
normalization to remain request-scoped for both latest and aggregate. 067 will
consume the same normalized current snapshot; see 074 implementation handoff.

074 ci-test currently cannot compile: your Gemini completion.rs:404 calls
`usage.token_usage()` without `GetTokenUsage` in scope (two E0599). Please fix
your import; log `/tmp/cmpct074-wire-red-ci.log`. Not behavioral red yet.

Owner: c3fcefab-0638-47f5-924b-1358abec76ae. No commits. Scope excludes 067/074/075.

## Remaining work at takeover

- 065: run existing four provider normalization scenarios to capture behavioral red;
  normalize actual OpenAI/Responses, Anthropic and Gemini live/final boundaries;
  inspect Copilot/custom provider routes, preserve raw details and unknown semantics;
  add property coverage and verify context/hook consumers count inclusive subsets once.
- 066: capture failing scripted Anthropic SSE tests; merge request-scoped start/delta
  usage without losing absent versus zero; cover malformed/incomplete usage and finish
  hook boundaries, then run all scenarios green.
- 069: existing JSON forwarding changes are preserved but are not completion.
  Specify and reproduce stable physical-request settlement, intermediate/retry/duplicate
  events, cancellation/failed-final policy, explicit persistence errors and real runtime
  invocation using the shared manifest transaction API. Preserve cache/reasoning/billing
  fields through actual bridge/subordinate and durable store routes.
- All three: maintain scenario-to-test 1:1 coverage, scoped ci-test evidence,
  formatting/feature/tag validation, work-unit status and final runtime handoff.

## Evidence

Resume after compaction: `/tmp/cmpct069-forwarding-behavioral-red.log` now has
two actual assertion failures: missing reasoning/billing keys and omitted unknown
keys (not null). Patched the real bridge/subordinate projection to retain them.
RigAgent already boxes its observer stream; rerunning provider065/066/069 targets
to verify current green and find remaining runtime gaps. Coverage files are still
incomplete and statuses must not be marked done until refreshed.

Current verification `/tmp/cmpct065066069-current.log`:065 six,066 four,
069 four ledger + one actual Codex continuation runtime tests pass. Common CLI
stream entry already binds `with_settlement_session(session_id)`, including
background and Gemini continuations; background Done no longer adds billing.
Remaining concrete gap found: custom-provider live/nonstreaming Usage still sets
total=input+output while final includes disjoint cache/reasoning. Add scripted
adapter scenario red before correcting all three custom usage routes. Also verify
ordinary (non-Codex) real finish/persistence-error paths and duplicate Done through
actual BackgroundOutput, refresh coverage, run scoped quality gates, update status.

Resume 12:xx: provider065 six tests and066 four tests currently green in
`/tmp/cmpct065066069-resume.log`;069 ledger four tests green. Real069 runtime
red `/tmp/cmpct069-runtime-red.log` asserts ledger length1 versus2 (physical
continuation collapsed). New075 explicit capacity guard now requires runtime
fixture limits; fixture updated, rerunning. Implemented physical usage observer
but CLI/background binding and removal of Done-derived spend remain pending.
Plan bind session once at common CLI stream entry via RigAgent, encompassing
all retry/Gemini continuation hooks. Persistence failures must surface as stream
errors, not merely hook cancellation. No completion/status claim yet.

074/075 update (9721cd0e): `GetTokenUsage::context_usage()` now defaults to
token_usage, Codex Responses overrides from latest RequestContext (unknown stays
None). `request_context()` exposes model/generation/physical UUID/optional usage.
`request_usage_records()` returns `(String, Usage)` for every measured internal
Codex physical request on final; spend and context are independent. Responses
final has defaulted request_context/request_usage fields. Hook consumes latest
inclusive normalized context only. Please use these record identities in069.

## Resumed runtime plan (069)

- Add request ledger via existing `update_session_manifest`, retain raw normalized
  category Options, checked totals and conflict detection, independent context.
- Add generic RequestSettlementHook wrapper allocating UUID at physical hook start;
  duplicate finish retains identity, retry gets fresh identity. Unknown final means
  no settlement; measured final remains billable even if later work is cancelled.
  Failed persistence cancels inference and exposes a typed stored error.
- Bind wrapper to every CLI/background stream and continuation hook; Done only
  projects context and never adds spend from display counters.
- Coordination for 074/075: Codex internal continuations are multiple physical
  requests behind one hook. Need additive `GetTokenUsage::request_usage_records()`
  returning `(String, Usage)` records from Responses final; default empty uses
  normal hook request identity. Please preserve this seam when updating Responses
  DTO. Parent implementing ledger/wrapper first; don't conflate aggregate spend
  with latest context. Busy session messages were not delivered.

Verified existing log `/tmp/cmpct065-green-066-incomplete-red.log` contains real
Anthropic terminal red assertions (cache None vs Some(400), zero None vs Some(0),
missing-start no error). Current RequestUsage implementation still needs green.
`/tmp/cmpct069-forwarding-red.log` is a compile blocker, not behavioral evidence.

Initial inspection: existing tests and feature files reviewed; no new implementation
or test result claimed yet. Shared worktree changes belong to multiple workers and
will not be reset or reverted.
# Cross-owner build blocker from068/083

`/tmp/cmpct068-publisher-current.log` (current run) fails with ten CLI E0277
errors: `observe_request_usage` wraps the formerly Unpin stream in an unpinned
async_stream. Please return a boxed pinned stream (or pin at the owning shared
RigAgent seam) so existing stream.next/select consumers retain their contract.
The same blocker prevented `/tmp/cmpct083-cold-current.log`. This is not behavioral
red evidence. Busy message to065 owner was not queued.
