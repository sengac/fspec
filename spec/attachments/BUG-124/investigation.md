# BUG-124: Shift+Arrow Navigation Skips Sessions

## Symptom

When a supervisor session spawns more than one subordinate agent, pressing
`Shift+Left` / `Shift+Right` does not cycle through every session. With one
extra subordinate it works; with two or more, navigation gets stuck in a
small loop and most subordinates are unreachable.

## Root Cause: `codelet/napi/src/navigation.rs:40-69`

```rust
pub fn build_navigation_list(
    sessions: &IndexMap<Uuid, Arc<BackgroundSession>>,
    chain_of_command: &ChainOfCommand,
) -> Vec<Uuid> {
    let mut result = Vec::new();

    for session_id in sessions.keys() {
        let has_subordinates = !chain_of_command.get_subordinates(*session_id).is_empty();

        if has_subordinates {
            continue;                    // skip the supervisor in the outer loop
        }

        result.push(*session_id);        // push the leaf session

        let supervisors = chain_of_command.get_supervisors(*session_id);
        for supervisor_id in supervisors {
            if sessions.contains_key(&supervisor_id) {
                result.push(supervisor_id);   // BUG: pushes supervisor once per child
            }
        }
    }

    result
}
```

For each subordinate, the function appends that subordinate's supervisors.
A single supervisor with N children therefore appears N times in the
resulting `Vec<Uuid>`.

### Worked Example: Supervisor + 5 Subordinates

`IndexMap` insertion order: `[supervisor, s1, s2, s3, s4, s5]`

| Iter | session_id | has_subordinates? | Action                                | `result` after step                                                                   |
| ---- | ---------- | ----------------- | ------------------------------------- | ------------------------------------------------------------------------------------- |
| 1    | supervisor | true              | skip                                  | `[]`                                                                                  |
| 2    | s1         | false             | push s1, push supervisor              | `[s1, supervisor]`                                                                    |
| 3    | s2         | false             | push s2, push supervisor              | `[s1, supervisor, s2, supervisor]`                                                    |
| 4    | s3         | false             | push s3, push supervisor              | `[s1, supervisor, s2, supervisor, s3, supervisor]`                                    |
| 5    | s4         | false             | push s4, push supervisor              | `[s1, supervisor, s2, supervisor, s3, supervisor, s4, supervisor]`                    |
| 6    | s5         | false             | push s5, push supervisor              | `[s1, supervisor, s2, supervisor, s3, supervisor, s4, supervisor, s5, supervisor]`    |

The supervisor appears five times.

### Why Cycling Then Breaks

`get_next_target` line 92:

```rust
let current_idx = nav_list.iter().position(|&id| id == active_id);
```

`Vec::position()` always returns the **first** match. From the supervisor:

- Find supervisor → idx `1` (right after s1)
- Next → idx `2` → **s2**

s1 is skipped. From s2:

- Find s2 → idx `2`
- Next → idx `3` → **supervisor**

From supervisor again, position() still returns `1`, so next is **s2**.
The user is now in an infinite `supervisor ↔ s2` loop and s1, s3, s4, s5
are unreachable. Backwards navigation fails symmetrically.

## Why "More Than One Extra Agent" Triggers It

With exactly one subordinate the list is `[s1, supervisor]` — only one
occurrence of supervisor, no duplication, navigation works. The bug
appears the moment a second subordinate is added, because that is when
the supervisor first gets pushed twice.

## The Fix (Option B — flat insertion order)

```rust
pub fn build_navigation_list(
    sessions: &IndexMap<Uuid, Arc<BackgroundSession>>,
    _chain_of_command: &ChainOfCommand,
) -> Vec<Uuid> {
    sessions.keys().copied().collect()
}
```

For the worked example this produces `[supervisor, s1, s2, s3, s4, s5]` —
pure spawn order, every session reachable, deterministic.

The `chain_of_command` parameter is preserved (still passed in) so
existing call sites and tests do not need to change their signatures.
The underscore prefix marks it as intentionally unused.

## Test Coverage Gap

Existing tests in `navigation.rs:154-301` construct nav_lists **manually**
with unique UUIDs (e.g. line 208: `vec![session_a, supervisor_w1, supervisor_w2, session_b]`).
**No existing test calls `build_navigation_list()` itself with a real
`ChainOfCommand` containing one supervisor and multiple subordinates.**
That is why this slipped through review.

### Required Regression Tests

1. **Single supervisor, multiple subordinates — no duplicates**

   ```rust
   #[test]
   fn test_build_nav_list_one_supervisor_many_subordinates() {
       // sessions: [supervisor, s1, s2, s3]
       // chain: supervisor → [s1, s2, s3]
       let nav = build_navigation_list(&sessions, &chain);
       let unique: HashSet<_> = nav.iter().collect();
       assert_eq!(unique.len(), nav.len(), "no duplicates allowed");
       assert_eq!(nav.len(), 4);
   }
   ```

2. **Insertion order preserved**

   ```rust
   #[test]
   fn test_build_nav_list_preserves_insertion_order() {
       // Insert in known order, assert nav matches keys() order
       assert_eq!(nav, sessions.keys().copied().collect::<Vec<_>>());
   }
   ```

3. **End-to-end cycling reaches every session**

   ```rust
   #[test]
   fn test_shift_right_visits_every_session_exactly_once() {
       // Spawn supervisor + 5 subordinates
       // From board, repeatedly call get_next_target()
       // Assert every session UUID is visited exactly once before reaching CreateDialog
   }
   ```

## Files Touched by the Fix

| File                                   | Change                                                          |
| -------------------------------------- | --------------------------------------------------------------- |
| `codelet/napi/src/navigation.rs:40-69` | Replace `build_navigation_list` body with flat IndexMap walk    |
| `codelet/napi/src/navigation.rs:154+`  | Add the three regression tests above                            |

## Files Confirmed Unaffected (no changes needed)

| File                                          | Why it stays                                                      |
| --------------------------------------------- | ----------------------------------------------------------------- |
| `src/tui/components/AgentView.tsx:4532-4554`  | Shift detection logic is correct                                  |
| `src/tui/components/MultiLineInput.tsx:201-219` | Propagation rule (TUI-049) is correct                           |
| `src/tui/hooks/useSessionNavigation.ts:48-79` | Hook delegation is correct                                        |
| `src/tui/utils/sessionNavigation.ts:32-70`    | NAPI wrappers are correct                                         |
| `codelet/napi/src/navigation.rs:77-152`       | `get_next_target`/`get_prev_target` work correctly on a clean list |

## Related Tickets

- **VIEWNV-001** — original implementation that introduced the bug
- **TUI-049** — input propagation behavior (unrelated, already correct)
- **FIX-7** — many-to-many `ChainOfCommand` refactor (unrelated, already correct)
