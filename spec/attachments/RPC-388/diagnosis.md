# RPC-388 — Tool-call argument header parity (`extractToolArgsDisplay`)

## Symptom

The Rust TUI renders a tool call as a header line:

```
● ToolName(argsDisplay)
```

…but `argsDisplay` is built differently from the TypeScript reference. The Rust
port shows **only one argument value** per tool and applies **no length cap**,
so headers show less detail than the reference yet can grow arbitrarily long.

## Reference behaviour (the contract)

`src/tui/utils/chunkProcessor.ts:130-205` — `extractToolArgsDisplay(toolName, inputObj)`.

Three branches, plus a per-value 100-character cap:

### Branch 1 — Edit/Write family → `file_path` only

`toolNameLower` ∈ {`edit`, `replace`, `write`, `write_file`}:
- If `inputObj.file_path` present → return `String(file_path)`.
- Else → return `''`.
- (Content is intentionally omitted from the header because it is shown later as
  a diff.)

### Branch 2 — has `command` or `action_type` key → command-first, then rest

```ts
const commandKey = inputObj.command ? 'command'
                 : inputObj.action_type ? 'action_type'
                 : null;
```
If `commandKey` set:
- `command = String(inputObj[commandKey])`
- `otherEntries` = all entries except `commandKey`
- If no other entries → return `command`
- Else → return `` `${command}, { ${parts.join(', ')} }` `` where each part is
  rendered by the **value formatter** below.

### Branch 3 — default → all params as object

```ts
const entries = Object.entries(inputObj);
if (entries.length === 0) return '';
return `{ ${parts.join(', ')} }`;
```
Each part rendered by the value formatter.

### Value formatter (used by branches 2 and 3)

For each `[key, value]`:
- `typeof value === 'string'`:
  `displayValue = value.length > 100 ? value.slice(0,100) + '...' : value`
  → `` `${key}: '${displayValue}'` `` (single-quoted)
- `value === null || value === undefined`:
  → `` `${key}: ${value}` `` (unquoted `null` / `undefined`)
- otherwise (object/array/number/bool):
  `jsonStr = JSON.stringify(value)`;
  `displayValue = jsonStr.length > 100 ? jsonStr.slice(0,100) + '...' : jsonStr`
  → `` `${key}: ${displayValue}` `` (unquoted)

**The only numeric threshold is 100 characters**, applied per value, tail-clipped
with a literal `...`.

## Current Rust behaviour (the gap)

`codelet/fspec-tui/src/store/agent_view/tool_args.rs:19-52` —
`extract_tool_args_display(tool_name, input_json)`:

- Selects a **single key** by tool name (`Bash`→`command`, `Read`/`Write`/`Edit`
  →`file_path`, `Grep`/`Glob`→`pattern`, `Fspec`→`command`, `WebSearch`→`query`,
  `WebFetch`→`url`, `Task`→`description`), or the **first JSON value** for
  unknown tools.
- Returns that value **uncapped** (`value_to_inline`, lines 56-61).
- **No** Edit/Write `file_path`-only special-case (Write/Edit map to `file_path`
  by coincidence but the branch semantics differ — e.g. it does not return `''`
  when `file_path` is absent, and `replace`/`write_file` aren't handled).
- **No** `command, { ...rest }` multi-param form.
- **No** default `{ key: 'val', ... }` all-params form.
- **No** 100-char cap.

## Required fix

Rewrite `extract_tool_args_display` to mirror the TS three-branch algorithm and
the value formatter, including the 100-char cap. Key parity points:

1. **Match on lowercased tool name** for the Edit/Write family
   (`edit`/`replace`/`write`/`write_file`) → `file_path` only (empty string when
   absent).
2. **`command`/`action_type` detection** by key presence (not tool name) →
   command first, then remaining params as `{ k: v, ... }`.
3. **Default branch** → all params as `{ k: v, ... }`.
4. **Value formatter** with single-quoting for strings, bare `null`/`undefined`,
   `serde_json` compact form for everything else, each capped at 100 chars with
   a `...` suffix.
5. **JSON-parse-failure fallback** → return the raw input verbatim (existing
   behaviour, keep it).

### Parity caveats to encode as tests

- Object key **ordering**: TS uses `Object.entries` (insertion order). Rust must
  preserve input JSON object order → use `serde_json` with the `preserve_order`
  feature OR rely on the existing `Map` ordering already used by the current
  code's `unknown_tool_returns_first_value` test (which asserts insertion order).
  Confirm `serde_json::Map` here preserves order (the existing test depends on
  it) and assert ordering explicitly in new tests.
- `null`/`undefined`: JSON has no `undefined`; only `null` is reachable from a
  parsed value → render as bare `null`.
- The 100-char cap counts **characters**; for ASCII this equals bytes. Use a
  char-safe slice (`chars().take(100)`) to avoid panicking on multi-byte UTF-8
  boundaries (TS `slice` is UTF-16-code-unit based; for the in-scope ASCII
  examples the observable output is identical — document this and prefer
  char-boundary-safe slicing in Rust).

## Behaviour table (target)

| Tool | input | output |
|------|-------|--------|
| `Edit` | `{"file_path":"/a.rs","old_string":"x","new_string":"y"}` | `/a.rs` |
| `Write` | `{"content":"..."}` (no file_path) | `` (empty) |
| `Bash` | `{"command":"ls -la","timeout":5000}` | `ls -la, { timeout: 5000 }` |
| `Fspec` | `{"command":"show","args":"{...}"}` | `show, { args: '{...}' }` |
| `WebSearch` | `{"action_type":"search","query":"hi"}` | `search, { query: 'hi' }` |
| `Grep` | `{"pattern":"foo","glob":"*.rs"}` | `{ pattern: 'foo', glob: '*.rs' }` |
| any | `{"q":"<120 chars>"}` | `{ q: '<100 chars>...' }` |
| any | `not-json` (parse fail) | `not-json` |

> Note: `Bash`/`Grep`/`Read` etc. no longer hard-code a single key — they fall
> into branch 2 (command/action_type) or branch 3 (default) like the TS code.
> This means `Bash` with only `{"command":"ls"}` → `ls` (branch 2, no other
> entries), and `Read {"file_path":"/etc/hosts"}` → `{ file_path: '/etc/hosts' }`
> (branch 3). **Existing tests that assert `Read → /etc/hosts` and
> `Bash → ls -la` (bare) WILL change** and must be updated to the new parity
> output — verify each against the TS function semantics.

## Files

| File | Role |
|------|------|
| `src/tui/utils/chunkProcessor.ts:130-205` | TS reference (contract) |
| `codelet/fspec-tui/src/store/agent_view/tool_args.rs:19-61` | **Function to rewrite** |
| `codelet/fspec-tui/src/store/agent_view/chunk_processor.rs:114` | Caller (`handle_tool_call`) — unchanged |

## Scope / Non-Goals

- **In scope:** Full three-branch port + 100-char value cap + updated tests.
- **Out of scope:** The header is still subject to Ink/ratatui width clipping at
  render time — that is unrelated and unchanged. Tool-result *body* collapsing is
  RPC-389.
