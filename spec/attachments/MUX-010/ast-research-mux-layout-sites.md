# MUX-010 AST research: the mux layout, grammar, dialog and focus sites

Run on the tree after the change: the three removed MUX-001 mutators (set_pane_list, set_pane_count, set_orientation) no longer match anywhere; the new sites are listed with their locations.

## `pub fn set_pane_list($$$ARGS) { $$$BODY }`
```
```

## `pub fn set_pane_count($$$ARGS) { $$$BODY }`
```
```

## `pub fn set_orientation($$$ARGS) { $$$BODY }`
```
```

## `pub fn shift_right(&mut self) -> bool { $$$BODY }`
```
rust/fspec-tui/src/views/multiplex/focus_moves.rs:55:5:pub fn shift_right(&mut self) -> bool {
```

## `pub fn parse_mux_command($$$ARGS) -> $RET { $$$BODY }`
```
rust/fspec-tui/src/app/mux_parser.rs:73:1:pub fn parse_mux_command(line: &str) -> Result<MuxSubcommand, MuxError> {
```

## `pub fn compute($$$ARGS) -> Geometry { $$$BODY }`
```
rust/fspec-tui/src/views/multiplex/layout_tree/geometry.rs:44:1:pub fn compute(area: Rect, layout: &MuxLayout, visible: &[bool]) -> Geometry {
```

## `pub fn neighbour($$$ARGS) -> Option<usize> { $$$BODY }`
```
rust/fspec-tui/src/views/multiplex/layout_tree/spatial.rs:28:1:pub fn neighbour(panes: &[Rect], from: usize, direction: Direction) -> Option<usize> {
```

Findings: the old flat grid (orientation + percentage splits + pane list) lived in views/multiplex/{mod,layout,splits,presets}.rs, the /mux grammar in app/mux_parser.rs, the per-pane dialog in components/mux_config_dialog{,_rows}.rs and linear focus in views/multiplex/window.rs. MUX-010 replaces them with views/multiplex/layout_tree/ (parse, geometry, fit, spatial, drag), focus_moves.rs, the layout-string dialog (mux_config_dialog{,_field,_render}.rs) and store/mux_state/migrate.rs.
