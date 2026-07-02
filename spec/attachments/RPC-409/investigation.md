# RPC-409 Investigation: stranded Paused chunk (LIFO-slot starvation)

## Symptom
User asked the fspec TUI to read `.env`. Backend paused correctly (Triple pause set,
tool blocked in `wait_for_pause_response`), but the RPC-406 inline allow/deny prompt
never rendered. Session hung with no UI. Reproduced 5/5 times against the live daemon.

## Evidence chain
1. Live daemon probe (`get_pause_state`) showed session `paused` with
   `PauseState{Triple, "Read: Environment files...", .env}` — backend chain healthy.
2. tarpc log showed the TUI NEVER called `get_pause_state` — chunk trigger never fired.
3. WS observation with ms timestamps: typed `[status] Paused` broadcast arrives at
   pause time (t=2.2s); `SessionStateChange{Paused}` CHUNK arrives only at t=35s —
   exactly when the pause was denied — back-to-back with `Running`.
4. Session output buffer (sampled mid-pause) already contained the Paused chunk →
   `handle_output` ran at pause time.
5. Late-subscriber discriminator: a chunks_rx subscription created mid-pause STILL
   received the Paused chunk → the broadcast delivery to that receiver happened after
   the deny (a late subscriber can never see pre-subscription sends... but tokio
   broadcast retains: actually it proved the *watcher wake-up* completed post-deny).
6. Isolated repro (no LLM/WS/rig): BackgroundSession + verbatim agent_loop pause
   closure + `pause_for_user` inside a spawned tokio task → identical 3s delay.
7. Instrumented `handle_output`: completed in 13µs on ThreadId(32) at pause time.
   The chunk was SENT instantly. The subscriber TASK did not run until deny.

## Root cause
`broadcast::Sender::send` wakes the waiting subscriber task into the CURRENT worker's
LIFO slot (tokio hot-wakeup optimisation). The pause handler then blocks that same
worker thread in `wait_for_pause_response()` (std::sync::mpsc recv). The LIFO slot is
NOT work-stealable, so the subscriber task is stranded until the worker unblocks —
i.e. until the pause is answered. The typed status subscriber escaped only by wake
ordering (displaced from the LIFO slot into the stealable local queue by the later
chunk wake) — luck, not design.

Same latent bug: `wait_for_fspec_response` (CODE-009) and `wait_for_hitl_response`
(BUG-117) — both block a worker right after emitting the chunk the UI needs to see
to unblock them.

## Fix
Wrap the blocking recv in `tokio::task::block_in_place` (guarded on
`RuntimeFlavor::MultiThread`; falls through to direct recv off-runtime / current-thread).
block_in_place hands the worker's queues (incl. LIFO slot) to another worker before
blocking. Verified in the isolated repro: Paused chunk delivered at 0.000s, while paused.

## Verification artifacts (scratch, deleted after)
- codelet/fspec-tui/examples/probe_pause.rs — live daemon pause-state probe
- codelet/fspec-tui/examples/repro_env_pause.rs — timestamped WS observation
- codelet/sessions/examples/repro_paused_chunk_delay.rs — isolated repro
