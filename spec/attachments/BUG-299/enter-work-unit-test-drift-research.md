# Research: TUI tests still expect automatic session creation from `EnterWorkUnit`

Reported by the BOARD-024 implementation run (2026-10-30). Not caused by BOARD-024: the
affected tests dispatch `Action::EnterWorkUnit` directly (not through the new board Enter
chooser) and none of the failing paths touch BOARD-024 code.

## Failing tests
| Target | Failing | Symptom |
|---|---|---|
| `app_bootstrap_rpc009` | 3 | time out waiting for chunk-forwarding actions |
| `app_with_mock_backend_repl` | 2 | expect a session to be created and receive chunks |
| `agentview_esc_exit_confirmation_rpc098` | 1 | expects an active agent session after entry |
| `mux004_mux_config_dialog` | `committing_with_enabled_on…` | same session-creation expectation |

Run with: `cargo test --profile ci-test -p codelet-fspec-tui --test <target>`.

## Analysis
The committed BUG-280/BUG-281 intent-safe entry changed `EnterWorkUnit` for a card with **no
associated session**: it now opens the session-creation dialog (normal vs isolated) instead
of silently creating a session. These older tests still assume the pre-BUG-280 behaviour, so
they wait for chunks from a session that is never created.

## Suggested fix
Update the fixtures to the current contract: either pre-associate a session with the card,
or drive the creation dialog (confirm "normal session") before asserting chunk forwarding /
Esc confirmation / mux commit. Re-verify each scenario's feature text still describes the
intended behaviour; where it describes auto-creation, update the spec (BUG-280/281 are the
source of truth).
