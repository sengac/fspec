# RPC-347 — Backend custom-model RPC + NAPI surface: TS-parity review findings

**Date:** 2026-06-20
**Reviewer:** Parallel ACDD review worker (impl-vs-TS comparison)
**Status:** WARN — wire surface correct & wired end-to-end, one behavior divergence + test gaps.

## 🔴 Critical
None. RPC trait → `FspecServiceImpl` (`rpc/src/lib.rs:1063-1106`) → `SessionManagerHandle`
(`handle_impl.rs:1075-1111`) → RPC-346 persistence. All three transports route; NAPI round-trips;
no prod `unwrap()/todo!()/panic!()`. Wire type `CustomModelDefinition` (`rpc-types/src/lib.rs:366-376`)
maps 1:1 to TS (`provider-config.ts:95-112`); `compactionThreshold` flatten/re-nest correct
(`conversions.rs:140-151`); `original_model_id` threaded through every layer.

## 🟡 Warnings (Must Fix)
1. **Non-openai add/update is a SILENT no-op in Rust, but TS THROWS.**
   TS `saveProfile` (`profile-management.ts:44-47`) `throw new Error('Profiles are only supported for OpenAI API provider')`.
   Rust `profile_sections.rs:231-233` (via `profile_object_mut`) returns `Ok(())`.
   Rule [7]/[8] only specifies no-op contract for DELETE on missing/non-openai — add/update divergence is undocumented.
   **Fix:** decide — either surface an error for non-openai add/update (TS parity) or document the no-op as intentional
   in the example map + add a scenario.
2. **Cross-transport "no-op without handle" scenario doesn't exercise websocket.**
   `rpc347_cross_transport_parity.rs:132` binds `let (embedded, _websocket)` and only calls embedded.
   Websocket `Disconnected`-guard path (`websocket.rs:447-459`) unverified. **Fix:** assert over both transports.
3. **Add/update field round-trip parity not asserted over websocket.** Parity test reads the shared in-memory stub
   (`StubSessionManagerHandle::custom_models`); camelCase wire→JSON over websocket is only proven by sessions-level
   + NAPI tests (which bypass websocket). **Fix:** add a websocket field-fidelity assertion.
