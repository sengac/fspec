# Remaining selected-card source preparation

This is a read-only discovery handoff, not implementation or runtime evidence.
No remaining card is Done. BUG-265 has separate source/spec/test preparation in
`../BUG-265/preparation-checkpoint.md`. BUG-266/267/268/269 and PROV-162 retain
their existing criteria and backlog state. CONT-011 remains last.

## BUG-266: custom stream settlement

Confirmed current boundaries: `custom/stream_http.rs` reads after stop and
settles immediately on `[DONE]`; `custom/rig_streaming.rs` emits Final only after
the converted stream ends. The stream processor is 316 lines and needs an
approved bounded mechanical split before edits under the file-size standard.
No such split approval is inferred from BUG-265's approval.

Prepare real scripted-provider regressions for trailing usage, `[DONE]` with a
still-open connection, clean EOF with stop, missing terminal evidence and a
transport failure after stop. Existing `bug227/bridge.rs` explicitly preserves
the last two distinctions. Do not turn a mere finish reason into proof that all
usage has arrived, or conceal a post-stop transport error. Use stream-state
transitions rather than unbounded EOF waiting or a timeout that drops late usage.
The protocol's actual settlement boundary must be specified before choosing code.

## BUG-267: selected implementation identity

Current `agent_loop/routing.rs` returns provider/model/thinking strings after
facade selection; `agent_loop/providers.rs` matches built-in strings to native
getters. This loses a selected `ProviderType::Custom` identity when its name
collides. Existing `custom_provider_script_shadowing_tests` verify resolution
and the disable-shadowing escape hatch, not the real shared dispatch.

The regression fixture must run a colliding script through normal provider
selection and shared-loop dispatch into a loopback endpoint. Assert script
request schema, native Done registration/acceptance and no accidental native
getter. Include explicit facades, aliases, noncolliding custom and genuine
built-in controls. Preserve typed identity without silently changing the
existing explicit-facade contract or bypassing credentials/admission.

## BUG-268: Copilot actual endpoint

Existing middleware tests mostly inspect selector output, formatted URLs and
request classification. They cannot establish that a native Copilot agent sends
the chosen request dialect. The current provider file is already 292 lines;
avoid pushing it past 300 or doing an unapproved architectural rewrite.

First reconcile the documented supported model/endpoint policy, then prepare
real local HTTP capture for both selected routes, required headers, opaque
reasoning, tool-result identity and armed stop/nudge/Done continuation. Reuse
generic Responses behavior from completed BUG-264; do not reopen its policy or
claim dedicated Codex tests establish Copilot behavior. Credential refresh tests
must use synthetic storage and a controlled endpoint, never real configuration.

## BUG-269: scripted OAuth actual request path

`ScriptedRefreshingClient::ensure_fresh_if_needed` still reads synthetic/stored
tokens, evaluates refresh policy and persists refreshed tokens as a helper.
`custom/rig_streaming.rs` obtains script headers then calls ordinary `post_sse`;
the helper is not integrated at this observed request boundary.

Prepare actual streaming and nonstreaming loopback requests with expired fake
tokens and a fake issuer. Assert refresh precedes Authorization, refreshed
storage is used once, and failure/transient error/cancellation/deadline outcomes
remain honest. Concurrency requires a deliberate single-flight ownership
contract, not another blind refresh. Preserve scripts that intentionally own
refresh in their header callback. No keyring/SDK/download or real provider probe
is authorized; do not repeat BUG-261's unsafe fixture routing.

## PROV-162: explicit policy gate

`codex/lifecycle.rs` still permits selected completed `end_turn:false` or
interrupted incomplete continuation without client tools, then restricts replay
to nonempty text-only assistant output. This is an explicit PROV-148 policy,
not permission to broaden accepted typed history. PROV-149 lineage and completed
BUG-260 refusal preservation do not silently approve a new continuation policy.

Supervisor/native discovery must obtain the exact allowed typed item families,
empty-output handling, finite follow-up cap relationship and refusal/filter
behavior before changing policy. Preserve opaque lineage, request admission,
cancellation and physical usage across each approved continuation. No human
answer, approval or protocol relaxation is fabricated in this preparation.

## CONT-011 final matrix

Retain immutable-baseline evidence for known stale source assertions from
BUG-262/263. Correct stale source readers without weakening behavioral invariants;
do not label extraction-induced failures as baseline. Completion still requires
all remaining cards' real red/green evidence, final-source provider/runtime
matrix, quality and native workflow gates. BUG-254 remains specifying and
Windows work remains outside the selected batch.
