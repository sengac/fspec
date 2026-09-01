# Research: fspec Foundation Discovery System — Efficiency & Clarity Refactor

Status: research complete (simulation performed 2026-09-01)
Author: agent session (DISC prefix)
Scope: ALL foundation-discovery tools: discover-foundation, update-foundation,
show-foundation, add/remove-capability, add/remove-persona,
add/remove-foundation-bounded-context, add/remove-aggregate-to-foundation,
add/remove-domain-event-to-foundation, add/remove-command-to-foundation,
show-foundation-event-storm, list-foundation-sections, generate-foundation-md,
validate-foundation-schema, plus the help configs, workflow-guidance doc, and
the two-front-doors plumbing (dispatch.rs, CLI bridges, NAPI/TS systemReminder
path).

## 1. Inventory of the current system

### 1.1 The 8-field draft workflow

`discover-foundation` (no args) writes `spec/foundation.json.draft` with a
fixed template of 8 ordered fields:

| # | JSON path | Placeholder | How to fill |
|---|-----------|-------------|-------------|
| 1 | project.name | `[QUESTION: What is the project name?]` | `update-foundation projectName "<v>"` |
| 2 | project.vision | `[QUESTION: What is the one-sentence vision?]` | `update-foundation projectVision "<v>"` |
| 3 | project.projectType | `[DETECTED: cli-tool]` | `update-foundation projectType "<v>"` |
| 4 | problemSpace.primaryProblem.title | `[QUESTION: What problem does this solve?]` | `update-foundation problemTitle "<v>"` |
| 5 | problemSpace.primaryProblem.description | `[QUESTION: What problem does this solve?]` | `update-foundation problemDefinition "<v>"` |
| 6 | solutionSpace.overview | `[QUESTION: What can users DO?]` | `update-foundation solutionOverview "<v>"` |
| 7 | solutionSpace.capabilities | `[]` (empty, NOT a placeholder string) | `add-capability` x3-7 (clears placeholder entries) |
| 8 | personas | `[{name:[QUESTION...],...}]` | `add-persona` x1+ (clears placeholder entries) |

After each `update-foundation` on the draft, the command chains to
`scan_draft_for_next_field_reminder` and emits a field-specific
`<system-reminder>` ("Field N/8: ..."). This is the only "what's next"
mechanism in the whole system.

### 1.2 Full command inventory (foundation domain)

Discovery phase:
- `discover-foundation` — create draft (or `--force` regenerate) /
  `--finalize` (validate → write foundation.json → delete draft → auto-create
  FOUND task → regenerate FOUNDATION.md)
- `update-foundation <section> <content>` — 7 named section aliases only
  (projectName, projectVision, projectType, problemTitle,
  problemDescription/problemDefinition, problemImpact, solutionOverview/
  projectOverview + legacy aliases). NO support for arbitrary JSON paths.
- `add-capability` / `remove-capability` — draft-aware (draft wins)
- `add-persona` / `remove-persona` — draft-aware (draft wins)
- `show-foundation` — reads foundation.json by default. `draft: true` arg
  reads the draft. `section` via FIELD_MAP or dotted path. format text/json.
- `list-foundation-sections` — static reference of the 7 update-foundation
  sections (no state awareness, no progress info)
- `validate-foundation-schema` — validates final foundation.json only (never
  the draft!)
- `generate-foundation-md` — regenerates FOUNDATION.md from final
  foundation.json

Foundation Event Storm phase (final foundation only, NOT draft-aware):
- `add-foundation-bounded-context` / `remove-foundation-bounded-context [--cascade]`
- `add-aggregate-to-foundation` / `remove-aggregate-from-foundation`
- `add-domain-event-to-foundation` / `remove-domain-event-from-foundation`
- `add-command-to-foundation` / `remove-command-from-foundation`
- `show-foundation-event-storm [type] [context]` — raw JSON array output
- `generate-tags-md` — TAGS.md generation (guidance doc calls it
  "derive-tags-from-foundation" which DOES NOT EXIST — phantom command)

