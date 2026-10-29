# REFAC-019: proper preparatory decomposition for BOARD-024

## Authorization and boundaries

User approved “do proper refactor”, as recorded in
`/tmp/board024-resume-proper-refactor.md`. This card precedes BOARD-024 production
integration. Preserve all existing checkout work, especially Done BUG-283's
navigator/auxiliary helper, specs, tests, mappings and documents. No edits to
dispatch_work_unit_entry.rs. Shared dispatch clearance remains one later chooser
registration only. No install, restart, commit, push or security changes.

## Acceptance

- Decompose the six named oversized integration modules into meaningful source
  modules below 300 lines, without compression or dropping documentation/behavior.
- Preserve Action variant names/payloads/order and existing public imports.
- Preserve the FspecBackend trait's complete signatures, default methods,
  external implementers and Arc<dyn FspecBackend> object safety.
- Preserve embedded/WebSocket construction, subscriptions, reconnect and method
  forwarding; preserve FspecService names/signatures/order and RPC wire variants.
- Whole enum/trait/annotated impl items must be composed before derives, tarpc or
  async_trait expansion. Do not rely on unsupported include fragments inside
  enum variants or annotated trait/impl bodies.
- Build schema hashing/rerun directives must include actual declaration inputs;
  MCP coverage must resolve the real composition and preserve exact method-set
  equality, not replace source checks with a hard-coded count or weakened test.
- Establish behavioral and contract baselines before edits. New structural/input
  tracking checks fail causally before implementation. Prove the same existing
  behavior and API/wire contracts afterward, with isolated embedded/WS fixtures.

## Intended approach, subject to source inspection

Use domain-grouped whole-item composition where a single public enum/trait or
annotated impl must remain intact; use ordinary Rust submodules/inherent impls
for independent state, constructors and helpers. Keep ordered declarations as
real auditable source, resolve composition honestly in build/test tooling, and
pin public/wire contracts before moving anything. No protocol redesign.

## Validation and evidence

Feature first, contract/baseline tests second, then mechanical decomposition.
Run only scoped offline Cargo targets with ci-test/debug0 and low job counts.
Retain distinct REFAC-019 baseline, structural RED, GREEN, regression, formatting,
Clippy/check and review receipts. A green refactor must precede BOARD-024 feature
integration; full completion requires canonical Done readback for both cards.
