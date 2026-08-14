# Session Search — Recover Context from Previous Conversations

You are continuing work that was discussed in previous sessions. Your memory does not carry over between sessions, but the full conversation history is stored locally. **Use the session search tool to recover context before doing anything else.**

## Step 1: Identify what you're continuing

Look at the user's request. Extract keywords — work unit IDs (e.g., `RLM-001`, `MCP-001`), feature names, technical terms, or anything that identifies the topic.

## Step 2: Search for relevant sessions

Run these commands using Bash. The script is at `./scripts/session-search.sh`.

### Find recent sessions (start here if unsure what to look for)

```bash
./scripts/session-search.sh recent --count 10
```

This shows the 10 most recent sessions with their first/last user messages. Look for sessions that match the topic.

### Search by keyword

```bash
./scripts/session-search.sh search "RLM-001" --last 500
```

This searches user input history for the keyword. Use work unit IDs, feature names, or technical terms. The `--last N` flag controls how many history entries to search (default 200).

### Get full context for a topic

```bash
./scripts/session-search.sh context "DeepSearch" --last 1000 --turns 20
```

This is the most powerful command. It:
1. Finds all sessions where the keyword appears in user inputs OR session names
2. For each matching session, shows the relevant conversation turns with context
3. `--turns N` controls how many matching turns to show per session (default 5)

### Reconstruct a full conversation

```bash
./scripts/session-search.sh show <session-id>
./scripts/session-search.sh show <session-id> --user-only
```

Once you've identified a relevant session ID from the commands above, use `show` to reconstruct the full conversation. Use `--user-only` to see just what the user said (faster to scan).

## Step 3: Read attachments and artifacts

Previous sessions often produced written artifacts — architecture docs, feature files, work unit attachments. Check for these:

```bash
# Check the work unit for attachments
fspec show-work-unit <WORK-UNIT-ID>

# Read any attachments found
cat spec/attachments/<WORK-UNIT-ID>/*.md
```

## Step 4: Check the fspec board for current state

```bash
fspec board
fspec show-work-unit <WORK-UNIT-ID>
```

This tells you where work stands — what status the work unit is in, what's been specified, what's been implemented.

## Step 5: Summarize what you found

Before continuing with any work, tell the user what you recovered:
- What sessions you found and when they were from
- What was discussed and decided
- What the current state of the work is
- What the next steps appear to be

Then ask the user to confirm or correct your understanding before proceeding.

---

## Tips

- **Multiple keywords**: If one keyword doesn't find results, try related terms. The user might have said "deep search" but the assistant wrote "DeepSearch" or "RLM".
- **Session IDs are UUIDs**: They look like `7e0358a4-3395-4ee3-9a4b-62575d625b8c`. You'll see them in search results.
- **Large sessions**: Some sessions have 200+ messages. Use `context` to find the relevant parts rather than `show` which dumps everything.
- **Blob storage**: Message content over a certain size is stored in `~/.fspec/blobs/` as SHA-256 hashed files. The script resolves these automatically.
- **Tool results appear as `[tool_result]`**: The search script summarizes tool call/result pairs compactly. If you need the full tool output, use `show` on that specific session.
- **history.jsonl only has user inputs**: It records what the user typed, with timestamps and session IDs. Assistant responses are in the messages/blobs storage and are resolved by `show` and `context` commands.