## 2. Simulated walkthrough (fresh project, agent-driven)

Reconstructed from source (fspec-core/commands/*, fspec/src/* bridges,
tools/src/fspec_workflow_guidance.rs). The agent-facing path is:
Fspec tool → dispatch.rs → run() → JSON envelope → wrapper appends
`systemReminder` to the data string (tools/src/facade/wrapper.rs:1093-1097).

### Step-by-step with the ACTUAL outputs an agent sees

**T0. Session start. No foundation.**
Agent runs `fspec board` (or any command gated by check_foundation_exists) →
gets the big FoundationMissing error with a 3-step workflow +
`<system-reminder>`. Good.

**T1. `discover-foundation`**
Output:
```
<system-reminder>
Draft created. To complete foundation, you must ULTRATHINK the entire codebase.
...
I will guide you field-by-field.

<system-reminder>        ← NESTED system-reminder tags (ugly)
Field 1/8: project.name
...
</system-reminder>
</system-reminder>
```
Then the CLI/agent gets: ✓ Generated spec/foundation.json.draft
+ "Next steps: 1. Use fspec update-foundation commands... 2. ...--finalize"

Problems:
- Nested system-reminder tags (the field reminder is embedded in the outer
  banner which is itself wrapped).
- The agent has to know the field ORDER. If it wants to jump to personas
  first (say, after deep codebase analysis), nothing prevents it — but
  `update-foundation` chaining only reports the FIRST unfilled field, so
  "Field 4/8" reminders still say the next step is field 4 even though the
  agent just filled field 8. The "N/8" numbering implies strict sequence.
- No overall progress display ("3 of 8 fields filled, remaining: ...").

**T2-T5. `update-foundation` per simple field**
Output (draft path):
```
✓ Updated "projectName" in foundation.json.draft
  Updated: spec/foundation.json.draft
<system-reminder>
Field 2/8: project.vision (elevator pitch)
...
</system-reminder>
```
Problems:
- The chain reminder is only emitted by `update-foundation` — NOT by
  `add-capability`, `add-persona`, `remove-capability`, `remove-persona`.
  So after filling the LAST simple field (solutionOverview) and moving to
  `add-capability`, the agent gets ZERO guidance on what to do next (how many
  capabilities? how many personas? when is it done? how to check progress?).
  This is the "doesn't tell the agent what to do next" problem the user
  reported.
- The "N/8" counter is a LIE for capabilities/personas: capabilities and
  personas are multi-entry arrays. The scan counts the whole array as one
  field, but the agent has to add 3-7 capabilities one call at a time with no
  feedback on how many remain "appropriate".
- No way to see the CURRENT draft state in one command that ALSO shows
  remaining fields. `show-foundation --draft` (agent: `show-foundation` with
  `{"draft": true}`) shows the raw draft but:
  * the help config (help/configs/show_foundation.rs) does NOT document the
    `draft` arg at all (OPTIONS only lists --list-sections, --line-numbers —
    both no-ops!) — the agent must guess.
  * `show-foundation` without `draft: true` AUTO-CREATES foundation.json with
    the canonical default literal (ensure_foundation_file) — side effect on a
    "show" command! For a project that has a draft, running
    `show-foundation` (the natural move) silently CREATES a final
    foundation.json with placeholder-y defaults ("Project Name",
    "Project vision statement", "Core Capability"...) and then shows THAT —
    not the draft. This is the "show-foundation is confusing when it doesn't
    show the draft version" problem.
  * Even when the agent does use `draft: true`, the output is raw JSON or
    plain text with the placeholders visible but NO summary of which fields
    remain, what's next, or examples of appropriate content.

**T6. Capabilities.**
`add-capability "Spec Validation" "Validate Gherkin features"`
Output: `✓ Added capability to foundation.json.draft` + name/description.
No reminder, no progress. Agent has to guess: "did I add enough? what are
good capability names?" The field reminder for field 7/8 said "3-7
recommended" but that reminder is gone after step 5 (chaining only fires on
update-foundation). The agent could re-run `update-foundation` on some field
just to get the next reminder — a hack no agent would think of.

**T7. Personas.** Same problem. `add-persona` returns success with no
"how many personas are appropriate" or "next step" guidance.

**T8. Finalize.**
`discover-foundation {"finalize": true}`
- If placeholders remain: `valid:false` + validationErrors listing the FIRST
  placeholder field only (scan returns first hit): "Field 'project.vision'
  still contains [QUESTION:]..." — does NOT list ALL remaining fields. The
  fix commands block is generic.
- If schema fails: lists schema errors (all of them, good) but with a fix
  block that says "For simple fields: fspec update-foundation <section>" —
  the agent must map JSON path → section alias itself.

**T9. Post-finalize event storm.**
`add-foundation-bounded-context` → success message only, no "next steps"
(nothing tells the agent to add aggregates/events/commands, or when to stop,
or to run show-foundation-event-storm, or that generate-tags-md exists).
`show-foundation-event-storm` → raw JSON array (fine for machines, fine for
agent, but has no human/agent-oriented summary: counts by type, which
contexts lack events, etc.)

### 2.1 The "next steps" gap — systematic

| After this call | Agent learns |
|---|---|
| discover-foundation | Field 1/8 only (in a nested reminder) |
| update-foundation (draft) | Only the NEXT single field (first unfilled in fixed order) |
| add-capability | Nothing (success + fields) |
| add-persona | Nothing (success + fields) |
| remove-* | Nothing |
| show-foundation (no draft) | Raw foundation, no draft awareness |
| show-foundation --draft | Raw draft JSON, no progress |
| list-foundation-sections | Static field reference, no state |
| validate-foundation-schema | Final file only; errors; no draft validation |
| finalize (fail) | First placeholder field only (or all schema errors) |
| add-foundation-* / event storm | Nothing |
| show-foundation-event-storm | Raw JSON |

Conclusion: the ONLY state-aware, next-step-emitting code path is
`scan_draft_for_next_field_reminder` (duplicated verbatim in
update_foundation.rs AND discover_foundation.rs — ~100 lines of duplicated
reminder text), invoked from exactly two places (discover create, update
draft). Everything else is silent.

## 3. Root-cause catalogue (what makes it inefficient & confusing)

**R1. No unified "where am I" command.** Progress is implicit. The agent
must reconstruct state by reading raw JSON. A single `foundation-status`
(or enhanced show-foundation) that returns: phase (none | draft | final),
field completion table (8 rows with ✓/✗ + current value preview), remaining
list, next-action suggestions, examples for each remaining field — is the
single most impactful change.

**R2. show-foundation's default is a trap.** Default = final foundation, and
if absent it AUTO-CREATES one (ensure_foundation_file) — meaning the most
natural "show me the foundation" call during discovery silently creates a
final foundation.json with template garbage, and the agent sees that instead
of the draft. The `draft: true` escape hatch is undocumented in the help
config. The draft-vs-final choice should be automatic (draft wins when
present, like the mutation commands do) with an explicit status line saying
which file was shown.

**R3. Chained reminders are one-shot and single-field.** No remaining-fields
list, no progress fraction, no "you are in the middle of X" context. The
"N/8" numbering implies the agent is on a conveyor belt it cannot see.

**R4. Mutation commands (add-capability/add-persona/remove-*) don't chain.**
The draft workflow's two most important array fields are the only ones with
no next-step feedback. They SHOULD emit the same (or a richer) post-mutation
status block.

**R5. The "N/8" model is the wrong model for arrays.** capabilities/personas
are open-ended. The scan should treat them as: "field complete when
non-empty AND no placeholder entries", and guidance should say "you have 3
capabilities so far; 3-7 recommended; run finalize when done" rather than a
phantom "Field 7/8" with no completion semantics.

**R6. Finalize failure reports are incomplete.** Only the FIRST placeholder
field is reported (scan returns early). An agent with 4 remaining fields has
to finalize → fix one → finalize → fix next → ... (4 rounds). All remaining
fields should be listed with per-field fix commands.

**R7. Duplication & drift.** `scan_draft_for_next_field`,
`extract_detected_value`, `field_reminder_body`,
`agent_supports_meta_cognition`, `is_known_agent`, `TOTAL_FIELDS` are
copy-pasted between discover_foundation.rs and update_foundation.rs (each
~200 lines). Any improvement to the reminder text must be made in two places.
They must be unified into one shared module (e.g.
fspec-core/src/foundation/guidance.rs) before improving the text.

**R8. The reminder text itself is thin on "what is appropriate".** The user
explicitly wants examples of what is appropriate for each remaining field.
Currently only projectType has examples (in both the reminder and
list-foundation-sections). Vision, problem title/description, solution
overview, capability names, persona names/goals have zero examples in the
guidance. Each field reminder should carry 1-2 concrete examples (not just
the placeholder question).

**R9. Guidance documentation (tools/fspec_workflow_guidance.rs) is WRONG.**
- `update-foundation` shown with `{"key": ..., "value": ...}` args — the real
  args are `{"section": ..., "content": ...}`.
- `add-capability` shown with `{"capability": ...}` — real arg is `name`.
- `add-persona` shown with `{"goal": ...}` — real arg is `goals: [...]` (and
  the shown JSON is malformed: `{"name": "Developer"}, "description": ...`).
- `remove-capability`/`remove-persona` shown with `{"capability": ...}` /
  `{"name": ...}` — real arg for remove-capability is `name` (not
  `capability`).
- `show-foundation` shown with `{"section": "What We Are Building",
  "format": "json"}` — section names are the 7 aliases, not free text;
  `{"listSections": true}` is not a real arg (the command is
  list-foundation-sections).
- `add-foundation-bounded-context` shown with `{"name": ...}` — real arg is
  `text`.
- `show-foundation-event-storm` example uses `{"type": "bounded-context"}` —
  the real item type is `bounded_context` (underscore).
- `derive-tags-from-foundation` does not exist (phantom).
- `update-foundation` args use `key`/`value` in TWO places (also the
  DRAFT_EXISTS_ERROR / FOUNDATION_EXISTS_ERROR and field-missing error text
  correctly use section names — only the guidance doc is wrong).
This doc is injected into EVERY agent's system prompt — it is the single
biggest source of agent confusion in the whole system.

**R10. Two-front-doors envelope inconsistency.** update-foundation returns
`{success, message, systemReminder?}`; discover-foundation returns
`{valid, systemReminder?, validationErrors?, completionMessage?, ...}`;
add-capability returns `{success, fileName, name, description, removedCount}`;
show-foundation returns a bare string (no envelope). The agent sees different
shapes per command. A consistent envelope
`{success, message, data?, progress?, nextSteps?, systemReminder?}` would
make the tool predictable. (Note: the NAPI path appends systemReminder to
data verbatim — the envelope fields like `progress`/`nextSteps` would only
reach the agent if the TS callback / wrapper surfaces them; the Rust CLI
bridge prints them. This refactor should standardize the envelope and update
the wrapper/TS side to append any `nextSteps` block the same way
systemReminder is appended today.)

**R11. `validate-foundation-schema` can't validate the draft.** The agent's
only way to "check my work" before finalize is... finalize. A draft-aware
validation (or a `--draft` flag) would let the agent catch schema problems
incrementally.

**R12. Event-storm phase has no equivalent guidance at all.** After finalize,
the FOUND task is auto-created with a description listing the commands — but
the commands themselves emit no "next" guidance, and there is no
`foundation-status`-equivalent for the event-storm phase (counts by type,
contexts with/without events, suggested next item type).

**R13. discover-foundation's guard-rail errors are good but the
"continue" path is hard to find.** The DRAFT_EXISTS_ERROR lists 3 options but
none of them is "show progress". Option 2 is "Observe: show-foundation
--draft" — which, per R2, is undocumented in show-foundation's help. The
status command (R1) would make option 2 trivial.

**R14. `show-foundation-event-storm` unmatched context → empty array (no
error).** An agent typo in the context name silently returns `[]` and the
agent concludes "context has no items". Should error on unknown context (or
list available contexts).

## 4. Proposed design (enhancement, no functionality loss)

### 4.1 New shared module: `fspec-core/src/foundation/guidance.rs`

Single source of truth for:
- `FOUNDATION_FIELDS: [(path, alias, kind, examples, guidance_body)]` —
  replaces both copies of the 8-field tables + the 7-section static table in
  list_foundation_sections.rs (which then renders FROM this table — killing
  the "keep in lock-step" drift risk).
- `scan_fields(foundation) -> FieldProgress` — returns ALL fields with
  status (complete | placeholder | missing | empty-array | placeholder-
  entries), not just the first.
- `render_progress_block(foundation, phase) -> String` — the canonical
  "where am I" block (see 4.2), used by every command that can emit it.
- `field_reminder(field) -> String` — per-field guidance with examples
  (enhanced text, see 4.3).
- Keep `agent_supports_meta_cognition` here (currently duplicated).

### 4.2 New/extended command: `foundation-status` (alias: enhanced
`show-foundation`)

Decision (recommended): ADD a dedicated command rather than overloading
show-foundation, because show-foundation's TS-parity contract (byte-exact
output asserted by tests) would break. New command:

`command: "foundation-status"` (also `--json` for machine output)

Output (text, agent-friendly):
```
Foundation: DRAFT (spec/foundation.json.draft)   [or: FINAL | MISSING]

Progress: 4/8 fields complete
  ✓ 1. project.name                 fspec
  ✓ 2. project.vision               A CLI tool for ...
  ✓ 3. project.projectType          cli-tool
  ✓ 4. problemSpace...title         Spec workflow gaps
  ✗ 5. problemSpace...description   [QUESTION: What problem does this solve?]
  ✗ 6. solutionSpace.overview       [QUESTION: What can users DO?]
  ● 7. solutionSpace.capabilities   0 added (3-7 recommended)
  ✗ 8. personas                     1 placeholder persona still present

Remaining (in any order — fill what you can):
  5. problemDefinition → update-foundation problemDefinition "..."
     Example: "Developers track specs in files that drift out of sync with code..."
  6. solutionOverview → update-foundation solutionOverview "..."
  7. capabilities → add-capability "<name>" "<description>" (3-7 total)
  8. personas → add-persona "<name>" "<description>" --goal "..."

When complete: fspec discover-foundation --finalize
```

JSON mode: `{phase, progress:{complete,total}, fields:[{path,alias,kind,
status,preview,example?,fixCommand?}], remaining:[...], nextAction,
draftPath?, finalPath?, eventStorm?: {...}}`.

Also: `show-foundation` (existing) gets:
- Automatic draft preference when a draft exists (mirrors the mutation
  commands' draft-wins rule), with a one-line banner: "Showing DRAFT
  (foundation.json.draft) — use `show-foundation --final` for the finalized
  file". This directly fixes the user's #1 confusion without breaking the
  no-draft case.
- `--final` flag (and `draft: "auto"|"draft"|"final"` arg shape for the
  dispatcher) to force either.
- A short `progress:` line in text mode when a draft is shown (reusing
  4.1's renderer) — not the full status, just `4/8 fields complete; run
  `foundation-status` for details`.

`validate-foundation-schema` gains `draft: true` support (validates the
draft against the same schema, reporting missing/placeholder fields as the
schema would see them).

### 4.3 Richer per-field reminders (with examples)

Every field's guidance body gains an `Examples:` line. E.g.:

```
Field 5/8: problemSpace.primaryProblem.description

USER perspective: Describe the problem users face in detail.
Examples:
  "Developers manage Gherkin feature files by hand; specs drift out of
   sync with code and nothing flags stale acceptance criteria."
  "Small teams track work in chat threads; decisions are lost and
   nothing links a spec to its tests."

Run: fspec update-foundation problemDefinition "<description>"
```

(The example corpus lives in guidance.rs, single source.)

### 4.4 Universal post-mutation "status trailer"

Every foundation-domain mutation command (update-foundation, add/remove-
capability, add/remove-persona, the six add/remove-foundation-event-storm
commands) appends a compact status trailer to its result:

Draft phase:
```
progress: 5/8 fields complete | remaining: solutionOverview, capabilities, personas
next: add-capability "<name>" "<description>"   (or the next field's command)
```

Event-storm phase (final foundation):
```
eventStorm: 2 contexts, 3 aggregates, 4 events, 1 command | last: bounded_context "Auth"
next: add-aggregate-to-foundation "Auth" "<Aggregate>"   (or show-foundation-event-storm)
```

Implementation: each command's run() calls
`guidance::render_trailer(project_root, self)` and stores it in a standard
envelope field `nextSteps`. The CLI bridges print it (2 indented lines); the
agent-facing wrapper appends it to data the same way it appends
systemReminder today (wrapper.rs:1093). This closes R4 + R12.

### 4.5 Finalize failure reports ALL remaining fields

Replace the first-field-only scan in `finalize()` with a full progress
listing: every incomplete field, its status, and its exact fix command.
(Schema errors already list all; keep.)

### 4.6 Consistent envelope (R10)

All foundation commands return:
`{success, message, data?, progress?, nextSteps?, systemReminder?}`
- `success` bool (discover-foundation's `valid` maps to this; keep `valid`
  as an alias for one release cycle).
- `message` short human line.
- `data` structured payload (replaces bare-string show-foundation: its
  rendered text moves into `data.rendered` + `data.section`).
- `progress` / `nextSteps` / `systemReminder` as in 4.2/4.4.
Update: dispatch.rs envelope handling (if any), all 12+ CLI bridges
(marshalling-only change), the NAPI/TS callback to forward `nextSteps`
(appended after data, before/after systemReminder — one convention).

### 4.7 Show-foundation-event-storm unknown-context error (R14)

Unmatched `context` filter → error envelope listing available bounded
contexts (instead of silent `[]`). Keep the type filter as-is.

### 4.8 Fix the injected guidance doc (R9) — highest ROI, zero code risk

Rewrite the "Phase 0: FOUNDATION" section of
tools/src/fspec_workflow_guidance.rs with CORRECT arg names
(section/content, name/description, goals[], text), the
`foundation-status` command added to the flow, and
`generate-tags-md` replacing the phantom `derive-tags-from-foundation`.
Also fix the DRAFT_EXISTS_ERROR option 2 to name `foundation-status`.

### 4.9 Suggested command count (net)

- NEW: `foundation-status` (1 command; ~150 lines in guidance.rs + ~80 in
  commands/foundation_status.rs)
- EXTENDED (no new commands): show-foundation (auto-draft + banner +
  progress line + --final), validate-foundation-schema (--draft),
  discover-foundation --finalize (full remaining list), all 10 draft
  mutations + 6 event-storm mutations (trailer),
  show-foundation-event-storm (unknown-context error).
- CONSOLIDATED: the duplicated scan/reminder code in
  update_foundation.rs + discover_foundation.rs moves to guidance.rs
  (net deletion of ~250 duplicated lines).

Total new concepts for the agent: ONE new command. Total deleted confusion:
all of R1-R14.

### 4.10 Migration / risk notes

- show-foundation byte-parity tests (TS-parity) constrain changes: keep the
  default (no-args, no-draft, final file) output byte-identical when NO
  draft exists; the auto-draft behavior only kicks in when a draft is
  present (a state where the current output is actively misleading anyway).
- The `systemReminder` string is asserted by existing tests in
  discover_foundation.rs / update_foundation tests; the enhanced text must
  keep the asserted substrings ("Field 2/8: project.vision",
  "Run: fspec update-foundation projectVision", "ULTRATHINK" branch) —
  append examples AFTER existing text, don't reword.
- Two-front-doors: all changes land in fspec-core; bridges are
  marshalling-only. The envelope change touches every bridge but
  mechanically (add pass-through of `nextSteps`/`progress`).
- The agent-facing path (NAPI → TS → wrapper) must also forward `nextSteps`
  (napi/types.rs FspecResult + wrapper.rs append logic) — otherwise the
  trailer only helps CLI users. This is the one truly cross-cutting piece;
  estimate a small follow-up story if it blows up.
- No functionality is removed: every existing flag/arg/behavior is kept;
  the auto-draft preference is additive and overridable (--final).

## 5. Recommended story breakdown

Story A (this story's scope — design + shared module + core):
1. guidance.rs (field table w/ examples, full scan, progress renderer,
   unified reminders)
2. foundation-status command
3. show-foundation auto-draft + --final + progress line
4. finalize full-remaining-fields report
5. universal trailer on 16 mutation commands
6. event-storm unknown-context error
7. dedup cleanup (delete the two copies)

Story B (follow-up, separate): consistent envelope + NAPI/TS forwarding of
nextSteps (cross-cutting, touches wrapper + napi types + all 12 bridges).

Story C (follow-up, quick): rewrite the Phase-0 FOUNDATION section of
fspec_workflow_guidance.rs with correct args + new command (docs only, no
runtime code — but it's the highest agent-visibility fix).

## 6. Files touched (Story A)

- rust/fspec-core/src/foundation/guidance.rs (NEW; ~300 lines; keep <300 or
  split field_text.rs)
- rust/fspec-core/src/foundation/mod.rs (NEW)
- rust/fspec-core/src/commands/foundation_status.rs (NEW)
- rust/fspec-core/src/commands/show_foundation.rs (draft auto-preference,
  --final, progress line)
- rust/fspec-core/src/commands/validate_foundation_schema.rs (--draft)
- rust/fspec-core/src/commands/discover_foundation.rs (dedup → guidance.rs;
  full remaining list in finalize)
- rust/fspec-core/src/commands/update_foundation.rs (dedup → guidance.rs)
- rust/fspec-core/src/commands/{add,remove}_capability.rs,
  {add,remove}_persona.rs, {add,remove}_foundation_bounded_context.rs,
  {add,remove}_aggregate_to_foundation.rs,
  {add,remove}_domain_event_to_foundation.rs,
  {add,remove}_command_to_foundation.rs,
  show_foundation_event_storm.rs (trailer / context error)
- rust/fspec-core/src/list_foundation_sections.rs (render from
  guidance.rs table)
- rust/fspec-core/src/dispatch.rs (register foundation-status)
- rust/fspec-core/src/help/configs/foundation_status.rs (NEW) +
  show_foundation.rs (document --final / draft auto-behavior) +
  help_dispatch_table.rs entry
- rust/fspec/src/foundation_status.rs (NEW CLI bridge) + main.rs clap
  variant + 2-line nextSteps printing in the 12 mutation bridges
- tests: fspec-core/tests/foundation_status.rs (new), updates to
  show_foundation / discover_foundation / update_foundation /
  add_capability / add_persona test files for the trailer lines
- spec/features/foundation-discovery-guidance-and-status.feature (NEW
  feature file — ACDD requires the spec FIRST; this research doc is the
  input)

## 7. Open questions (for the human)

Q1. New command name: `foundation-status` vs extending `show-foundation`?
    (Recommendation: new command — parity-test safety, one-command-one-
    purpose matches FOUND-044 convention.)
Q2. Should the auto-draft preference in show-foundation also apply to the
    event-storm show command (show-foundation-event-storm never sees drafts —
    event storm only exists in the final file, so NO change needed there;
    confirming.)
Q3. Story B (envelope + NAPI forwarding) — include in this refactor or a
    separate story? (Recommendation: separate; Story A is already 8+ points.)
Q4. How aggressive on examples: 1-2 per field inline in reminders vs
    "run list-foundation-sections for examples"? (Recommendation: 1-2
    inline — the user explicitly asked for examples of what is appropriate.)
