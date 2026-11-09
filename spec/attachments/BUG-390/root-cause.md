# BUG-390 root cause

`fspec link-coverage` step validation rejects four scenarios because their feature steps no longer match the
`@step` comments in their tests. Each time the behaviour changed, the test (assertions and `@step` comment) was
updated but the feature step was not:

| Feature / scenario | Feature step (stale) | Behaviour and test now |
| --- | --- | --- |
| agentview-multilineinput-parity / Idle placeholder text is verbatim | placeholder `Type a message... ('Shift+↑/↓' history \| ...)` | RPC-426 placeholder `Type a message... 'Ctrl+J' newline, 'Shift+↑/↓' history, 'Shift+←/→' sessions, 'Tab' turns`; test comment truncated to `"Type a message..."` |
| agentview-scrollback-wrap / End-to-end App.render top-fills scrollback ... | user line at y=1, API error at y=2 | MENU-003 menu row at y=1 and RPC-401 separator rows: user line at y=2, API error at y=4 |
| rpc013-source-shape / AgentView splits its area ... | footer string `"Enter=send  Ctrl+C=interrupt  ESC=back"` on the step line | test comment wraps the quoted string onto a second comment line, so the step line does not match |
| rpc019-multiline-input / ESC inside AgentView emits Action::BackToBoard | `Then AgentView emits Action::BackToBoard` | RPC-051: AgentView emits `Action::AgentEscPressed` (the App routes it to BackToBoard) |

Because validation fails, these scenarios' coverage could not be remapped when BUG-377 shifted `views/agent.rs` by
one line; their `agent.rs` impl mappings are one line stale until this card relinks them.

Fix: bring each feature step (and the rpc019 scenario name) in line with the current behaviour, and make the test
`@step` comments state the full step text on one line.

## Also fixed here

The validating gate showed agentview-multilineinput-parity had three scenarios with no impl mappings and every test
range drifted (e.g. "New modules stay under the 300-LoC source-shape ceiling" pointed at 246-261, the idle placeholder
test). All ten ranges now point at their test functions, and the three scenarios map to `render_input_transition`,
`components/spinner.rs` (`DOTS_INTERVAL_MS`, `current_frame_glyph`, `paint_spinner_line`) and the two module files.
