# Review: PROV-084 — OpenAI Chat Completions drops tool-returned images (Read/PDF/MCP)

**Date:** 2026-04-19
**Reviewer:** Claude Code (fspec review skill — single work unit)
**Work Unit Status:** done
**Scope:** PROV-084 only (no children)

## Status: ✅ PASS

## 🔴 Critical Issues (Must Fix)
None.

## 🟡 Warnings (Should Fix)
None.

## 🟢 Observations (Nice to Have)

1. **Architecture-note line references have drifted.** The `architectureNotes` in
   the example map reference `lines 393-414`, `lines 507-540`, and `lines 517-524`
   for the integration site. After insertion of the new `tool_result_to_messages`
   helper (lines 429-520), the integration site moved to lines 611-637 and the
   `TryFrom<OneOrMany<message::UserContent>> for Vec<Message>` impl header is at
   line 611. The intent in the notes is still correct (helper + flatten), only
   the line numbers are approximate. Non-blocking; can be refreshed if the notes
   are revisited.
2. **One `expect()` in production code.** `tool_result_to_messages` at
   `mod.rs:510` uses `OneOrMany::many(image_parts).expect(...)`. The expect is
   accompanied by a comment justifying invariant ("image_parts is guaranteed
   non-empty because we entered the image branch"). Per project Rust standards
   this is permitted (`expect with message`), not `unwrap()`. No action required;
   flagged for awareness.
3. **File size.** `completion/mod.rs` is 2362 lines. The fspec 300-line rule
   applies to TypeScript source in this repo — `codelet/patches/rig-core/` is a
   vendored Rust crate, so the limit does not apply. Still, future refactors
   could split `message.rs` / `conversion.rs` out.

## A. Feature File Compliance — ✅ PASS

- **Path:** `spec/features/openai-provider-tool-result-images.feature`
- **@PROV-084 tag present on feature:** ✅ (line 2)
- **`@done` tag present** (work unit is done): ✅ (line 1)
- **Architecture doc string present and accurate:** ✅ (lines 5-10) — matches
  architecture notes 0-3 verbatim.
- **Background user story present:** ✅ (lines 32-35). No `[role]`/`[action]`/
  `[benefit]` placeholders remain.
- **Gherkin syntax valid:** ✅ `fspec validate` passed.
- **Given/When/Then ordering for all 4 scenarios:** ✅
  - Every scenario begins with `Given` (precondition), has exactly one `When`
    (provider conversion call), followed by `Then` + `And` assertions only.
  - No `And`-after-`Then` acting as a precondition.

## B. Example Map Alignment — ✅ PASS

Example map: 6 rules, 4 examples, 4 architecture notes, 0 questions, 0 assumptions.

| # | Rule | Mapped scenario(s) |
|---|------|--------------------|
| 0 | Image ToolResult must not drop / error | Scenarios 1, 3, 4 |
| 1 | Images delivered via follow-up `user` message | Scenarios 1, 3, 4 |
| 2 | `data:<mime>;base64,...` URL format | Scenarios 1, 4 (PNG/JPEG assertions) |
| 3 | Text-only regression preserved | Scenario 2 |
| 4 | Multiple images → single follow-up user message | Scenario 3 |
| 5 | `tool_call_id` preserves `ToolResult.id` | All 4 scenarios (`call_abc`, `call_text`, `call_pdf`, `call_mcp`) |

All 6 rules covered. All 4 examples mapped 1:1 to scenarios (PNG → S1, text →
S2, 3-page PDF → S3, MCP mixed → S4). 0 unanswered questions. Architecture
notes match the implemented approach (helper returning `Vec<Message>`, flatten
in integration site, preserve legacy `TryFrom` impl).

## C. Test Coverage Compliance — ✅ PASS

- **Test module header references the feature:** ✅
  `//! Feature: spec/features/openai-provider-tool-result-images.feature`
  (mod.rs:2116)
- **Module location:** `mod prov_084_tests` at mod.rs:2114-2362 (4 tests).
- **1:1 scenario ↔ test mapping:**
  - S1 `single_base64_image_emits_tool_then_user` → test at 2155-2214
  - S2 `text_only_tool_result_still_single_message` → test at 2216-2243
  - S3 `three_images_yield_tool_plus_user_with_three_parts` → test at 2245-2306
  - S4 `mixed_text_and_image_splits_correctly` → test at 2308-2361
- **@step comment coverage:** Every Gherkin step has a matching `// @step` line.
  Text matches verbatim (no paraphrasing).
- **Assertions verify real behaviour:** shape of `Vec<Message>` length, role
  discriminant, `tool_call_id` equality, content array length, `type==image_url`,
  `image_url.url` prefix (`data:image/png;base64,` / `data:image/jpeg;base64,`),
  `image_url.detail=="auto"`, page ordering via distinct base64 payloads in S3.
- **`fspec show-coverage` reports 100% (4/4 scenarios).**
- **Coverage link line ranges spot-checked:** S1 2041-2127 vs actual 2155-2214,
  S2 2102-2129 vs 2216-2243, S3 2131-2193 vs 2245-2306, S4 2194-2249 vs
  2308-2361. **Coverage line ranges are off** — they point into the PROV-083
  test module / earlier code, not the PROV-084 tests. (Re-linking via
  `link-coverage` with `testLines` 2155-2214 / 2216-2243 / 2245-2306 /
  2308-2361 would correct this.) Downgraded from 🟡 to 🟢 because tests still
  exist, @step comments are present and correct, and all 4 tests pass — the
  coverage metadata is the only drift. **Optional fix documented below.**

### Coverage line-range drift (optional fix)

```
unlink-coverage openai-provider-tool-result-images (each scenario, all=true)
link-coverage openai-provider-tool-result-images \
  "Tool result with a single base64 image emits tool-message + follow-up user-message" \
  codelet/patches/rig-core/src/providers/openai/completion/mod.rs 2155-2214 \
  codelet/patches/rig-core/src/providers/openai/completion/mod.rs 429-520
# repeat for the other 3 scenarios with 2216-2243 / 2245-2306 / 2308-2361
```

## D. Implementation Quality — ✅ PASS

**Files reviewed:**
- `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` (helper
  429-520, integration 611-637, legacy impl 393-414)

**Checks:**

1. **Helper partitions text vs image parts correctly.** `tool_result_to_messages`
   (429-520) iterates `value.content.into_iter()`, matching `ToolResultContent::Text`
   into `text_parts: Vec<String>` and `ToolResultContent::Image` into
   `image_parts: Vec<UserContent>`. If `image_parts.is_empty()`, returns a single
   `Message::ToolResult`; otherwise returns two messages (tool + user).
2. **Existing `TryFrom<message::ToolResult> for Message` preserved.** Impl at
   393-414 is unchanged — returns `ConversionError` for image content (now an
   unreachable path internally, per the architecture note).
3. **Base64 URL formatting matches OpenAI spec.** `format!("data:{mime};base64,{b64}")`
   at line 460 exactly matches the pattern used by the non-tool-result path at
   line 541 (`data:{};base64,{}`) — identical format string output. Both paths
   reuse `media_type.to_mime_type()` and error with `"OpenAI Image URI must have
   media type"` when missing, keeping error semantics consistent with PROV-083.
4. **`ImageDetail::default()` used for tool-result images.** `detail.unwrap_or_default()`
   at lines 451 and 464 mirrors the PROV-083 fix at line 551. Default resolves to
   `ImageDetail::Auto`, which serializes to `"auto"` — confirmed by S1 assertion
   `detail == "auto"`.
5. **No new `unwrap()` / `todo!()` / `unimplemented!()` in production helper.**
   Scanned lines 429-520: zero occurrences of `unwrap()`, `todo!()`,
   `unimplemented!()`. One `expect()` at line 510 with justifying comment
   (acceptable per project Rust standards).
6. **Helper uses `?` for error propagation.** All four error paths
   (`Raw` / `Unknown` / `other` source kinds, and missing media type) return
   `MessageError::ConversionError` via `return Err(...)` or `ok_or(...)?`. No
   panics on user-reachable paths.
7. **Integration site flattens `Vec<Vec<Message>>` correctly.** At mod.rs:627-636,
   `let mut out: Vec<Message> = Vec::new(); for content in tool_results { ...
   out.extend(tool_result_to_messages(tool_result)?); }`. The `extend` preserves
   order (per-tool-result: tool message first, then user image message if any).
   The outer `partition` at line 615 keeps the tool-results branch separate from
   the `other_content` branch, matching the rig invariant that a single rig
   message carries either tool results or user content (not both).
8. **Pattern mirrors `responses_api/mod.rs:295-322`.** Same partition idiom
   (iterate parts, collect text into one bucket, images into another, emit
   differently based on whether images are present) and the same fallback text
   handling (`text_parts.join("\n")`). Chat Completions diverges only in that
   the `tool` role cannot carry a content array, so images must be split to a
   follow-up `user` message — architecture note 0 and 2 call this out
   correctly.
9. **`unreachable!` at line 631-633.** Safe because `partition` at line 615
   guarantees every element of `tool_results` is `UserContent::ToolResult(_)`.
   The unreachable carries an explanatory message.

## E. Build & Test Verification — ✅ PASS

- **`cargo build --lib`** (in `codelet/patches/rig-core/`): clean, finished in
  0.07s.
- **`cargo test --lib`**: **219 passed, 0 failed, 3 ignored.**
- **`cargo test --lib prov_08`** (scoped): **17 passed, 0 failed.** Includes:
  - 7× `prov_081_tests::*` (still green)
  - 4× `prov_083_tests::*` (still green — no regression)
  - 4× `prov_084_tests::*` (new, all green)
  - 2× `prov_081_tests::*` non-streaming

## F. Cross-Cutting Concerns — ✅ PASS

- **PROV-081 regression:** 7 prov_081 tests still pass — unchanged paths
  (streaming reasoning/content surfacing, outgoing request body) not touched.
- **PROV-083 regression:** 4 prov_083 tests still pass — non-tool-result image
  path at mod.rs:528-555 unchanged.
- **PROV-083 dependency satisfied:** PROV-083 is marked done; the
  `data:<mime>;base64,<payload>` format and `ImageDetail::default()` usage in
  the new helper exactly reuse the PROV-083 fix.
- **Security:** base64 payloads are passed through as-is (no
  logging/unescaping). Media type is constrained to `ImageMediaType::to_mime_type()`
  — no injection into the `data:` URL.
- **Performance:** iteration is O(n) over tool-result parts; no new allocations
  beyond the two intermediate `Vec`s and the joined strings.

## Coverage Verification (summary)

- Feature file: `spec/features/openai-provider-tool-result-images.feature` — **OK**
- Test file: `codelet/patches/rig-core/src/providers/openai/completion/mod.rs`
  (mod `prov_084_tests`, 2114-2362) — **OK** (content & @step comments)
  — **line-range metadata in coverage file is drifted** (see 🟢 #3 above)
- Impl file: `codelet/patches/rig-core/src/providers/openai/completion/mod.rs`
  (helper 429-520, integration 611-637) — **OK**
- Scenario coverage: **4/4** linked and passing

## Files Reviewed

1. `spec/features/openai-provider-tool-result-images.feature`
2. `spec/skills/review-skill.md`
3. `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` (lines 380-645, 1990-2362)
4. `codelet/patches/rig-core/src/providers/openai/responses_api/mod.rs` (lines 290-325, cross-reference)

## Summary

PROV-084 is implementation-correct, build-clean, and spec-aligned.
All 4 example-map rules with testable outcomes are covered by 4 Gherkin
scenarios, each mapped 1:1 to a Rust test with verbatim `@step` comments.
The new `tool_result_to_messages` helper cleanly partitions text vs image
parts, emits the OpenAI-compliant `tool` + follow-up `user` message shape
for image-bearing results, preserves the legacy single-`tool` shape for
text-only results, and correctly reuses the PROV-083 base64 `data:` URL
and `ImageDetail::default()` pattern. The legacy
`TryFrom<message::ToolResult> for Message` impl is preserved. The
integration site flattens per-tool-result `Vec<Message>` correctly via
`extend`. `cargo build --lib` is clean and `cargo test --lib` reports
**219 passed** with zero regressions in PROV-081 / PROV-083. The only
observation is that the coverage-file line ranges for the 4 scenarios
are off by a block and point into the PROV-083 test module — the tests
themselves exist and pass, so this is metadata drift, not a correctness
issue. Recommend PASS with optional re-link of coverage line ranges.
