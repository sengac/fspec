# Provider restore preflight API handoff

Provider owner e22f772a is adding these nonmutating APIs to `ProviderSession`:

```rust
pub fn validate_restore(&self, envelopes: &[String]) -> Result<(), ExtensionError>;
pub fn validate_committed_restore(
    &self,
    generation: u64,
    metadata: &HashMap<String, Value>,
    envelopes: &[String],
) -> Result<(), ExtensionError>;
```

`validate_restore` runs full checkpoint selection against the current snapshot.
`validate_committed_restore` checks the proposed boundary on a newly allocated,
isolated Codex history, then validates every retained envelope against that
prospective boundary. Neither method calls live restore, installs a boundary,
changes blocked readiness, writes persistence, or resets routing. A clone of
`ProviderSession` is not a sandbox because its extensions share Arcs.

CLI should call prospective validation after checking selected binding/identity
but before changing the live boundary/tokens/replay. Sessions retains ownership
of real malformed-checkpoint cold-restore tests and blocked readiness on failure.
This API complements rather than replaces those actual runtime regressions.

Current status: both public APIs and Codex implementations are now present.
`/tmp/cmpct-provider-preflight-red.log` executed the inherited shallow-clone
strategy: 0 passed, 2 failed, Cargo101 (blocked original authority and changed
live checkpoint). Only after this behavioral RED, tests switch to the new APIs
without removing their state/writer/readiness assertions. Focused GREEN is
pending in `/tmp/cmpct-provider-freshness-preflight-green.log`; do not infer its
success from its name. CLI direct message was busy/not queued; this attachment
is the durable mailbox. The API signatures above are ready for integration now.
