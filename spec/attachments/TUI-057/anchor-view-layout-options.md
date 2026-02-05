# Anchor View Layout Options

All layouts use split-screen with anchor list on left and scrollable preview (VirtualList) on right.

Future features to keep in mind:
- Delete anchors
- Load compacted conversation segments back into context
- Create custom anchors around loaded content
- Navigate to specific turns in conversation history

---

## Layout 1: Simple 40/60 Split

Minimal chrome, maximum content. Left pane shows anchor list, right pane shows selected anchor's turn content in a scrollable VirtualList.

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ 📍 Anchors (4)                                                          ESC  │
├───────────────────────────┬──────────────────────────────────────────────────┤
│ ANCHORS                   │ TURN 14 DETAILS                                ▲ │
│                           │                                                █ │
│ ▸ ✅ TaskCompletion       │ User:                                          █ │
│   Turn 14 • 0.91          │ Can you analyze the compaction logs?           │ │
│                           │                                                │ │
│   🔧 ErrorResolution      │ Assistant:                                     │ │
│   Turn 8 • 0.85           │ I found the issue with anchor detection.       │ │
│                           │ The JSON parsing was failing because...        │ │
│   📍 UserCheckpoint       │                                                │ │
│   Turn 5 • 0.80           │ Tool Calls:                                    │ │
│                           │ • Read: src/compactor.rs                       │ │
│   🏁 FeatureMilestone     │ • Edit: src/compactor.rs (45-62)              │ │
│   Turn 2 • 0.75           │ • Bash: cargo test                             │ │
│                           │                                                │ │
│                           │ Files Modified:                                │ │
│                           │ • src/compactor.rs (+15, -3)                   │ │
│                           │                                                │ │
│                           │ Status: ✅ Success                             ▼ │
├───────────────────────────┴──────────────────────────────────────────────────┤
│ ↑↓ Navigate │ Enter Expand │ Esc Close                                       │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Pros:** Clean, simple, most screen real estate for content
**Cons:** No obvious place for future action buttons

---

## Layout 2: 30/70 Split with Action Bar

Narrower anchor list with action hints. Footer shows context-sensitive actions.

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ 📍 Anchors                                                              ESC  │
├──────────────────┬───────────────────────────────────────────────────────────┤
│ ANCHORS (4)      │ TURN 14                                                 ▲ │
│                  │ ─────────────────────────────────────────────────────── █ │
│ ▸ ✅ Task (0.91) │                                                         █ │
│   Turn 14        │ 👤 User:                                                │ │
│                  │ Can you analyze the compaction logs and figure out      │ │
│   🔧 Error(0.85) │ why the anchor detection is failing?                    │ │
│   Turn 8         │                                                         │ │
│                  │ 🤖 Assistant:                                           │ │
│   📍 Check(0.80) │ I found the issue with anchor detection. The JSON       │ │
│   Turn 5         │ parsing was failing because LLMs wrap their response    │ │
│                  │ in markdown code blocks like ```json ... ```            │ │
│   🏁 Mile (0.75) │                                                         │ │
│   Turn 2         │ I've added extract_json_from_response() to handle this. │ │
│                  │                                                         │ │
│                  │ 🔧 Tool Calls (3):                                      │ │
│                  │ ├─ Read: codelet/core/src/compaction/anchor.rs          │ │
│                  │ ├─ Edit: codelet/core/src/compaction/anchor.rs          │ │
│                  │ └─ Bash: cargo test --test anchor                       │ │
│                  │                                                         ▼ │
├──────────────────┴───────────────────────────────────────────────────────────┤
│ ↑↓ Select │ Enter Expand │ D Delete │ L Load into Context │ Esc Close        │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Pros:** Action bar shows available commands, narrower list = more preview space
**Cons:** Anchor names truncated, less room for anchor metadata

---

## Layout 3: 40/60 with Collapsible Sections in Preview

Preview pane has collapsible sections for different content types. User can expand/collapse with Tab.

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ 📍 Anchor Points - Session: debugging-compaction                        ESC  │
├───────────────────────────┬──────────────────────────────────────────────────┤
│ 4 anchors                 │ ✅ TaskCompletion @ Turn 14              [0.91] ▲ │
│                           │ "Fixed JSON parsing for anchor detection"       █ │
│ ▸ ✅ TaskCompletion       │ ──────────────────────────────────────────────── │ │
│   T14 • 04:35 • 0.91      │                                                  │ │
│   Fixed JSON parsing...   │ ▼ User Message                                   │ │
│                           │   Can you analyze the compaction logs and        │ │
│   🔧 ErrorResolution      │   figure out why the anchor detection is         │ │
│   T8 • 03:22 • 0.85       │   failing?                                       │ │
│   Resolved test failure   │                                                  │ │
│                           │ ▼ Assistant Response                             │ │
│   📍 UserCheckpoint       │   I found the issue with anchor detection...     │ │
│   T5 • 02:45 • 0.80       │   [truncated - press Enter to expand]            │ │
│   Checkpoint before ref   │                                                  │ │
│                           │ ▶ Tool Calls (3)                                 │ │
│   🏁 FeatureMilestone     │                                                  │ │
│   T2 • 01:30 • 0.75       │ ▶ File Modifications (1)                         │ │
│   Initial anchor system   │                                                  │ │
│                           │ Status: ✅ Success                               ▼ │
├───────────────────────────┴──────────────────────────────────────────────────┤
│ ↑↓ Navigate │ Tab Expand Section │ Enter Full View │ D Delete │ Esc Close    │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Pros:** User controls detail level, can focus on what they need
**Cons:** More complex UI, extra key bindings to learn

