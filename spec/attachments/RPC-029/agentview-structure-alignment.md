# AgentView Structure Alignment — Rust ratatui → TS Ink Original

**Card:** RPC-029
**Parent:** RPC-002 (rust-frontend epic)
**Related (already done):** RPC-018 (header/footer widgets), RPC-019 (multi-line input + scrollback), RPC-027 (dialog theming)

This document captures the visible structural deltas between the canonical TS Ink AgentView (`src/tui/components/AgentView.tsx`) and its Rust ratatui port (`codelet/fspec-tui/src/views/agent.rs`) as observed in:

- `/Users/rquast/Desktop/agentview-rust.png`   — current Rust output
- `/Users/rquast/Desktop/agentview-typescript.png` — canonical TS output

The goal of RPC-029 is to make the Rust render path **structurally identical** to the TS version. This is not a re-skin — it is a re-layout. The visible chrome (header, role banner, scrollback, footer, input) must occupy the same screen slots, in the same order, with the same borders (or lack thereof), and the same per-segment colors.

> **CRITICAL FINDING from deep-search of `src/tui/components/AgentView.tsx`:** The TS AgentView **does NOT use `ConversationInputArea`**. It inlines its own input area as a bare `<Box paddingX={1}>` with a green `> ` prefix and `<InputTransition>` — there is **NO border at all** on the AgentView input (not even a top rule). The `ConversationInputArea` component exists but is used by other conversation views, not AgentView.

---

## 1. Side-by-side render summary

### TS Ink (target) — AgentView.tsx lines 5212–5460

```
┌────────────────────────────────────────────────────────────────────┐ ← bg #333333 (header row)
│ #1 (RPC-029: implementing): claude-sonnet-4 [R] [V] [200k] [T:H]   │   paddingX=1
│                              45.2 tok/s  tokens: 1234↓ 567↑ [45%]  │
├────────────────────────────────────────────────────────────────────┤ ← transparent (terminal bg)
│ Role: Senior Rust engineer focused on TUI ergonomics               │   (RoleBanner, only when role set)
├────────────────────────────────────────────────────────────────────┤
│ <scrollback content — NO BORDER, NO PADDING, NO BACKGROUND>        │
│ (VirtualList rendered directly inside a flexGrow=1 flexBasis=0 Box)│
│                                                                    │
│ ...                                                                │
├────────────────────────────────────────────────────────────────────┤ ← bg #333333 (footer row)
│                                          ~/projects/fspec [⎇ main] │   paddingX=1, right-aligned only
├────────────────────────────────────────────────────────────────────┤
│ > <input area — NO BORDER, just paddingX=1, green `> ` + Input...> │
└────────────────────────────────────────────────────────────────────┘
```

Layout order, top→bottom (verbatim from `AgentView.tsx` L5212–5460):

1. **`<SessionHeader …/>`** — 1 row, **bg `#333333`**, `paddingX={1}`
2. **`<RoleBanner roleText={…}/>`** — 1 row when role set; component returns `null` when not (zero height); **transparent bg**, no padding
3. **`<Box flexGrow={1} flexBasis={0}>` wrapping `<VirtualList>`** — **NO border, NO padding, NO background**
4. **`<SessionFooter sessionId={…}/>`** — 1 row, **bg `#333333`**, `paddingX={1}`, content RIGHT-aligned only
5. **Input area** — bare `<Box paddingX={1}>` with `<Text color="green">&gt; </Text>` and `<Box flexGrow={1}><InputTransition…/></Box>` — **NO border**

Overlays (slash palette, file popup, modals, dialogs) render conditionally AFTER the input slot.

### Rust ratatui (current — wrong) — `agent.rs` L214–273

```
┌────────────────────────────────────────────────────────────────────┐ ← transparent bg (NO #333333!)
│ #1: claude-sonnet-4 [R] [V] [192k]       tokens: 0↓ 0↑ [0%]        │   no padding, white-on-default
├────────────────────────────────────────────────────────────────────┤
│ ┌─ Agent — rpc-no-session-manager ─────────────────────────────┐   │ ← scrollback wrapped in Block::ALL with title (WRONG)
│ │ <scrollback content>                                         │   │
│ └──────────────────────────────────────────────────────────────┘   │
│ ┌──────────────────────────────────────────────────────────────┐   │ ← input wrapped in Block::ALL (WRONG)
│ │> Type a message...                                           │   │
│ └──────────────────────────────────────────────────────────────┘   │
│ Enter=send  Ctrl+C=interrupt  ESC=back   ~/projects/fspec [⌥ main] │ ← footer BELOW input (WRONG ORDER!)
└────────────────────────────────────────────────────────────────────┘   ← transparent bg, NO padding, ⌥ glyph (wrong)
```

Current Rust constraint order (`agent.rs` L214–222):

```rust
.constraints([
    Constraint::Length(1),            // 0 header
    Constraint::Length(role_height),  // 1 role banner
    Constraint::Min(0),               // 2 scrollback (Block::ALL) ← border WRONG
    Constraint::Length(input_height), // 3 input (Block::ALL)       ← border WRONG
    Constraint::Length(1),            // 4 footer                   ← slot WRONG (must move up)
])
```

