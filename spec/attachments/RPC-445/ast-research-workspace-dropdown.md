# AST research: RPC-445 workspace dropdown

## Start-workspace work-unit fanout (rpc-server)
```
rpc-server/src/server.rs:340:1:async fn work_units_fanout(

```

## Work-unit subscriber in the TUI
```
fspec-tui/src/app/bootstrap.rs:139: pub(crate) fn spawn_subscriber_tasks(&mut self)  (work_units_rx -> Action::WorkUnitsLoaded)
```

## Session chip list
```
fspec-tui/src/views/board/menu_snapshot.rs:37:1:pub fn active_menu_session_ids(store: &AgentViewStore) -> Vec<SessionId> {

```

## Header paint
```
fspec-tui/src/views/board/header.rs:35:1:pub fn paint(area: Rect, buf: &mut Buffer, store: &BoardStore, theme: &Theme) -> HeaderRows {

```

