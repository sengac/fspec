# RPC-064 — `/search` slash command end-to-end (UI view)

**Parent:** RPC-030 · **Phase:** 6.4 · **Estimate:** 2 pts · **Depends on:** RPC-063

## Goal

Polish the `/search` command path for full TS parity. The trait is already wired (`persistence_search_history` on `FspecService` / `FspecBackend`) and `SearchHistoryView` exists (RPC-026). This card ensures search-as-you-type, result highlighting, and Enter-to-insert all match the TS frontend.

## Current state

`dispatch.rs` line 195 dispatches `Action::OpenSearchView` on `Ctrl+R`. The view exists. `Action::SearchHistory(query)` calls `backend.persistence_search_history(query)` and populates results.

What may be missing (audit during this card):

1. **Search-as-you-type debouncing**: every keystroke shouldn't fire a full RPC. Debounce at ~150ms.
2. **Result highlighting**: matched substring is bolded/coloured in the result list.
3. **Enter-to-insert**: selected result inserts into the current input via `Action::InsertIntoInput(text)`.
4. **Esc to dismiss without inserting**: dispatches `Action::CloseSearchView`.
5. **Empty-query state**: shows recent history (last 50) or a hint, never crashes.

## TS reference

`AgentView.tsx` line 2721: `setIsSearchMode(true)`, `setSearchQuery('')`, `setSearchResults([])`, `setSearchResultIndex(0)`. The search component re-renders on each query update with the filtered results.

## Work

### Audit `SearchHistoryView` and fix gaps

```rust
// codelet/fspec-tui/src/views/search/mod.rs

pub struct SearchHistoryView {
    query: String,
    results: Vec<HistoryMatch>,
    selected: usize,
    search_in_flight: Option<JoinHandle<()>>,
    last_query_sent: String,
    debounce_handle: Option<JoinHandle<()>>,
}
```

### Debounce keystroke → RPC

```rust
fn on_query_change(&mut self, new_query: String) {
    self.query = new_query.clone();

    // Cancel previous debounce.
    if let Some(h) = self.debounce_handle.take() { h.abort(); }

    let sender = self.dispatch_sender.clone();
    self.debounce_handle = Some(tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(150)).await;
        let _ = sender.send(Action::SearchHistory(new_query));
    }));
}
```

### Render highlighting

In the row renderer, find the case-insensitive match position(s) and render those substrings in `Style::default().add_modifier(Modifier::BOLD)`.

### Action wiring

```rust
Action::SearchHistory(query) => {
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        let results = backend.persistence_search_history(query.clone()).await.unwrap_or_default();
        let _ = sender.send(Action::SearchResultsLoaded { query, results });
    });
}

Action::SearchResultsLoaded { query, results } => {
    if let Some(view) = self.navigator.search_view_mut() {
        if view.query == query {  // ignore stale results
            view.results = results;
            view.selected = 0;
        }
    }
}

Action::SearchResultSelected => {
    let Some(view) = self.navigator.search_view() else { return };
    let Some(result) = view.results.get(view.selected) else { return };
    let text = result.text.clone();
    self.emit_action(Action::CloseSearchView);
    self.emit_action(Action::InsertIntoInput(text));
}
```

## Acceptance criteria

1. `/search` (palette) AND `Ctrl+R` (chord) both open SearchHistoryView.
2. Typing debounces at 150ms then fires `persistence_search_history`.
3. Results show matched substring in bold.
4. j/k or arrows navigate the result list.
5. Enter inserts the selected result into the live input and closes the view.
6. Esc closes the view without inserting.
7. Stale results (slow RPC returning after newer query) are discarded.
8. Integration test in `codelet/fspec-tui/tests/search_view.rs` covers debounce + insert + dismiss.

## Out of scope

- Cross-session search filtering (already supported by `persistence_search_history`).
- Search history persistence (separate from prompt history; not in scope).