---


## 2. Major issues (prioritized — break these first)

The following table is the single source of truth for what RPC-029 must fix. Each row maps to one acceptance scenario.

| # | Issue | Severity | Fix scope |
|---|-------|---------:|-----------|
| 1 | Scrollback wrapped in `Block::ALL` with `" Agent — {sid} "` title — TS has NO border, NO title | **CRITICAL** | `agent.rs` L249–259: delete the Block; render `ScrollbackList` directly into `scrollback_area` |
| 2 | Input wrapped in `Block::ALL` (full 4-sided border) — TS AgentView has NO border on input | **CRITICAL** | `agent.rs` L261–265: delete the Block; render `MultiLineInput::render_with_prompt` directly into `input_area`; reduce `input_height` from `visible_rows + 2` to `visible_rows` |
| 3 | Footer rendered BELOW input — TS renders footer ABOVE input | **CRITICAL** | Reorder `agent.rs` L214–225 constraint array so footer is slot 3, input is slot 4 |
| 4 | Header missing work-unit prefix `(RPC-029: implementing)` — TS shows it between session # and model name | **HIGH** | `header.rs`: add `work_unit_id` / `work_unit_status` fields; `agent.rs` L237 wires from `store.current_work_unit_id()` / `store.current_work_unit_status()` |
| 5 | Header missing `#333333` background — TS paints full row dark grey | **HIGH** | `header.rs`: add row-fill helper before span paint |
| 6 | Footer missing `#333333` background — TS paints full row dark grey | **HIGH** | `footer.rs`: same row-fill helper |
| 7 | Header missing `paddingX={1}` — TS pads both sides | **MEDIUM** | `header.rs::paint_two_columns`: offset start by `+1`, reduce width by `2` |
| 8 | Footer missing `paddingX={1}` — TS pads both sides | **MEDIUM** | `footer.rs::paint_two_columns`: same offset |
| 9 | Header colors are flat (white left, darkgray right) — TS uses per-badge colors: cyan.bold prefix+model, magenta `[R]`, blue `[V]`, dim `[Nk]`, red.bold `[DEBUG]`, cyan `[SELECT]`, yellow `[T:*]`, green `[ISOLATED]`, dim tokens, fill-pct color | **HIGH** | `header.rs`: rewrite `build_left_text` / `build_right_text` to return `Line<'static>` with multi-span styling; update `paint_two_columns` to accept `Line` |
| 10 | Header missing badges: `[ISOLATED]` (green), `[DEBUG]` (red.bold), `[SELECT]` (cyan), `tokensPerSecond` ("X tok/s" magenta), reasoning tokens (🧠), `compactionReduction` ("COMPACTED N%") | **MEDIUM** | `header.rs`: extend `SessionHeader` struct with fields for `is_isolated`, `is_debug`, `is_select_mode`, `tokens_per_second`, `reasoning_tokens`, `compaction_reduction` (note: some may be deferred to follow-up cards — minimum required for visual parity is `[DEBUG]` + `[ISOLATED]`) |
| 11 | Footer has left-side hints (`Enter=send  Ctrl+C=interrupt  ESC=back`) — TS footer left is EMPTY | **HIGH** | `footer.rs`: delete `build_left_hints`; render only the right column |
| 12 | Footer branch glyph is `⌥` (U+2325 OPTION KEY) — TS uses `⎇` (U+2387 ALTERNATIVE KEY) | **HIGH** | `footer.rs::build_right_text` L60: swap glyph |
| 13 | Footer paints full path in `Cyan` — TS dims the cwd and only colors `[⎇ branch]` in cyan | **MEDIUM** | `footer.rs::paint_two_columns`: split right text into two spans (dim cwd + cyan branch) |
| 14 | Header `#N` separator is `: ` after the number — TS keeps `#N` without a trailing colon, the colon appears only after the work-unit segment (e.g. `#1 (RPC-029: implementing): claude...`) | **MEDIUM** | `header.rs::build_left_text` L59–63: drop the `": "` after the number, let work-unit segment own the colon |
| 15 | Right side missing `tokens per second` (magenta, while loading) and reasoning-tokens (🧠) indicator | **LOW** | `header.rs::build_right_text`: extend struct + format string when those fields are populated |
| 16 | Inline pause/HITL/compaction indicators inside `InputTransition` (TS) are not implemented in Rust `MultiLineInput` | **DEFERRED** | Out of scope for RPC-029 — surface as follow-up story (RPC-???) |
| 17 | Loading→input character-by-character animation (TS `InputTransition`) is not implemented in Rust | **DEFERRED** | Out of scope — follow-up story |

Issues 1, 2, 3 are the source of the visible "boxed-in" look in the Rust screenshot. Issues 5, 6, 11, 12 are the visible "header/footer don't look the same" surface deltas. Issue 4 is the visible "session prefix missing context" delta. Issues 9, 10, 13 are the colour-fidelity polish.

---

## 3. Reference: exact TS structure (verbatim)

### 3.1 AgentView return wrapper (`AgentView.tsx` L5212–5213, L5419–5460)

