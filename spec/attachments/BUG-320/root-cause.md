# BUG-320 root cause

These failures predate this card: both tests fail on the committed code, with or without CMPCT-086. Both tests had gone
stale against newer engine contracts, and no production change was needed.

- **Usage now required.** The patched rig OpenAI decoder rejects streams without a usage block
  (`patches/rig-core/src/providers/openai/completion/streaming/decoder.rs:161`, "missing OpenAI usage"). The cmpct052
  mocks had none.
- **Attribution now required.** `AdmissionMiddleware` (`rust/tools/src/tool_admission.rs`) refuses tool calls without
  a top-level `work_unit_id`, and neither test's `inject_summary` arguments had one. The rejection scenario was
  therefore passing for the wrong reason: the call was refused at attribution, so the DAG was never validated. It now
  asserts the "invalid DAG candidate" error.
- **The DAG covered more than its source.** The bug214 deferred DAG claimed `turns="0-2"`, but `TurnCompaction`
  captures the durable source before the new prompt is admitted, so only turns 0–1 exist. Validation correctly rejected
  it with "provenance exceeds persisted history", so `CommittedCompaction` was never published.
- **Measured tokens.** With real usage, the rejected request measures 100 prompt tokens. That counter is now expected
  at the measured value, and every other counter stays pinned.
