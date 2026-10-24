# Service access through the Fspec frontend

`fspec client --mcp --connect ws://127.0.0.1:PORT` selects a stdio MCP
presentation mode in the existing client. Without `--mcp`, the TUI behaves as
before. Omitting `--connect` uses the existing daemon discovery file.

The path is MCP host → Fspec client → WebSocketFspecBackend → existing
tarpc/bincode WebSocket protocol → SharedFspecService. It does not replace
the daemon, add a model provider, or open a listening socket.

## Tools and compatibility

The compatibility tools `fspec_health`, `fspec_work_units`, and
`fspec_work_unit_detail` remain available. A matching daemon additionally
advertises every current RPC as `fspec_rpc_<method>` and `fspec_rpc_result`
for continuation. Discovery is paginated at 20 tools per page. Typed argument
schemas are generated from Rust signatures and checked by deserialization;
unknown top-level fields are rejected. Inputs are capped at 64 KiB. Native
RPC/domain validation still applies.

Full RPC dispatch requires the exact schema fingerprint negotiated during
WebSocket handshake; older daemons keep only the compatibility tools.
Installing and starting an updated daemon is a separate user action.

## Authorization and transport

The adapter forwards valid typed RPC calls without adding a confirmation-form
protocol. Caller authorization, host/runtime permissions, and Fspec's existing
daemon and agent approval controls still apply. Exposing a tool does not
authorize arbitrary use.

Pause and HITL replies bind the current pending request's UUID before dispatch.
The daemon checks and consumes this identity under the same lock used to
replace pending requests, rejecting stale or repeated answers. The bridge
forwards the caller's explicit answer; it never chooses an approval itself.

The daemon has no per-user authentication in its WebSocket accept path.
MCP mode therefore accepts only numeric loopback `ws` addresses, without
URL credentials, query strings or fragments. Local stdio access relies on
the launching process's OS permissions; this is not multi-user isolation.
Do not expose the raw daemon port. Remote operation requires an explicitly
authorized authenticated transport to a loopback endpoint. The client does
not configure tunnels, remote authentication, or MCP host registration.

## Results and failures

RPC calls have a 30-second deadline and are never automatically retried.
A timeout or lost connection means execution is unknown. Daemon errors may
reflect partial changes. Unit results acknowledge dispatch, not agent-job
completion.

Result text is JSON serialized once and returned in 4096-Unicode-scalar chunks.
Continue with `fspec_rpc_result(result_id, offset)` and concatenate `text`
before parsing. `has_more`, `partial`, `next_offset`, and `total_chars` make
completeness explicit. Partial results are retained for 300 seconds, up to
eight snapshots and 32 MiB total. Existing snapshots are not evicted to hide
a missing tail. Capacity or serialization failures after execution are
reported explicitly and must not trigger a mutation retry. Continuation
does not call the daemon again.

Generic legacy RPCs can still fetch a complete result into memory before
chunking; this is not backend pagination. Prefer the bounded work-unit tools
or `fspec_rpc_get_transcript_page`. Output is limited to a 256 KiB
presentation budget. Project text is untrusted data, never instructions.

## Bounded work-unit reads

`fspec_work_units` returns ID-sorted summaries with `items`, `total`,
`returned_count`, `has_more`, `next_cursor`, and `revision`. Default page size
is 20; `limit` must be an integer from 1 to 50. Continue by sending both
`cursor: next_cursor` and `revision`. A changed full-list revision rejects
continuation: restart from the first page. Reordered backend lists alone do
not invalidate it. The revision is a change detector, not a retained snapshot
or authorization token.

Each summary contains id, title (256 Unicode scalars), status and workType
(64 each). `truncated_fields` explicitly names shortened fields;
`summary_only` and `omitted_fields` identify omitted detail. IDs are never
truncated; malformed, duplicate, or over-512-byte IDs fail the read.

`fspec_work_unit_detail` accepts `id` and returns the entire serialized record
as JSON-text chunks of at most 4096 Unicode scalars. Concatenate `text`
chunks before parsing the record. The outer MCP response is always valid
JSON. `offset_unit`, `offset`, `total_chars`, `partial`, `next_offset`, and
`revision` make partial reading explicit. Continue with the same id,
`offset: next_offset`, and revision. Changed records, missing or ambiguous
IDs, invalid offsets, and missing continuation revisions fail explicitly.

The frontend requests `/fspec-read-v2` during WebSocket upgrade and uses
`read_work_units_bounded(detail, arguments)` only when the response advertises
`x-fspec-read-version: 2`. This connection sends no unsolicited workspace,
session, or log snapshots. `backend_fetch: "bounded_rpc"` identifies bounded
daemon-to-client responses. The daemon still clones and scans its local
watcher snapshot; this bounds wire responses, not daemon memory or computation.

Old daemons use the existing full-list RPC and return
`backend_fetch: "full_list"`. Every page or detail request still transfers
and materializes that list locally, and old servers may push their initial
snapshot. This fallback bounds assistant output only. Read responses retain
the 256 KiB safety limit and 10-second RPC deadline.

## Verification and maintenance

Run the scoped integration tests:

```sh
cargo test -p codelet-fspec --test client_mcp -j 1
cargo test -p codelet-rpc --test mcp_coverage
```

Run `python3 scripts/generate-mcp-rpc.py` after changing `FspecService`.
Generated typed calls fail compilation for incompatible signature changes;
the coverage test detects missing mappings. Read-only classification is an
explicit audited list; newly added methods default to state-changing
annotations. The build-time compatibility fingerprint covers the service
source and every RPC wire-type source file. It is a conservative compatibility
check, not authentication or a security token.