```jsx
return (
  <Box flexDirection="column" flexGrow={1}>
    <SessionHeader … />
    <RoleBanner roleText={…} />
    <Box flexGrow={1} flexBasis={0}>
      <VirtualList items={conversationLines} … />
    </Box>
    <SessionFooter sessionId={currentSessionId} />
    <Box paddingX={1}>
      <Text color="green">&gt; </Text>
      <Box flexGrow={1}>
        <InputTransition … />
      </Box>
    </Box>
    {/* …overlays… */}
  </Box>
);
```

**Outer `Box`:** no border, no background, no padding — just `flexDirection="column" flexGrow={1}`. Comment on L5210–5211 confirms: *"Removed outer border to maximize usable space and reduce rendering overhead."*

### 3.2 SessionHeader (`SessionHeader.tsx` L179–204)

```jsx
<Box flexDirection="column" width="100%">
  <Box height={1} width="100%" flexDirection="row"
       backgroundColor="#333333" paddingLeft={1} paddingRight={1}>
    <Box flexGrow={1} flexShrink={1} minWidth={0}>
      <Text wrap="truncate-end">{leftContent}</Text>
    </Box>
    <Text> </Text>
    <Box flexShrink={0} flexDirection="row">
      {isLoading && tokensPerSecond !== null && (
        <Text color="magenta">{tokensPerSecond.toFixed(1)} tok/s  </Text>
      )}
      <Text dimColor>tokens: {inputTokens}↓ {outputTokens}↑{reasoningTokens > 0 ? ` ${reasoningTokens}🧠` : ''}  </Text>
      <Text color={getContextFillColor(contextFillPercentage)}>{percentText}</Text>
    </Box>
  </Box>
</Box>
```

**Left content build (`SessionHeader.tsx` L141–177):**

```ts
const sessionPrefix = sessionNumber !== undefined ? `#${sessionNumber}` : '';
const workUnitText  = workUnitId ? ` (${workUnitId}${workUnitStatus ? `: ${workUnitStatus}` : ''})` : '';
const separator     = (sessionPrefix || workUnitId) ? ': ' : '';

leftContent += chalk.cyan.bold(`${sessionPrefix}${workUnitText}${separator}${modelId || 'Loading...'}`);
if (isIsolated)        leftContent += chalk.green(' [ISOLATED]');
if (hasReasoning)      leftContent += chalk.magenta(' [R]');
if (hasVision)         leftContent += chalk.blue(' [V]');
if (badgeValue > 0)    leftContent += chalk.dim(` [${formatContextWindow(badgeValue)}]`);
if (isDebugEnabled)    leftContent += chalk.red.bold(' [DEBUG]');
if (isSelectMode)      leftContent += chalk.cyan(' [SELECT]');
if (thinkingLabel)     leftContent += chalk.yellow(` ${thinkingLabel}`);
```

Format with all flags on: `#1 (RPC-029: implementing): claude-sonnet-4 [ISOLATED] [R] [V] [200k] [DEBUG] [SELECT] [T:High]`

**Percent format (`SessionHeader.tsx` L129–132):**
```ts
const percentText = compactionReduction !== null
  ? `[${formatPercentage(contextFillPercentage)}%: COMPACTED ${formatPercentage(Math.abs(compactionReduction))}%]`
  : `[${formatPercentage(contextFillPercentage)}%]`;
```

### 3.3 SessionFooter (`SessionFooter.tsx` L67–77)

```jsx
<Box height={1} width="100%" flexDirection="row"
     backgroundColor="#333333" paddingLeft={1} paddingRight={1}>
  <Box flexGrow={1} flexShrink={1} minWidth={0} />
  <Box flexShrink={0}>
    <Text wrap="truncate-end">{rightContent}</Text>
  </Box>
</Box>
```

**Right content build (L58–65):**
```ts
let rightContent = chalk.dim(footerState.displayPath);
if (footerState.git.isGitRepo) {
  rightContent += ' ' + chalk.cyan(formatBranchDisplay(footerState.git.branch));
}
```

**Branch glyph (L41–44):** `[⎇ ${branchName}]` — uses **U+2387 ALTERNATIVE KEY (`⎇`)**.

**Footer LEFT side is empty** — it's just a `<Box flexGrow={1}>` spacer with no children. The hints `Enter=send  Ctrl+C=interrupt  ESC=back` that currently live in the Rust footer **do not exist in the TS footer at all**.

### 3.4 RoleBanner (`RoleBanner.tsx`)

```jsx
<Box height={1} width="100%" flexShrink={0} overflow="hidden">
  <Text wrap="truncate-end">{chalk.cyan('Role:')} {chalk.dim(singleLineRole)}</Text>
</Box>
```

- 1 row, **transparent bg**, no padding
- `"Role:"` in `chalk.cyan`, role text in `chalk.dim`
- Multi-line collapsed to single line via `.replace(/\s+/g, ' ').trim()`
- Returns `null` when `roleText` is null/empty (zero-height collapse)

Rust `role_banner.rs` already matches this. ✓

### 3.5 Input area (`AgentView.tsx` L5422–5460)

