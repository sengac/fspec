# PROV-155 — Codex cache affinity and turn routing

## Goal and evidence

Carry explicit cache affinity and server turn-state through Codex requests without cross-session
leakage. Audit: fspec `38a95522`, upstream https://github.com/openai/codex `bb64b89`.
Official client.rs derives prompt_cache_key and session identity and preserves x-codex-turn-state.
Our Codex request builder does not set a cache key and shared AdditionalParameters cannot retain it.
This is an affinity gap, not evidence that existing server-side prompt caching never occurs.

## Codex-only boundary

Depends on PROV-147, PROV-152 and PROV-153. Own key generation and routing state in Codex session
state/request serialization. Do not change cache policy for OpenAI, Claude or other providers.
Do not place OAuth tokens, user prompts or private identifiers in observable cache keys.

## Acceptance criteria

1. Requests in the same Codex session carry a stable opaque cache-affinity key under documented lifecycle rules.
2. Independent sessions, accounts and subordinate-session lifecycles do not accidentally share routing state.
3. Server turn-state is retained and returned only within its valid Codex turn/account scope.
4. Session reset, account change and stale-state recovery invalidate state according to a documented policy.
5. Missing affinity support gracefully uses ordinary SSE requests, without claiming guaranteed cache hits.
6. Cached token usage remains accurately reported and other providers' keys and headers are unchanged.

## Test examples and implementation notes

Compare repeated requests in one session with requests from two independent sessions. Test account
switching, turn completion, retry, sub-agent creation and explicit reset using synthetic identifiers.
Verify upstream session/thread/turn-state lifetime rules before encoding headers. Keep routing data
out of normal logs; metadata extraction is owned by PROV-153.

## Validation and exclusions

Gherkin first and failing request snapshots/local-server tests second. Use shared helpers and
scoped Cargo targets, including non-Codex controls. Cache hit-rate benchmarks are optional live
validation and require opt-in. Cache retention controls not present in the audited upstream are
not part of this card. Automatic previous_response_id continuation belongs to PROV-156.
