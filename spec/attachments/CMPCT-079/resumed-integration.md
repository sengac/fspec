# Runtime owner continuation

Owner b722dc96 is actively wiring 057/058/079 on 2026-10-18.

- Durable 059 `Session::capture_durable_compaction` and `commit_durable_compaction`
  are present. Runtime callers must capture before building, clone071's preparer,
  commit and reconcile before emitting completion. Queued never resets attempts.
- Editing compactor_sub_agent, compaction_operation, affected stream restart and
  generate handler sections, and pending application lifecycle. Please avoid
  overlapping edits there; Session compaction remains059 ownership.
- Fresh compile `/tmp/cmpct057058079-resume-runtime.log` is blocked by provider
  custom module extraction imports (`super::http` / RhaiCustomProviderModel).
  These are unrelated065/069 shared edits, not behavioral evidence.
- 053/054 existing evidence is not done: latest053 log has failure returning
  `Ok(true)`; 054 historical red is explicitly unverified. Fresh actual tests
  and honest evidence are required before any lifecycle completion.

071 runtime requirement: expose `Session::compaction_request_preparer() ->
Result<SelectedRequestPreparer, RequestAdmissionError>` cloning the installed
preparer.079 needs it to call059's mutable commit without borrowing Session in
the callback.079 will add `Session.compaction_working` only, not your preparer
field. Agent-loop pending application will use captured durable source, never
recapture after candidate generation.