```jsx
<Box paddingX={1}>
  <Text color="green">&gt; </Text>
  <Box flexGrow={1}>
    <InputTransition
      isLoading={displayIsLoading}
      isPaused={displayIsPaused}
      pauseInfo={displayPauseInfo}
      triplePauseSelection={triplePauseSelection}
      hitlRequest={displayHitlRequest}
      …
    />
  </Box>
</Box>
```

**No border. No background. Only `paddingX={1}`.** Green `> ` prompt, then `InputTransition` (which internally renders either `MultiLineInput`, a loading/compacting indicator, a pause/HITL panel, or an action prompt — all using the same single-line height when idle).

---

## 4. Reference: exact Rust current state (verbatim)

### 4.1 `agent.rs::render_with_store` L186–274

```rust
let input_height = self.input.visible_rows().saturating_add(2);  // +2 for Block::ALL borders ← WRONG

let role_height: u16 = sid.as_ref().and_then(|s| store.role_for(s)).map(|_| 1).unwrap_or(0);
let split = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(1),            // header
        Constraint::Length(role_height),  // role banner
        Constraint::Min(0),               // scrollback
        Constraint::Length(input_height), // input
        Constraint::Length(1),            // footer  ← WRONG SLOT
    ])
    .split(area);
let (header_area, role_area, scrollback_area, input_area, footer_area) =
    (split[0], split[1], split[2], split[3], split[4]);

// Header
SessionHeader { session_index: store.session_index(), model, thinking, tokens }
    .render(header_area, buf);

// Role
if role_height > 0 { /* paint RoleBanner */ }

// Scrollback — WRAPPED IN Block::ALL WITH TITLE
let title = match &sid { Some(s) => format!(" Agent — {} ", s.value), None => " Agent ".to_string() };
let scrollback_block = Block::default().borders(Borders::ALL).title(title);
let inner_scrollback = scrollback_block.inner(scrollback_area);
scrollback_block.render(scrollback_area, buf);
self.last_scrollback_viewport = inner_scrollback.height;
if let Some(ctx) = store.current_session_context_mut() {
    ctx.scrollback.render_count_visited(inner_scrollback, buf);
}

// Input — WRAPPED IN Block::ALL (full 4-sided)
let input_block = Block::default().borders(Borders::ALL);
let inner_input = input_block.inner(input_area);
input_block.render(input_area, buf);
self.input.render_with_prompt(inner_input, buf, INPUT_PLACEHOLDER_HINT);

// Footer — BELOW INPUT
SessionFooter { workspace: store.workspace() }.render(footer_area, buf);
```

### 4.2 `header.rs` L42–155

`SessionHeader` struct fields: `session_index`, `model`, `thinking`, `tokens` — **missing** `work_unit_id`, `work_unit_status`, `is_isolated`, `is_debug`, `is_select_mode`, `tokens_per_second`, `reasoning_tokens`, `compaction_reduction`.

`build_left_text` returns a plain `String` — there is no per-span styling. `paint_two_columns` accepts `&str` and renders **exactly two single-color spans**: the entire left side as one `Span::styled(..., Color::White)` (L130, L133) and the entire right side as one `Span::styled(..., Color::DarkGray)` (L131, L145). There is no multi-span composition anywhere in the file.

> ⚠️ **STALE COMMENT — DO NOT BE MISLED:** `header.rs:128` says *"paint the left in dim grey so the right side dominates when scanning for token deltas during long bursts"*. The actual code on the next line is `Color::White` — the opposite of what the comment claims. When rewriting `paint_two_columns`, delete or rewrite this comment.

No background paint on the row.

### 4.3 `footer.rs` L34–126

```rust
fn build_left_hints() -> String {
    super::PLACEHOLDER_FOOTER_HINTS.to_string()   // ← MUST DELETE for parity
}

fn build_right_text(workspace: &WorkspaceInfo) -> String {
    let mut out = shorten_with_home(&workspace.cwd);
    if let Some(branch) = workspace.git_branch.as_deref() {
        out.push_str(" [⌥ ");                     // ← MUST CHANGE to ⎇
        out.push_str(branch);
        out.push(']');
    }
    out
}
```

The branch glyph is `⌥` (U+2325 OPTION KEY, UTF-8 bytes `e2 8c a5`) — confirmed by `xxd` on `footer.rs:60`. The TS source uses `⎇` (U+2387 ALTERNATIVE KEY SYMBOL, UTF-8 bytes `e2 8e 87`) at `SessionFooter.tsx:43`. The Rust module doc-comment on L7–8 explicitly calls this out as a *"per-RPC-018 architecture-note swap of `⎇` → `⌥`"* — RPC-029 reverses that decision because the canonical screenshots show `⎇`.

`paint_two_columns` (L94–126) renders **exactly two single-color spans**: the entire left side as one `Span::styled(..., Color::DarkGray)` (L100, L102) and the entire right side as one `Span::styled(..., Color::Cyan)` (L101, L115). The right paint is guarded by `if right_len > 0` (L112) — the header has no such guard.

No background paint on the row.

---


## 5. Migration plan (suggested ordering)

### Phase A — Structural (issues 1, 2, 3, 11, 12)

