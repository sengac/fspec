# Isolated Sessions & Worktrees

When you start a new agent, you can choose between **Normal** and **Isolated** mode:

- **Normal** — Agent works directly in your project directory
- **Isolated** — Agent works in a git worktree (separate directory, same repository)

## Why Isolated Sessions?

Isolated sessions provide safe experimentation without risking your main codebase:

- **Parallel development** — Multiple agents can work on different features simultaneously
- **Safe experimentation** — Changes are contained in a separate worktree
- **Easy rollback** — Discard changes without affecting the main project
- **Clean merges** — Apply changes back only when you're satisfied

## How It Works

1. **Create an isolated session** — Press `/` and select "Isolated" mode
2. **Work normally** — The agent sees the worktree as its project root
3. **Review changes** — All files are modified in the isolated worktree
4. **Merge or discard** — When finished, merge changes back or discard them

## Merging Changes

To merge isolated session changes back to the main project:

```
/merge-worktree
```

This command:

1. **Checks for conflicts** — Detects if files changed in both session and main project
2. **Applies changes** — Copies modifications, additions, and deletions to main worktree
3. **Shows summary** — Displays files modified, added, and deleted
4. **Closes session** — Returns you to the board view

## Conflict Handling

If conflicts are detected:

- The merge is **not applied**
- Conflict details are shown in the chat
- The session remains **active** so you can resolve conflicts
- After resolving, run `/merge-worktree` again

## Discarding Changes

If you decide not to keep isolated changes:

```
/sessions
```

This opens the session manager where you can:

- View all isolated sessions
- Inspect changes before deciding
- Discard sessions you don't want

Discarding removes the worktree and all uncommitted changes—**no changes are applied to the main project**.

## When to Use Isolated Sessions

| Use Case | Recommended Mode |
|----------|------------------|
| Quick bug fix | Normal |
| Experimental feature | Isolated |
| Multiple agents working simultaneously | Isolated |
| Refactoring with uncertain outcomes | Isolated |
| Production hotfix | Normal |
| Code review/analysis | Normal |

## Technical Details

- Worktrees are created in `.fspec/worktrees/<session-id>/`
- Each worktree shares the same git history as the main repository
- Sessions are tracked in `~/.fspec/git-sessions/`
- Orphaned worktrees (from crashed sessions) are cleaned up automatically
