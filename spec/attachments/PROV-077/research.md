# PROV-077 — File watcher + push refresh for custom provider directories

## Problem

Even with PROV-072's `rediscoverProviders()` and PROV-076's `/provider reload`, the TUI only learns about filesystem changes when the user explicitly asks. If a user edits `~/.fspec/providers/my-provider.json` in another window or drops a new file into the directory via their package manager, the Settings Screen and Model Selector remain stale until the user manually invokes reload.

Rust already has a `work_units_watcher` (`codelet/napi/src/work_units_watcher.rs`) that streams file changes to TS via a callback. The pattern is proven.

## Target

Add `providers_watcher` that:

1. Watches both `fspec_home()/providers/` and `<cwd>/.fspec/providers/` for JSON / RHAI changes.
2. Debounces events (200 ms) so an atomic save that produces two events (`unlink` + `create`) collapses.
3. On any change: runs `rediscoverProviders()` server-side and emits a single callback with the diff.
4. Exposes a subscribe/unsubscribe pair:

```rust
#[napi]
pub fn providers_watcher_subscribe(
    callback: ThreadsafeFunction<ProvidersChangeEvent>,
) -> Result<u32>; // returns subscription ID

#[napi]
pub fn providers_watcher_unsubscribe(id: u32) -> Result<()>;

#[napi(object)]
pub struct ProvidersChangeEvent {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<String>,
    pub invalid_added: Vec<String>,    // files that now fail to parse
    pub invalid_resolved: Vec<String>, // files that were invalid and are now valid
}
```

Implementation uses the `notify` crate (already a dep of `work_units_watcher`).

## TS integration

A new hook `useProvidersWatcher()`:

```ts
export function useProvidersWatcher(onChange?: (event: ProvidersChangeEvent) => void): void {
  useEffect(() => {
    const id = providersWatcherSubscribe(async (event) => {
      invalidateProviderRegistry();
      onChange?.(event);
    });
    return () => { providersWatcherUnsubscribe(id); };
  }, [onChange]);
}
```

Mounted inside `ProviderSettingsScreen` (so changes while the screen is open trigger auto-reload) AND at the app root so the `AgentView` provider selector / model selector stay warm.

## Toast UI

When a watcher event fires while the Settings Screen is **not** focused, surface a transient toast in the chat log:

```
ℹ Custom provider added:   coolllm-local
  /provider list to view
```

```
⚠ Custom provider became invalid:  my-provider
  /provider validate my-provider to see details
```

Toasts are additive — they never interrupt user input.

## Performance

- Debounce window: 200 ms.
- Max event frequency per second: 10 (drop silently above, log warning).
- Watch scope: only `*.json` and `*.rhai` files. Ignore hidden files, temp files (`*.swp`, `.DS_Store`), and anything larger than 256 KB (defensive).

## Test plan

- Integration: start watcher, write a file, assert callback fires within 500 ms with the file in `added`.
- Atomic save: rename-into-place should produce a single `modified` event, not `removed` + `added`.
- Invalid → valid: write malformed JSON, watch reports `invalid_added`; fix it, watch reports `invalid_resolved`.
- Unsubscribe stops events.

## Acceptance summary

- Filesystem changes under `~/.fspec/providers/` and project `.fspec/providers/` propagate to the TUI within 500 ms without any user action.
- Settings Screen auto-refreshes when open.
- Toast notifications appear for relevant changes when the screen is closed.
- Watcher is cleanly torn down on session exit.

## Dependencies

- PROV-071 (resolve both watch paths)
- PROV-072 (`rediscoverProviders` + event diff format)
- PROV-073 (registry cache invalidation)

## References

- `codelet/napi/src/work_units_watcher.rs` (reference implementation)
- `codelet/napi/Cargo.toml` (notify crate already listed)
- `src/tui/components/ProviderSettingsScreen.tsx` (mount point)