Visible "wow, that looks right now" delta after this phase. Snapshot tests in this phase should be enough to catch regressions in the other phases.

1. **Reorder layout slots** in `agent.rs::render_with_store`:
   ```rust
   .constraints([
       Constraint::Length(1),            // 0 header
       Constraint::Length(role_height),  // 1 role banner
       Constraint::Min(0),               // 2 scrollback
       Constraint::Length(1),            // 3 footer    ← moved up
       Constraint::Length(input_height), // 4 input
   ])
   let (header_area, role_area, scrollback_area, footer_area, input_area) =
       (split[0], split[1], split[2], split[3], split[4]);
   ```
2. **Delete the scrollback Block** — render `ScrollbackList` directly into `scrollback_area`; update `self.last_scrollback_viewport = scrollback_area.height;`.
3. **Delete the input Block** — render `MultiLineInput::render_with_prompt(input_area, buf, INPUT_PLACEHOLDER_HINT)` directly; reduce `input_height` from `visible_rows + 2` to `visible_rows` (or `+1` if we keep a 1-row gap above the prompt — but TS has zero gap, so `+0`).
4. **Pad the input area on left/right** to mirror `paddingX={1}`. Either:
   - Carve `input_area` with `Layout::horizontal([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])` and render the prompt+input into the middle slot, OR
   - Have `MultiLineInput::render_with_prompt` paint starting at `area.x + 1` and clamp width to `area.width - 2`.
5. **Footer: remove `build_left_hints`** — `SessionFooter::render` paints only the right column.
6. **Footer: swap `⌥` → `⎇`** in `footer.rs::build_right_text` L60.
7. **Update affected snapshot tests** under `codelet/fspec-tui/src/views/agent/snapshots/`. Regenerate after visual confirmation.

### Phase B — Theming (issues 5, 6, 7, 8, 13)

8. **Add a `paint_row_bg(area, buf, color)` helper** in `views/agent/mod.rs` that walks every cell of `area` and calls `buf[(x, y)].set_bg(color)`. Call it **before** spans paint, so spans inherit the bg via ratatui's cell-merge.
9. **Header: paint `Color::Rgb(0x33, 0x33, 0x33)` row bg + horizontal padding of 1.** `paint_two_columns` shifts `area.x` by `+1` and shrinks `area.width` by `2` when building the inner paint rect.
10. **Footer: same row bg + horizontal padding.**
11. **Footer: split right text** into two spans — `Span::styled(cwd, Style::default().fg(Color::DarkGray))` and `Span::styled(branch_suffix, Style::default().fg(Color::Cyan))`.

### Phase C — Header semantics (issues 4, 9, 10, 14, 15)

12. **Extend `SessionHeader` struct:**
    ```rust
    pub struct SessionHeader<'a> {
        pub session_index: (usize, usize),
        pub model: Option<&'a ModelInfo>,
        pub thinking: ThinkingLevel,
        pub tokens: TokenState,
        pub work_unit_id: Option<&'a str>,
        pub work_unit_status: Option<&'a str>,
        pub is_isolated: bool,
        pub is_debug_enabled: bool,
        pub is_select_mode: bool,
        pub tokens_per_second: Option<f32>,
        pub reasoning_tokens: u64,
        pub compaction_reduction: Option<i32>,
        pub is_loading: bool,
    }
    ```
13. **Rewrite `build_left_text` → `build_left_line(…) -> Line<'static>`** returning multi-span:
    ```rust
    fn build_left_line(...) -> Line<'static> {
        let mut spans = Vec::new();
        let prefix = format!("#{}", index.0);
        let wu = match (work_unit_id, work_unit_status) {
            (Some(id), Some(status)) => format!(" ({}: {})", id, status),
            (Some(id), None)         => format!(" ({})", id),
            _                        => String::new(),
        };
        let sep = if !prefix.is_empty() || work_unit_id.is_some() { ": " } else { "" };
        let model_name = model.map(|m| m.display_name.as_str()).unwrap_or("Loading...");
        spans.push(Span::styled(
            format!("{prefix}{wu}{sep}{model_name}"),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
        if is_isolated      { spans.push(Span::styled(" [ISOLATED]".into(), Style::default().fg(Color::Green))); }
        if model.map_or(false, |m| m.supports_reasoning) { spans.push(Span::styled(" [R]".into(), Style::default().fg(Color::Magenta))); }
        if model.map_or(false, |m| m.supports_vision)    { spans.push(Span::styled(" [V]".into(), Style::default().fg(Color::Blue))); }
        if let Some(m) = model { if m.context_window > 0 {
            spans.push(Span::styled(format!(" [{}]", format_context_window(m.context_window)),
                                    Style::default().fg(Color::DarkGray)));
        }}
        if is_debug_enabled { spans.push(Span::styled(" [DEBUG]".into(), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))); }
        if is_select_mode   { spans.push(Span::styled(" [SELECT]".into(), Style::default().fg(Color::Cyan))); }
        if let Some(label) = thinking_label(thinking) {
            spans.push(Span::styled(format!(" [T:{}]", label), Style::default().fg(Color::Yellow)));
        }
        Line::from(spans)
    }
    ```
