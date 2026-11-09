# BUG-283: Auxiliary views lose their navigation origin

## Scope and status

- Investigated on 2026-10-28; card started in `specifying`.
- User report: opening Providers and other screens, including screens previously opened from Agent view, then escaping returns to an empty or last-used Agent instead of the view most recently left.
- This attachment records source-level findings, reproduction paths, coverage gaps, and a proposed fix strategy. No production code or tests have been changed, and no interactive TUI reproduction or Cargo tests were run during this investigation.

## Finding: hard-coded destinations, not remembered origin

`rust/fspec-tui/src/views/navigator.rs:57` owns the persistent view objects and `active_view`, but has no general return-origin field or navigation stack.

At `rust/fspec-tui/src/views/navigator.rs:183`, opening Providers simply replaces `active_view`. At line 186, closing Providers unconditionally sets it to `ViewMode::Agent`. It does not inspect which surface opened Providers or whether an agent session exists.

Consequently, previous use from Agent is **not required** to trigger the bug. It only determines what is visible when the incorrect return destination is rendered:

- No current session: the retained Agent view is shown without a session.
- Existing current session: the retained Agent view shows that session, even if the user opened Providers from Board or Mux.

`rust/fspec-tui/src/views/navigator_render.rs:35` renders the child selected by `active_view`; it does not resolve the caller or correct an invalid destination. The provider close helper only clears status (`rust/fspec-tui/src/app/dispatch_provider_settings.rs:36`).

## Entry and exit path

1. Board `P`/`p` emits `OpenProviderSettingsView` (`rust/fspec-tui/src/views/board/keys.rs:72`).
2. Settings > Providers maps to the same action (`rust/fspec-tui/src/components/menu_bar/items.rs:94`).
3. Board/Agent menu execution redispatches the registry action (`rust/fspec-tui/src/app/dispatch_menu.rs:122`); Mux menu execution does likewise (`rust/fspec-tui/src/app/dispatch_menu_mux.rs:107`).
4. App handles the action, then `finish_dispatch_tick` invokes `Navigator::apply_action` (`rust/fspec-tui/src/app/dispatch_capability.rs:45`). Capture any future origin before the view is overwritten. The provider-open helper's comment claiming the flip happens before the helper is stale relative to the actual dispatch order.
5. Final Escape in Providers emits `CloseProviderSettingsView` through Navigator event translation (`rust/fspec-tui/src/views/navigator_events.rs:68`).
6. Navigator sets `active_view = Agent`, regardless of the entry path.

## Related screens and boundaries

| Screen/path | Current close destination | Risk |
| --- | --- | --- |
| Providers | Always Agent | Confirmed mismatch for Board/Mux callers |
| Blocklist | Always Agent | Same structural assumption; current slash-command path is agent-oriented |
| Model selector, including model commit | Agent while selector is active | Loses Board/Mux origin inherited through Providers |
| Changed Files, full-screen | Board while Files is active | Fixed destination rather than origin; assess every entry path |
| Checkpoints, full-screen | Board while Checkpoints is active | Same origin-loss pattern |
| Files/Checkpoints pane Escape inside Mux | Removes the focused pane via mux-specific routing | Different behavior; preserve it |
| Resume/history search inside Agent | Clears an Agent-owned optional mode view | Does not independently change top-level `active_view`; not the same confirmed cause |
| Dialogs, attachment picker, browser Foundation viewer | Separate overlay/external-view paths | Do not rewrite unrelated dismissal behavior without evidence |

The relevant fixed-destination arms are together at `rust/fspec-tui/src/views/navigator.rs:183`. Mux already keeps its own `pre_mux_view` for disabling the grid (`rust/fspec-tui/src/views/multiplex/mod.rs:149`); this is not a general auxiliary-view origin and must not be reused as one.

## Reproduction paths to execute

### Fresh Board with no session

1. Start on Board with no agent sessions.
2. Press `P`, or choose Settings > Providers.
3. With no filter, form, detail, or confirmation active, press Escape.
4. Expected: Board with its previous column/card selection. Source predicts: empty Agent view.

### Previously visited Agent, now on Board

1. Open an agent session and visit Providers from it, then close Providers.
2. Return to Board.
3. Open Providers again from Board, then Escape from its root list.
4. Expected: Board, the most recent caller. Source predicts: last-used Agent.

### Mux and Providers/Models switching

1. Enable Mux and focus a known pane.
2. Open Providers from the mux menu or press `P` in its Board pane.
3. Optionally Tab between Providers and Models repeatedly, then Escape.
4. Expected: the original mux layout and focus. Source predicts: single Agent view.

## Existing specifications and coverage gaps

