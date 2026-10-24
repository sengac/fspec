# CMPCT-076 implementation checkpoint

Completed 2026-10-17; no commit created.

- HTTP setup rejections now carry `CodexRejection` through the model stream,
  preserving bounded body, status, provider code/message, request ID, retry advice
  and physical attempt count. SSE failures use the same typed diagnostic.
- Only the declared exact `context_length_exceeded` code classifies overflow.
  Malformed, incomplete and oversized HTTP evidence remains unknown.
- Evidence collection is limited to 8192 bytes and at most one second. Request
  credentials, individual cookies and escaped/partial credential echoes are redacted;
  response headers are allowlisted rather than copied wholesale.
- The original failing-test evidence remains in `/tmp/cmpct076-*-red.log`.
  Resume verification: six scenario integration tests and nine unit/property tests
  pass (`/tmp/cmpct076-resume-tests.log`). Prior check, Clippy and neighboring
  transport/lifecycle/recovery tests also pass; their logs are `/tmp/cmpct076-final-*`
  and `/tmp/cmpct076-neighbor-tests.log`.
- All six scenarios have test and implementation mappings; `fspec audit-coverage`
  passes. The feature validates. Repository-wide tag validation still reports 526
  pre-existing feature violations, unrelated to this feature.

This completes the adapter diagnostic slice, not the full compaction epic.
CMPCT-082 owns removing the broad caller-side recovery classifiers.