14. **Rewrite `build_right_text` → `build_right_line(…) -> Line<'static>`:**
    ```rust
    fn build_right_line(...) -> Line<'static> {
        let mut spans = Vec::new();
        if is_loading {
            if let Some(tps) = tokens_per_second {
                spans.push(Span::styled(format!("{:.1} tok/s  ", tps), Style::default().fg(Color::Magenta)));
            }
        }
        let reasoning_part = if reasoning_tokens > 0 { format!(" {}🧠", reasoning_tokens) } else { String::new() };
        spans.push(Span::styled(
            format!("tokens: {}↓ {}↑{}  ", tokens.input_tokens, tokens.output_tokens, reasoning_part),
            Style::default().fg(Color::DarkGray),
        ));
        let pct_text = match compaction_reduction {
            Some(r) => format!("[{}%: COMPACTED {}%]", tokens.context_fill_pct, r.abs()),
            None    => format!("[{}%]", tokens.context_fill_pct),
        };
        spans.push(Span::styled(pct_text, Style::default().fg(context_fill_color(tokens.context_fill_pct))));
        Line::from(spans)
    }
    ```
15. **Implement `context_fill_color(pct: u8) -> Color`** mirroring TS `getContextFillColor` from `src/tui/utils/sessionHeaderUtils.ts` L37–42 — note it has **FOUR thresholds, not three**:
    ```rust
    fn context_fill_color(pct: u8) -> Color {
        if pct < 50      { Color::Green }
        else if pct < 70 { Color::Yellow }
        else if pct < 85 { Color::Magenta }   // ← TS includes magenta band, easy to miss
        else             { Color::Red }
    }
    ```
16. **Update `paint_two_columns`** to accept `Line<'static>` for both halves; replace `Paragraph::new(Line::from(Span::styled(...)))` calls with the multi-span lines built above. The width-budgeting logic stays — truncate the left line if total span width exceeds the budget.
17. **Wire work-unit data** in `agent.rs` L237–238:
    ```rust
    SessionHeader {
        session_index: store.session_index(),
        model,
        thinking,
        tokens,
        work_unit_id: store.current_work_unit_id(),
        work_unit_status: store.current_work_unit_status(),
        is_isolated: false,           // TODO: requires SessionContext::is_isolated
        is_debug_enabled: false,      // TODO: requires AppState::debug
        is_select_mode: false,        // TODO: requires AgentView::is_select_mode
        tokens_per_second: None,      // TODO: from token-rate accumulator
        reasoning_tokens: 0,          // TODO: from TokenState
        compaction_reduction: None,   // TODO: from CompactionResultUpdate
        is_loading: false,            // TODO: from session status
    }.render(header_area, buf);
    ```
18. **Wiring for `is_debug_enabled`, `is_isolated`, `is_select_mode`, `tokens_per_second`, `reasoning_tokens`, `compaction_reduction`** — note that only `is_debug_enabled` and `is_isolated` are required for visual parity with the screenshots. The others can be threaded in but default to `false`/`0`/`None` and the badge simply won't render when the data isn't available. If routing all of them through `AgentViewStore` is too much scope, **`is_isolated` and `is_debug_enabled` are the minimum** and the rest can be deferred to a follow-up card.

### Phase D — Tests + feature files

19. **Update `header.rs` unit tests** to assert the new spans + colors. Add cases for:
    - `build_left_line_includes_work_unit_prefix_when_set`
    - `build_left_line_omits_work_unit_when_unset`
    - `build_left_line_paints_cyan_bold_prefix_then_per_badge_colors`
    - `build_right_line_omits_tokens_per_second_when_not_loading`
20. **Update `footer.rs` unit tests:**
    - `build_right_line_uses_alternative_key_glyph` (asserts `'\u{2387}'`)
    - `render_paints_dark_grey_row_background`
    - `render_paints_nothing_on_left_side`
21. **Add `agent.rs` integration test** verifying layout order: scan the buffer top→bottom and assert the footer's workspace string appears at a row above the input prompt row.
22. **Regenerate snapshot fixtures** under `codelet/fspec-tui/src/views/agent/snapshots/`.
23. **Update feature files:**
    - `spec/features/rpc018-agent-chrome.feature` — scenarios asserting old chrome shape
    - `spec/features/rpc019-multiline-input.feature` — scenarios asserting input has 4-sided border
    - Add new scenarios under a new `spec/features/rpc029-agent-structure-alignment.feature` covering the 8 acceptance criteria below.

---

## 6. Acceptance criteria for RPC-029

Given the Rust AgentView renders into a Rect with a current session, model info, work unit context, and a git workspace:

