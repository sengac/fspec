# Diagnostic logging privacy

fspec's Rust diagnostic sinks omit system instructions, prompts, completions,
provider bodies, and tool arguments/results by default. `RUST_LOG=debug` or
`RUST_LOG=trace` changes severity **not** payload authorization. Logs retain
targets, levels, timestamps, span names, session/request/tool-call IDs, token
counts, durations, byte counts, and actionable error messages/chains.

## Deliberate local payload diagnostics

For a short, controlled reproduction using synthetic inputs:

```sh
FSPEC_LOG_PAYLOADS=1 RUST_LOG=info fspec
```

Use the normal fspec invocation for your mode. Only the exact value `1` enables
payloads. The setting is read when tracing starts; restart the process to change
it. It applies only to the **combined/client local file formatter**. Restart
without the variable immediately after reproducing the issue. It does not
retroactively change records already written.

- Each payload value is redacted and limited to 4,096 Unicode characters with
  `[TRUNCATED]` when bounded. Span payload updates appear on the next event
  rather than being repeated in every descendant event. Newlines are escaped.
- Credential-named fields (authorization, API keys, passwords, secrets, cookies,
  access/refresh tokens) are never exposed, even with opt-in. Recognizable
  Authorization/Proxy-Authorization and Cookie/Set-Cookie headers redact the
  complete quoted or newline-delimited value, including Basic credentials and
  trailing cookie pairs. `Bearer ...` tokens and credential `key=value`/JSON
  assignments are redacted before bounding. Complete actionable errors are not
  size-truncated.
- Valid JSON embedded in diagnostic strings, including application provider
  errors, applies the same content-key policy as structured fields. Default
  diagnostics omit conversation/body keys but preserve error messages, status,
  codes and surrounding correlation. Local opt-in allows bounded redacted values.
  This includes JSON inside Debug-quoted streaming HTTP errors and provider
  preamble keys such as Anthropic `system` and OpenAI Responses `instructions`.
  Rust Unicode Debug escapes are decoded without changing literal backslashes.
  Headers in decoded JSON strings are redacted before reserialization so a
  credential header cannot consume the following root cause or status fields.
- This is **not perfect secret detection**: a key pasted as arbitrary prose,
  custom credential names, malformed JSON, unknown content keys, signed URLs,
  bare base64/binary content, or private data
  in provider/tool error messages may evade these rules. Do not use real secrets
  in reproductions. Treat all diagnostics as sensitive and review before sharing.
- Payload authorization never propagates to daemon stderr, RPC/WebSocket log
  broadcasts, or the common `log.entry` debug stream. The common logging
  initializer also remains metadata-only. No telemetry exporter is installed or
  authorized by this switch; a future exporter must apply its own privacy boundary.

## Storage, access, and retention

New diagnostic files live in `~/.fspec/logs/private/`:

| Source | Daily filename prefix | Payload opt-in |
|---|---|---|
| Combined fspec process | `fspec-combined.log` | Yes |
| Client fspec process | `client.log` | Yes |
| Common Rust logging initializer | `rust` | No |
| Daemon | stderr, owned by its launcher | No |

On Unix the private directory is enforced to mode `0700` **before** opening the
appender. File access is protected by that directory even if the process umask
permits broader file modes. Do not relocate it into a shared directory or loosen
its permissions. On Windows, restrict the directory ACL to the running user;
Unix permissions cannot provide that guarantee there. Do not synchronize these
logs to cloud storage or include them blindly in diagnostic bundles.

Daily rotation retains at most **five files per prefix** in the new directory.
Pruning occurs on appender startup/rotation, not on a wall-clock timer: an inactive
process's last file can remain indefinitely. This is a file-count bound, not a
byte quota or a promise that every file disappears after five days. External
stderr collectors must enforce their own access and retention policy.

After investigation, stop all relevant processes, review the new directory, and
remove the reproduction's payload logs with your normal secure data-handling
process. Rotation does not erase backups, snapshots, copied bundles, or blocks
on disk. **Existing legacy logs are not deleted or sanitized by this change**:
review `~/.fspec/logs/fspec-combined.log.*`, `~/.fspec/client.log.*`, and older
common logs separately, and remove or restrict sensitive historical records in
accordance with your retention policy. Log consumers must migrate to the new
private directory; common diagnostics now use the same privacy-aware text format.

Conversation/session storage is a separate product capability and is unchanged.
The explicit debug-capture product may store conversation/tool data independently
of diagnostic `log.entry` records; this switch does not authorize or erase it.

## Adding emitters and sinks

Put content in structured payload fields (`input`, `body`, `args`, `result`,
`content`, or rig's `gen_ai.*` content fields), never interpolate it into a log
message. Keep correlation and root causes in separate metadata/error fields.
Rig's verbose DEBUG/TRACE messages are treated as payloads at fspec sinks;
otherwise arbitrary message prose is considered diagnostic text. Changing a
log level must never enable bodies. New content fields and sinks need synthetic
marker runtime tests, including nested span updates and credential failures.

Regression specification: `spec/features/privacy-conscious-diagnostic-logging.feature`.
