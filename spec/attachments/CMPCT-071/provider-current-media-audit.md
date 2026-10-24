# Current provider media and typed-error audit

Provider/non-provider-Rig owner e22f772a resumes the inherited implementation,
not a replacement compaction strategy. The current 083 matrix still reports
cold 7/11 and negative 6/8; previous provider-only passes do not close that gate.

The actual selected Z.AI requests contain native tool definitions. The shared
`selected_request::media_occupancy` currently recursively interprets schema
property names such as `image` and `source` as supplied opaque media, blocking
otherwise textual requests. New feature `selected-request-media-semantics`
precedes the native-adapter behavioral target `cmpct071_media_semantics`. The
RED command is queued behind other owners in the shared ci-test cache; no
production repair is applied before actual behavioral RED. All serialized tool
schema bytes must remain included in admission. Actual image content must still
return typed unknown-media failure without transport or checkpoint mutation.

The two typed Codex negative controls already expose the real runtime failure.
Provider `CheckpointError` is boxed into `CompletionError::RequestError`; rejected
WebSocket upgrades are boxed into `HttpError::Instance`. Both preserve sources.
The actual loss is CLI `interactive/stream_loop.rs` return sites 1837 and 2249:
`anyhow::anyhow!("Agent error: {e}")` replaces the source with display text.
CLI should preserve the original error with context, maintaining blocked
inference and existing no-fallback assertions. Coordinator and CLI direct
messages were busy-not-queued; this attachment is the durable handoff.

Provider checkpoint precedence/idempotent boundary and malformed-state tests
already exist and will be rerun, not weakened. The real Codex child finite-positive
context-limit restore failure belongs to sessions/CLI; native Gemini semantic
pairing belongs to the Rig-provider owner. No cross-owner files are changed.

## Executed media and direct typed-transport verification

`/tmp/cmpct-provider-media-semantics-red.log` executed both scenarios before
the repair: textual schema preparation failed with `UnknownMediaOccupancy`.
The real-image fixture initially omitted Claude's required image media type;
the fixture now supplies PNG metadata rather than accepting serialization
failure as the unknown-occupancy result.

`/tmp/cmpct-provider-media-semantics-green.log` completes with Cargo exit 0:
`cmpct071_media_semantics` 2/2, `cmpct071_selected_preparation` 2/2 and
`cmpct081_explicit_transport` 4/4. Command: `cargo test --profile ci-test
-p codelet-providers --features test-support --test cmpct071_media_semantics
--test cmpct071_selected_preparation --test cmpct081_explicit_transport -j 2`.
Native schemas remain physically serialized and every byte is counted;
real images still return typed unknown-media failure without sending or
changing history. All rejected-upgrade cases preserve their typed source.
The gate owner must rerun actual selected Z.AI normal and plan requests;
these eight provider tests do not establish full runtime-gate success.