1. **No scrollback border or title.** Snapshot test: no `┌` / `└` / `─` / `│` characters anywhere inside the scrollback slot's boundary, and the literal string `" Agent — "` does not appear in the buffer.
2. **No input border.** Snapshot test: no `┌` / `└` / `─` / `│` characters around the input slot. The green `> ` prompt sits flush at `(input_area.x + 1, input_area.y)` (accounting for the `paddingX=1` left pad).
3. **Footer above input.** Scan buffer rows top→bottom: the row containing the workspace cwd appears strictly above the row containing the green `>` prompt.
4. **Work-unit prefix in header.** When `store.current_work_unit_id()` returns `Some("RPC-029")` and `current_work_unit_status()` returns `Some("implementing")`, the header's left text contains `(RPC-029: implementing)` between `#N` and `: <model>`.
5. **Per-segment header colors.** Test on the built `Line` asserts span styles: cyan+bold for prefix+work-unit+model run, magenta for `[R]`, blue for `[V]`, dark-grey for `[Nk]`, red+bold for `[DEBUG]`, cyan for `[SELECT]`, yellow for `[T:*]`, green for `[ISOLATED]`.
6. **Dark grey row backgrounds.** Every cell of `header_area` and `footer_area` has `bg == Color::Rgb(0x33, 0x33, 0x33)` after render.
7. **Footer branch glyph is `⎇` (U+2387).** Footer right text matches regex `\[⎇ [^\]]+\]`. The Unicode escape `\u{2387}` is present in `footer.rs::build_right_text`.
8. **Footer left side is empty.** Scanning `footer_area` row from `area.x + 1` to the start of the right-aligned workspace text yields only background cells (no glyphs). The strings `Enter=send`, `Ctrl+C`, `ESC=back` do not appear anywhere in `footer_area`.
9. **Horizontal padding of 1 on header and footer.** The first column of `header_area` and `footer_area` contains only background (no glyph), and the last column too.
10. **Footer cwd is dim, branch suffix is cyan.** The span at the cwd position has `fg == Color::DarkGray`; the span at the `[⎇ branch]` position has `fg == Color::Cyan`.

---

## 7. Files touched (estimate)

| File                                                              | Change                                                                                              |
|-------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------|
| `codelet/fspec-tui/src/views/agent.rs`                            | Layout reorder, remove scrollback + input Blocks, pad input area, recompute `input_height` and `last_scrollback_viewport` |
| `codelet/fspec-tui/src/views/agent/header.rs`                     | Extend struct, multi-span `Line` rendering, row bg + horizontal padding, update tests               |
| `codelet/fspec-tui/src/views/agent/footer.rs`                     | Delete `build_left_hints`, swap `⌥` → `⎇`, multi-span right line, row bg + padding, update tests    |
| `codelet/fspec-tui/src/views/agent/mod.rs` (new)                  | Add `paint_row_bg(area, buf, color)` helper                                                          |
| `codelet/fspec-tui/src/store/agent_view.rs`                       | Possibly extend if `is_isolated` / `is_debug_enabled` etc. need new store fields                    |
| `codelet/fspec-tui/src/views/agent/snapshots/*`                   | Regenerate after layout change                                                                       |
| `spec/features/rpc018-agent-chrome.feature`                       | Update scenarios asserting old chrome shape                                                          |
| `spec/features/rpc019-multiline-input.feature`                    | Update scenarios asserting input has 4-sided border                                                  |
| `spec/features/rpc029-agent-structure-alignment.feature` (new)    | New file covering the 10 acceptance criteria above                                                   |

---

## 8. Out of scope (deferred to follow-up cards)

### 8.1 Early-return overlay modes — NOT touched by RPC-029

The TS AgentView has **7 early-return modes** that bypass the normal layout entirely:

| Mode | `AgentView.tsx` line | Rust equivalent |
|---|---:|---|
| Error overlay (`error && !currentSessionId`) | 4862 | not implemented |
| `showProviderSelector` | 4909 | not implemented |
| `showModelSelector` | 4962 | not implemented |
| `showSettingsTab` | 4980 | not implemented |
| `isSearchMode` | 5003 | `search_view: Some(_)` early-return in `agent.rs:198–201` |
| `isResumeMode` | 5053 | `resume_view: Some(_)` early-return in `agent.rs:194–197` |
| `isBlocklistMode` | 5194 | not implemented |

**RPC-029 only touches the "normal layout" code path** — the one entered when none of these conditions are active. The Rust port already handles `search_view` and `resume_view`; the other five overlays remain as separate-story future work.

`showTurnModal` is NOT an early-return — it's a conditional overlay rendered alongside the normal layout (line 5486, after the input area), so it's unaffected by the layout reorder.

### 8.2 Other deferred items

- **Inline pause/HITL/compaction indicators inside the input** (TS `InputTransition` substitution behaviour: HITL question UI, `⏸` pause panels for confirm/triple/continue, action-prompt `✓` messages, `ThinkingIndicator` loading/compacting dots). The Rust `MultiLineInput` only renders the textarea — surfacing those states would more than triple this card's scope.
- **Loading→input character-by-character animation** (TS `InputTransition` `animationPhase` hiding/showing partial text).
- **Reasoning-tokens 🧠 indicator + tokens-per-second `X tok/s` magenta segment** can ship in this card if the source fields are easy to thread through `AgentViewStore`; otherwise defer.
- **Mouse-region dispatch** (`mouse_dispatch.rs`) — the input/scrollback Rect math may need tiny adjustments if any handler reads `inner_input` vs `input_area`, but the click→action routing logic itself does not change.
- **Popup overlays** (slash command, file search) — they paint into the full Rect, not the inner slots, so layout reordering does not affect them.
- **Dialog theming** — covered by RPC-027 and complete.

