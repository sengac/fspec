# BUG-348 design
- One submission path: `start_turn` retains the submission, clears pending input, optionally echoes, sets Running and sends the PromptInput.
- `TurnEcho::{Echo, AlreadyEchoed}` (private enum in messaging.rs) names the choice instead of a bare bool.
- `send_input` (fresh input) echoes. `send_steers_as_turn` (pub(super), used only by `continue_with_inbox`) does not, because each steer was echoed by the `(true, Steer)` arm of `submit_input`.