---

## Layout 4: Three-Pane with Compacted History Browser

Left: anchors, Middle: selected anchor preview, Right: available compacted segments to load.
This layout is designed for the future "load compacted content" feature.

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ 📍 Anchor Management                                        [Tab] Switch Pane│
├─────────────────┬──────────────────────────────┬─────────────────────────────┤
│ ANCHORS (4)     │ TURN 14 PREVIEW            ▲ │ COMPACTED HISTORY         ▲ │
│                 │                            █ │                           │ │
│ ▸ ✅ Task       │ User:                      │ │ 📦 Segment 1 (T1-T10)     │ │
│   T14 • 0.91    │ Can you analyze the        │ │    500 tokens • 01:00     │ │
│                 │ compaction logs?           │ │                           │ │
│   🔧 Error      │                            │ │ 📦 Segment 2 (T11-T20)    │ │
│   T8 • 0.85     │ Assistant:                 │ │    650 tokens • 02:15     │ │
│                 │ I found the issue...       │ │                           │ │
│   📍 Check      │                            │ │ 📦 Segment 3 (T21-T30)    │ │
│   T5 • 0.80     │ Tools: 3 calls             │ │    480 tokens • 03:45     │ │
│                 │ Files: 1 modified          │ │                           │ │
│   🏁 Mile       │                            │ │ (Select + L to load)      │ │
│   T2 • 0.75     │ ✅ Success                 ▼ │                           ▼ │
├─────────────────┴──────────────────────────────┴─────────────────────────────┤
│ ←→ Switch Pane │ ↑↓ Navigate │ Enter Expand │ L Load │ D Delete │ Esc Close  │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Pros:** Everything visible at once, supports advanced workflows
**Cons:** Complex, cramped on smaller terminals, over-engineered for v1

---

## Layout 5: 40/60 with Inline Actions (Recommended for Extensibility)

Simple split with inline action indicators on selected item. Actions appear contextually.

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ 📍 Conversation Anchors                                                 ESC  │
├───────────────────────────┬──────────────────────────────────────────────────┤
│                           │                                                ▲ │
│ ▸ ✅ TaskCompletion       │ TURN 14                                        █ │
│   Turn 14 • 0.91          │ ────────────────────────────────────────────── █ │
│   [D]elete [L]oad         │                                                │ │
│                           │ 👤 User:                                       │ │
│   🔧 ErrorResolution      │ Can you analyze the compaction logs and        │ │
│   Turn 8 • 0.85           │ figure out why the anchor detection is         │ │
│                           │ failing?                                       │ │
│   📍 UserCheckpoint       │                                                │ │
│   Turn 5 • 0.80           │ 🤖 Assistant:                                  │ │
│                           │ I found the issue with anchor detection.       │ │
│   🏁 FeatureMilestone     │ The JSON parsing was failing because LLMs      │ │
│   Turn 2 • 0.75           │ wrap their responses in markdown code blocks.  │ │
│                           │                                                │ │
│                           │ I've added a new function to extract JSON:     │ │
│                           │ `extract_json_from_response()` which handles   │ │
│                           │ ```json blocks properly.                       │ │
│                           │                                                │ │
│                           │ 🔧 Tools: Read, Edit, Bash (3 calls)           │ │
│                           │ 📁 Files: anchor.rs (+35 -2)                   │ │
│                           │ ✅ Status: Success                             ▼ │
├───────────────────────────┴──────────────────────────────────────────────────┤
│ ↑↓ Navigate │ Enter Expand │ D Delete │ L Load Context │ A Add Anchor │ Esc  │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Pros:** 
- Clean split layout with ample preview space
- Inline hints show available actions on selected item
- Footer provides complete action reference
- Easy to extend with new actions (just add to footer + implement)
- Right pane is standard VirtualList for scrollable content

**Cons:** 
- Inline hints take a line in anchor list

---

## Recommendation

**Layout 5** is recommended because:

1. **Simple but extensible** - 40/60 split is proven (SplitSessionView uses it)
2. **Actions are discoverable** - Inline hints on selection + footer reference
3. **VirtualList on both sides** - Left for anchors, right for turn content
4. **Room for future features**:
   - `D` Delete anchor
   - `L` Load compacted segment into context
   - `A` Add custom anchor
   - `G` Go to turn in conversation
5. **Consistent with existing patterns** - Similar to watcher split view

The three-pane layout (4) is interesting for advanced use but should be a future enhancement after the "load compacted content" feature is actually implemented.
