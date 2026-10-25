# Approved Copilot composition contract

## Controller correction — authoritative current acceptance

The model-owned clearing-at-genuine-turn clauses below are preserved ONLY as
superseded chronology, not the current contract. Original immutable PROV-055
and PROV-057 scenarios require the owned opaque value on the NEXT genuine TUI
turn. Controller review of b2475dcd disproves the earlier narrower reading.
The current candidate is `session-lineage-correction.md`: nonpersistent live
SESSION ownership must survive legitimate provider/model reconstruction. Restart
retention is not required. No opaque integration was implemented under the
superseded model-only design. BUG-268 has returned natively to specifying.

The native supervisory responses to bug268_typed_dialect_composition,
bug268_refresh_and_opaque_contract and bug268_opaque_ownership_gap approve a
bounded integration, not fabricated human speech or live provider conformance.
The repository selector is unchanged: GPT-N >=5 uses Responses except exact
gpt-5-mini; every other model uses Chat. Official model availability/BYOK docs
do not establish authenticated Copilot proxy acceptance of every such model.

Typed client/model composition must delegate existing adapters for client.agent,
native create_rig_agent and nonstreaming completion. Two old Chat-only type
assertions may change to the real routed API while retaining middleware guards.
Construction-only extraction is approved if needed to remain below 300 lines.

## Opaque representation and ownership before implementation

Only the known encrypted Copilot reasoning_opaque field is eligible, preserved
uninterpreted as its JSON value. It is neither assistant text nor caller-supplied
request metadata, and is never persisted/restored or logged. A freshly created
typed model owns one latest lineage; only its clones share that owner. Ownership
is bound to the originating logical GitHub account, deployment/base URL and exact
model. Short-lived Copilot token renewal does not replace the logical account.

Before sending a follow-up, require unchanged preamble and an exact typed-history
prefix consisting of the previous admitted request history plus its successfully
settled assistant output (including tool identities). Normal subsequent tool
results/nudge may extend that prefix; incompatible/rewritten/cleared ancestry
invalidates the opaque value. Never adopt a caller-supplied/restored opaque value.
Failed/cancelled settlement invalidates it. Concurrent or superseded attempts
cannot install stale state. This is an ephemeral owner, not a durable replay or
ProviderSession subsystem; ProviderSession currently registers only Codex.

The real manager reconstructs CopilotProvider and create_rig_agent constructs a
fresh model: opaque state clears at these genuine-turn/restart boundaries by
design. Tests must exercise these boundaries, clones, incompatible/cross-owner
histories, serialization exclusion and normal same-account credential renewal.
No survival across genuine-turn reconstruction or restart will be claimed.

## Refresh and fixture constraints

Reuse existing freshness/coalescing/persistence/error policy at actual requests,
with no automatic inference replay. A nondefault test-support-only in-process
issuer seam may accept only numeric-loopback URLs, rejecting DNS/nonloopback,
userinfo/fragments and redirects escaping fixtures. Prove seam isolation before
expired-token fixtures. All auth/config/keyring data is synthetic and temporary;
no external issuer, user auth loading or paid provider calls are authorized.

Source review also finds CopilotHttpClient freezes its constructor token while
ensure_fresh updates only auth, and generic Responses currently drops the known
opaque field. These are acceptance gaps to test and repair, not evidence that
the endpoint switch alone preserves their behavior. No Codex policy is changed.