- `spec/features/rpc054-provider-settings-dispatch.feature` and its coverage file map the scenario "Esc returns from ProviderSettingsView to AgentView" to `rust/fspec-tui/tests/provider_settings_dispatch_rpc054.rs:124`. This covers the original Agent slash-command use case, not return-to-caller behavior.
- `spec/features/rpc054-provider-settings-view.feature` covers Escape hierarchy: filters, detail views, and confirmations consume Escape before root-list close. Preserve these semantics.
- `spec/features/board-menu-bar-reorganize-zone-a-into-kanban-tools-settings-categories.feature` introduced Board access to Providers. `rust/fspec-tui/tests/menu008_menu_bar_reorg.rs:513` checks Board opening and line 568 checks opening from a mux Board pane; neither completes the Escape round trip.
- `spec/features/model-selector-navigator-wiring.feature` and `rust/fspec-tui/tests/rpc337_navigator_model_selector.rs:57` encode return-to-Agent. That test opens directly from the default Board and expects Agent: update the setup to explicitly enter Agent when preserving the agent-origin contract, and add separate Board-origin coverage.
- `spec/features/model-selector-tab-to-providers.feature` and `spec/features/model-selector-first-press-navigation.feature` cover reciprocal tab navigation. Add origin preservation across the whole Providers/Models excursion, not only tab-event emission.
- `spec/features/rpc012-board-agent-navigation.feature` establishes Board focus/selection preservation on return. Extend the same preservation expectation to auxiliary-view dismissal.
- Preserve existing Mux regression contracts in `rust/fspec-tui/tests/bug175_mux_enabled_leak.rs` and `rust/fspec-tui/tests/bug183_mux_pane_esc_close.rs`.

Historical evidence: `git blame` attributes the hard-coded provider return to `3affacdd6` (2026-05-31), while the Settings > Providers registry entry arrived with `295f54d68` (2026-10-04). This supports an original Agent-only navigation assumption remaining after additional entry surfaces were added.

## Proposed implementation direction

1. Specify caller-aware dismissal before writing tests or production code. Keep the card in `specifying` until acceptance criteria are agreed and recorded.
2. Centralize auxiliary open/close origin handling in Navigator; do not patch only the Providers Escape key or duplicate routing in menus and slash commands.
3. Capture the current live surface on entry and consume/reset it on final dismissal. A later visit must capture its own caller rather than reuse an earlier Agent origin.
4. Treat Providers/Models Tab changes as one excursion: `Board -> Providers -> Models -> Providers -> Esc` should return to Board, not trap the user cycling between the two screens. Use a shared excursion origin or explicit sibling-switch semantics rather than blindly stacking every open action.
5. Preserve existing owned views/stores, session identity, draft, scroll position, board selection, mux configuration, pane window, and focused pane. Restoring a surface must not create or resume a session.
6. Guard mismatched or late close actions so an inactive screen cannot pull the user into a stale destination. Determine a safe fallback if an originating session or layout disappears while the screen is open.
7. Keep Mux pane-local close behavior distinct from leaving a full-screen auxiliary excursion. Do not use the persisted mux enabled flag as evidence that Mux was the caller.

## Regression matrix for the implementation phase

- Board without sessions -> Providers -> Escape -> same Board; no session creation.
- Agent visit followed by Board -> Providers -> Escape -> Board, not retained Agent.
- Agent A -> Providers -> Escape -> Agent A with draft and scroll intact.
- Mux -> Providers -> Escape -> same Mux layout and pane focus.
- Board/Agent/Mux -> Providers <-> Models repeatedly -> Escape -> original surface.
- Model commit after entering through Providers preserves the excursion origin without changing the existing model-update contract.
- Full-screen Changed Files/Checkpoints return to their actual caller; pane-local Escape in Mux retains current pane-removal behavior.
- Filter/detail/form/confirmation Escape remains local until the screen actually closes.
- Reopening from a different surface replaces the old origin; duplicate opens do not create self-return loops; stale closes do not navigate.

Use 1:1 Gherkin scenarios and integration tests with the existing App/MockBackend harness. Run failing regression tests before implementation. Suggested focused commands from `rust/` after adding the tests:

```sh
cargo test --profile ci-test -p codelet-fspec-tui --test bug283_auxiliary_view_return_origin
cargo test --profile ci-test -p codelet-fspec-tui --test provider_settings_dispatch_rpc054
cargo test --profile ci-test -p codelet-fspec-tui --test rpc337_navigator_model_selector
cargo test --profile ci-test -p codelet-fspec-tui --test menu008_menu_bar_reorg
cargo test --profile ci-test -p codelet-fspec-tui --test bug183_mux_pane_esc_close
```

The proposed BUG-283 test target does not exist yet. No unscoped workspace test run is needed for this research-only task.
