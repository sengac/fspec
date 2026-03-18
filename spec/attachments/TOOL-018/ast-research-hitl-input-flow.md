# AST Research: HITL Input Flow

## Files to Modify

### 1. `src/tui/hooks/useHitlInput.ts` (212 lines)

**State variables (lines 73–75):**
- `questionIndex` (0-based index into multi-question flow)
- `selectedOption` (0-based index into current question's options)
- `answers` (accumulated `HitlAnswer[]`)

**Derived state (lines 79–82):**
- `isCurrentQuestionFreeform`: true when current question has no options or empty options array

**Keyboard handler (lines 124–201), registered at `InputPriority.HIGH`:**
- Escape → `handleCancel()` → `sessionSendHitlResponse(sessionId, { cancelled: true })` (lines 139–142)
- ↑ Arrow → decrements `selectedOption` with wrap-around, only if has options (lines 146–151)
- ↓ Arrow → increments `selectedOption` with wrap-around, only if has options (lines 152–157)
- Enter → two branches (lines 161–192):
  - Options question: captures `options[selectedOption].label` into `selected: [label]`
  - Freeform question: captures `inputValue` as `other` field
  - Then: advance questionIndex or submit all answers
- Other keys (freeform): returns `false` to let input pass through to MultiLineInput

**Changes needed:**
- Add `isOtherActive` state (boolean, default false)
- When `selectedOption === options.length` (the "Other..." index) and Enter pressed → set `isOtherActive = true`
- When `isOtherActive` and Escape pressed → set `isOtherActive = false` (return to options), NOT cancel
- When `isOtherActive` and Enter pressed → validate non-empty, capture as `{ selected: [], other: inputValue }`
- When `isOtherActive` and Enter pressed with empty text → reject, no state change
- Wrap-around for ↑/↓ must account for `options.length + 1` (extra "Other..." entry)
- Expose `isOtherActive` in return value
- Update `isCurrentQuestionFreeform` logic or add separate flag for "Other..." freeform mode

### 2. `src/tui/components/InputTransition.tsx` (571 lines)

**HITL props (lines 107–125):**
- `hitlRequest`, `hitlQuestionIndex`, `hitlSelectedOption`, `hitlFreeformActive`

**HITL rendering branch (lines 372–437):**
- Branch A — Freeform (lines 381–405): `!hasOptions && hitlFreeformActive` → MultiLineInput
- Branch B — Options (lines 408–436): option list with ●/○ indicators

**Changes needed:**
- Add `hitlOtherActive?: boolean` prop
- Branch B (options rendering): append virtual "Other..." entry after real options
  - Render in dim/italic text to distinguish from LLM options
  - Selected state: `hitlSelectedOption === options.length` highlights the "Other..." entry
- New Branch: when `hitlOtherActive` is true and `hasOptions` → render MultiLineInput (like Branch A) but with different header indicating "Other..." mode
- Add hint text rendering for empty submission rejection

### 3. `src/tui/components/AgentView.tsx`

**Wiring (lines 1235–1241 and 5165–5193):**
- Pass `hitlInput.isOtherActive` to InputTransition as `hitlOtherActive`

### 4. `src/tui/types/hitlRequest.ts` (117 lines)

**No changes needed** — answer shape `{ selected: [], other: "text" }` already supported.

## No Rust/NAPI Changes

The "Other..." option is TUI-only. No changes to:
- `codelet/tools/src/request_user_input.rs`
- `codelet/napi/src/` HITL bindings
- Tool schema or HitlRequest/HitlResponse types
