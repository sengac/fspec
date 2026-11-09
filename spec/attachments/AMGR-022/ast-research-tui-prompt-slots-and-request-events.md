# AST research: TUI prompt slots, prompt caches and request events (AMGR-022, 2026-11-08)

fspec astgrep over rust/fspec-tui/src and rust/sessions/src; each line is file:line:col:match.

## `$S.pause_state_for($$$A)`
```
rust/fspec-tui/src/views/agent/input_area.rs:82:47:store.pause_state_for(s)
rust/fspec-tui/src/views/agent/input_area.rs:187:58:store.pause_state_for(s)

```

## `$S.hitl_prompt_for($$$A)`
```
rust/fspec-tui/src/app/dispatch_pause_hitl.rs:190:32:self
rust/fspec-tui/src/app/dispatch_hitl_prompt.rs:152:12:self.agent_view_store.hitl_prompt_for(&session_id)
rust/fspec-tui/src/app/dispatch_exec_stdin.rs:86:12:self
rust/fspec-tui/src/views/agent/input_area.rs:73:47:store.hitl_prompt_for(s)
rust/fspec-tui/src/views/agent/input_area.rs:138:58:store.hitl_prompt_for(s)

```

## `$S.pause_route_for($$$A)`
```
rust/fspec-tui/src/app/dispatch_pause_hitl.rs:217:26:self

```

## `self.last_pause = $V`
```
rust/fspec-tui/src/views/agent/input_area.rs:161:13:self.last_pause = None
rust/fspec-tui/src/views/agent/input_area.rs:174:21:self.last_pause = None
rust/fspec-tui/src/views/agent/input_area.rs:190:13:self.last_pause = Some((session.clone(), state.kind))
rust/fspec-tui/src/views/agent/input_area.rs:193:9:self.last_pause = None
rust/fspec-tui/src/views/agent/input_area.rs:284:9:self.last_pause = None

```

## `self.last_hitl = $V`
```
rust/fspec-tui/src/views/agent/input_area.rs:150:13:self.last_hitl = Some((
rust/fspec-tui/src/views/agent/input_area.rs:164:9:self.last_hitl = None
rust/fspec-tui/src/views/agent/input_area.rs:285:9:self.last_hitl = None

```

## `StreamChunk::InteractionRequestChanged { $$$F }`
```
No matches found
```

## `$S.handle_pause_chunk($$$A)`
```
rust/fspec-tui/src/app/dispatch_pause_route.rs:75:9:self.handle_pause_chunk(session_id.clone())
rust/fspec-tui/src/app/dispatch_pause_route.rs:134:17:self.handle_pause_chunk(sid.clone())

```

## `build_chips($$$A)`
```
rust/fspec-tui/src/views/board/menu_snapshot.rs:108:17:build_chips(&inputs, clock_ms)
rust/fspec-tui/src/components/menu_bar/chips.rs:157:21:build_chips(&[input(1, SessionStatus::Running, None)], 0)
rust/fspec-tui/src/components/menu_bar/chips.rs:160:21:build_chips(&[input(1, SessionStatus::Running, None)], 80)
rust/fspec-tui/src/components/menu_bar/chips.rs:162:21:build_chips(&[input(1, SessionStatus::Running, None)], 240)
rust/fspec-tui/src/components/menu_bar/chips.rs:170:21:build_chips(
rust/fspec-tui/src/components/menu_bar/chips.rs:192:21:build_chips(&[input(1, SessionStatus::Idle, None)], 0)
rust/fspec-tui/src/components/menu_bar/chips.rs:204:21:build_chips(
rust/fspec-tui/src/components/menu_bar/chips.rs:219:21:build_chips(

```

## `self.announce($$$A)` (sessions)
```
rust/sessions/src/background_session/request_routing.rs:370:9:self.announce(route.clone(), RequestStatus::Pending)
rust/sessions/src/background_session/interactions.rs:176:13:self.announce(raised.route(id), RequestStatus::Cancelled)
rust/sessions/src/background_session/interactions.rs:189:13:self.announce(raised.route(id), RequestStatus::Pending)
rust/sessions/src/background_session/interactions.rs:302:9:self.announce(
rust/sessions/src/background_session/interactions.rs:338:9:self.announce(

```
