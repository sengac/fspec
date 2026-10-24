# BUG-215: Missing compaction output reservation

## Scope and confidence

- Investigated on 2026-10-21 against source at `7e60de8e`.
- Confirmed runtime failure and a source-confirmed raw/effective-limit mismatch.
- Investigation used source, history, logs and persisted session data. No live provider calls or regression tests were run.

## Observed failure

At 03:49:37 UTC, session `dc428d30-6e5e-4225-bfe4-c09f6b252859` failed compaction with:

> compaction builder failed: request admission requires an explicit positive output reservation

The record is in `~/.fspec/logs/fspec-combined.log.2026-10-21:1536`. The affected durable session had no committed compaction. Successful compactions in other sessions are not recovery evidence for this failure.

This is later than the 13:11 local-time Qwen screenshot. Do not attribute that earlier stopped response to this error; its separate investigation is BUG-221.

## Source chain

1. `rust/agent-loop/src/generate_compaction_handler.rs:27`: `Builder::capture` stores `raw_model_context_window()` and `raw_model_max_output_tokens()`.
2. `rust/providers/src/manager.rs:1177`: the raw output accessor returns `None` when registry/user metadata is absent, although `max_output_tokens()` at line 1131 resolves an effective fallback.
3. `rust/agent-loop/src/compaction_builder.rs:127`: the optional captured output value becomes the request's output reservation.
4. `rust/providers/src/request_admission.rs:32`: admission rejects an absent or nonpositive reservation. The exact error text is defined at line 77.

Fail-closed admission is correct. The caller must supply a truthful, resolved output allowance rather than silently removing the reservation requirement.

## Reproduction outline

Create a profile session whose provider has an effective positive output allowance but no explicit profile/registry output metadata. Trigger the captured-builder compaction route. Assert that the admitted builder request uses the actual resolved allowance and fits the complete input plus reserved output within capacity.

This outline has not been executed as a new regression.

## Existing coverage and missing regressions

- Review `rust/agent-loop/tests/bug211_synthesis_admission.rs` and `rust/providers/tests/cmpct071_serialized_admission.rs` for admission fixtures.
- Review `rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs` and `rust/agent-loop/tests/support/captured_compaction_builder.rs` for real builder routes.
- Add coverage for missing metadata with an effective fallback, explicit overrides, provider hard caps, and a genuinely invalid reservation.
- Cover both generated and registered compactor entry points; they share captured builder state.
- Verify cancellation still prevents promotion and does not convert admission failure into success (BUG-214).

## Recommended direction and questions

Resolve limits consistently with the request-issuing provider before admission; ensure the serialized request uses the same reservation. Do not invent capacity metadata or bypass admission when metadata is unavailable. Determine whether analogous auxiliary callers also pass raw optional limits, and scope any follow-up from evidence.

Related completed work: BUG-211, BUG-213 and BUG-214. Those cards address budget fallback, activity and cancellation, not this missing reservation.

## Separate follow-up incident

The 2026-10-21 15:32:33 local screenshot concerns session `c655c22f-b638-4189-890a-616fb9227f99`, which failed with `complete request input 947739 plus output reservation 128000 exceeds capacity 1050000`. Its reservation is present; supplying provider defaults does not fix its over-capacity exploration. That incident and occupancy-aware synthesis runway are tracked separately as [BUG-229](../BUG-229/research.md).

## Implementation and regression evidence

- Follow-up development on 2026-10-21 changed only shared `Builder::capture` to capture `Some(context_window())` and `Some(max_output_tokens())`. Raw-accessor semantics and fail-closed admission remain unchanged.
- `spec/features/compaction-builder-effective-limits.feature` maps four scenarios 1:1 to `rust/agent-loop/tests/bug215_effective_compaction_limits.rs`; every scenario exercises both registered and self-target GenerateCompaction routes with real persisted originals and a local HTTP provider.
- Before implementation, absent output metadata failed with `request admission requires an explicit positive output reservation`, and absent context/output metadata failed with `request admission requires an explicit positive context capacity`. The explicit-limit/provider-clamp control passed without the fix.
- After implementation, all four scenarios pass, including explicit overrides, a provider hard-cap resolver using an OpenAI-compatible facade, zero/invalid limits, final evidence retention, and durable originals preservation. This is propagation coverage, not a native Claude transport regression.
- The seven-target agent-loop run passed 19 tests, retaining BUG-211 depth/synthesis admission and BUG-214 cancellation coverage plus CMPCT-056 caller interruption and target identity checks. Commands were package/target scoped with dev/test debug info disabled.
- `cargo check -p codelet-agent-loop`, `cargo clippy -p codelet-agent-loop --test bug215_effective_compaction_limits`, formatting, coverage audit, and all 2,009 feature syntax validations pass. Clippy reports only four unused-item warnings from reused test helpers. Repository-wide tag validation reports 529 pre-existing feature-file violations; this new feature has no reported tag violations.