---

## 9. Estimated effort

**8 story points** (4–6 hrs):

- Phase A (structural — layout reorder + border removal + padding): 45–60 min
- Phase B (theming — bg + padding + footer split spans): 45 min
- Phase C (header semantics — struct extension + multi-span lines + wire data): 90–120 min
- Phase D (tests + feature files + snapshot regeneration): 90 min
- Manual cargo run + screenshot diff against TS canonical + adjustments: 45 min

Up from the original 5-point estimate because the deep-search revealed the input has no border at all (not just no 4-sided border), the footer color split needs two spans, the header needs ~5 more badge codepaths than first identified, and several store-side fields need plumbing for the new header inputs.

---

## Appendix — Verified facts (DeepSearch double-pass)

Every claim below was triple-checked against the source. File:line citations are authoritative.

| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | TS AgentView render order: `SessionHeader → RoleBanner → VirtualList Box → SessionFooter → inline input Box` | ✅ | `AgentView.tsx:5213–5449` |
| 2 | `SessionHeader` uses `backgroundColor="#333333"` and `paddingLeft=1 paddingRight=1` | ✅ | `SessionHeader.tsx:181` |
| 3 | `SessionFooter` uses `backgroundColor="#333333"` and `paddingLeft=1 paddingRight=1` (BOTH early-return AND main branches) | ✅ | `SessionFooter.tsx:52, 68` |
| 4 | TS branch glyph is U+2387 `⎇` (UTF-8 bytes `e2 8e 87`) | ✅ | `SessionFooter.tsx:43` |
| 5 | TS header prefix concat produces `"#1 (RPC-029: implementing): claude-sonnet-4"` | ✅ | `SessionHeader.tsx:141–151` (walk-through in §3.2) |
| 6 | TS header chalk colours per badge | ✅ | `SessionHeader.tsx:151–177` |
| 7 | TS footer LEFT side is completely empty (`<Box flexGrow={1} flexShrink={1} minWidth={0} />` self-closing spacer) | ✅ | `SessionFooter.tsx:70` |
| 8 | TS footer right content = `chalk.dim(displayPath)` + ` ` + `chalk.cyan(formatBranchDisplay(...))` — two separate chalk calls concatenated | ✅ | `SessionFooter.tsx:59, 64` |
| 9 | `getContextFillColor` has **FOUR** thresholds: `<50 green`, `<70 yellow`, `<85 magenta`, `≥85 red` | ✅ | `src/tui/utils/sessionHeaderUtils.ts:37–42` |
| 10 | TS AgentView does NOT import `ConversationInputArea`; inlines bare `<Box paddingX={1}>` with NO border | ✅ | `AgentView.tsx:5423–5425` (no border attrs), no `ConversationInputArea` reference in file |
| 11 | `FullScreenWrapper` adds NO border and NO padding — just sets `width`, `height`, `flexDirection="column"` | ✅ | `FullScreenWrapper.tsx:47–54` |
| 12 | `RoleBanner` returns `null` when `roleText` is falsy (zero-height collapse) | ✅ | `RoleBanner.tsx:29–32` |
| 13 | NO inline indicators rendered between `<SessionFooter />` and the input area `<Box>` — they abut directly | ✅ | `AgentView.tsx:5420 → 5423` (only a comment between them) |
| 14 | `INPUT_PLACEHOLDER_HINT` is identical in Rust and TS (`"Type a message... ('Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn)"`) | ✅ | `agent.rs:73–74` ≡ `AgentView.tsx:5442` |
| 15 | Rust currently wraps scrollback in `Block::default().borders(Borders::ALL).title(" Agent — <sid> ")` | ✅ | `agent.rs:249–255` |
| 16 | Rust footer glyph is U+2325 `⌥` (UTF-8 bytes `e2 8c a5`) — a deliberate divergence from TS per RPC-018 module doc comment, which RPC-029 reverses | ✅ | `footer.rs:60, 156`; module doc L7–8 |
| 17 | Rust `header.rs::paint_two_columns` paints two single-color spans (left=`Color::White`, right=`Color::DarkGray`) | ✅ | `header.rs:130–131, 133, 145` — note the comment on L128 ("paint the left in dim grey") is STALE/incorrect |
| 18 | Rust `footer.rs::paint_two_columns` paints two single-color spans (left=`Color::DarkGray`, right=`Color::Cyan`); right is guarded by `if right_len > 0` | ✅ | `footer.rs:100–102, 112, 115` |
| 19 | TS input area Box has ONLY `paddingX={1}` — no `flexShrink`, no `flexDirection`, no `borderTop`, no other props | ✅ | `AgentView.tsx:5423–5425` |
| 20 | TS AgentView has 7 early-return modes (error, providerSelector, modelSelector, settingsTab, search, resume, blocklist); RPC-029 targets ONLY the normal-layout codepath | ✅ | `AgentView.tsx:4862, 4909, 4962, 4980, 5003, 5053, 5194` |
