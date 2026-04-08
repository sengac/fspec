# fspec Foundation Workflow Analysis: A Case Study in Friction

## Executive Summary

This document analyzes the significant friction encountered while attempting to create a project foundation using fspec. What should have been a straightforward 5-minute task became a 30+ minute exercise in confusion, trial-and-error, and eventual manual file manipulation.

**Important correction from the original draft:** An initial version of this analysis concluded that `discover-foundation --finalize` "doesn't actually finalize." After reading the source code (`src/commands/discover-foundation.ts` lines 297–489), **that claim is wrong**. `--finalize` does perform a full finalization: validation → write `foundation.json` → delete draft → create `FOUND-XXX` work unit → generate `FOUNDATION.md`. The friction was real, but the root cause was different from what was originally diagnosed.

The actual root causes are: (1) deferred validation that only catches invalid enum values at the end of the workflow instead of at the point of entry, (2) no way to observe draft state without reading the file directly, (3) vague/truncated help text ("etc."), and (4) misleading help language that talks about "FOUNDATION.md sections" when the command actually updates JSON fields.

---

## The Journey: What Actually Happened

### Phase 1: Initial Setup (Correct)
1. Ran `discover-foundation` — correctly created `foundation.json.draft`
2. Used `add-capability` and `add-persona` commands to populate the draft
3. Used `update-foundation projectType "web-saas"` — the command accepted this value silently (**this is where the bug actually was**)
4. Draft file contained content that *looked* complete for a "Prometheus" marketing platform

### Phase 2: The Confusion Spiral (Where Things Went Wrong)

#### Attempt 1: Trying to "Finalize"
```
command: "discover-foundation", args: {"finalize": true}
Result: Schema validation failed. projectType "web-saas" is not a valid enum value.
```

The error message was interpreted as "missing required: project.projectType" due to how Ajv surfaces enum failures, which misled the user into thinking the field wasn't filled at all. **This is a real error-message bug**, not a rename issue.

#### Attempt 2: Removing Draft and Trying Again
- Removed `foundation.json.draft`
- Ran `discover-foundation --finalize` again
- Result: Error — no draft to finalize

At this point, the user had inadvertently destroyed all of their Example Mapping work because there was no way to `show-foundation --draft` before deleting.

#### Attempt 3: Self-Imposed Blockage
- Added a blocklist rule preventing direct access to `foundation.json.draft`
- This created a catch-22: couldn't read the draft to debug, couldn't discover a valid flow through fspec.

#### Attempt 4: Manual Workaround
- Removed blocklist
- Read draft file directly (bypassing fspec)
- Manually copied draft content to `foundation.json`
- Edited `projectType` from `"web-saas"` to `"cli-tool"` (because `"web-saas"` isn't in the enum)
- Ran `validate-foundation-schema` — passed
- Ran `generate-foundation-md` — worked

**Time elapsed:** ~35 minutes for a task that should take 5.

**The intended flow that would have worked:**
```
fspec update-foundation projectType "web-app"
fspec discover-foundation --finalize
```

One command fix. But this was never discoverable from the error message.

---

## Root Cause Analysis

### 1. Validation Is Deferred to the Wrong Moment (PRIMARY ROOT CAUSE)

This is the single most important issue. Reading `src/commands/update-foundation.ts` lines 143–215, the `updateJsonField()` function validates exactly **one** enum field (`problemImpact` — lines 184–187) at write time. Every other field, including `projectType`, is written verbatim with zero validation.

This means an LLM can run:
```
fspec update-foundation projectType "web-saas"
```
…and get back `✓ Updated "projectType" in foundation.json.draft` with no indication that the value is invalid. The JSON Schema enum check doesn't fire until `discover-foundation --finalize` is run, which may be many turns and many tool calls later.

**Valid values (from `src/schemas/generic-foundation.schema.json` lines 36–50):**
`web-app`, `cli-tool`, `library`, `sdk`, `mobile-app`, `desktop-app`, `service`, `api`, `other`

Note that the system-reminder emitted during discovery (`discover-foundation.ts` line 127) **does** list these 9 valid values. So the information is available — it's just not enforced at write time. A weaker LLM that isn't carefully reading the system-reminder will happily substitute `"web-saas"`, `"saas"`, `"cli"`, or `"rest-api"` and fly blind until the end of the workflow.

**The fix:** Validate enum fields inline in `updateJsonField()` alongside the existing `problemImpact` check. Return an actionable error listing the valid values. This creates a tight feedback loop and prevents the "fill 8 fields, one of them wrong, fail at the end" anti-pattern.

### 2. The "Draft Already Exists" Error Is a Dead End

When `discover-foundation` (without `--finalize`) is invoked while a draft already exists, the command returns:

> Failed to create draft - draft already exists

This tells the user what they **can't** do, not what they **can** do. For a weaker LLM, this is a dead end — there's no actionable next step. A better error message would offer the three legitimate options:

```
Draft exists at spec/foundation.json.draft

Next steps:
  • fspec discover-foundation --finalize  (validate and promote to foundation.json)
  • fspec show-foundation --draft          (view current draft state)
  • fspec discover-foundation --force      (overwrite draft — destructive)
```

### 3. No Way to Observe Draft State

Confirmed by reading `src/commands/show-foundation.ts`: there is no `--draft` flag. `showFoundation()` calls `ensureFoundationFile(cwd)` which reads `spec/foundation.json` only. If a draft exists but the final file doesn't, the command fails.

For weaker LLMs that rely on observe-act-observe loops, hiding the working state is a major usability problem. The only ways to inspect the draft today are:

1. Read the file directly (via the Read tool) — this leaked into the user's workaround
2. Call `discover-foundation` without `--finalize`, which errors out (see #2 above)
3. Call `update-foundation` on a dummy field, which chains to the next field system-reminder — but this doesn't dump the raw content

**The fix:** Add `fspec show-foundation --draft` that reads `foundation.json.draft` and displays it using the same formatter as the existing `show-foundation` command.

### 4. Misleading Help Text for `update-foundation`

From `src/commands/update-foundation-help.ts` line 6:

> "Update section content in FOUNDATION.md"

This is wrong in a subtle but harmful way. The command updates **JSON fields** in `foundation.json` or `foundation.json.draft`. The markdown is regenerated as a side effect (and only for the final file, not the draft). But the help text primes the LLM to think in terms of markdown headings, which is why users try things like:

```
update-foundation "What We Are Building" "..."   → Unknown section
update-foundation "project.name" "..."            → Unknown section
```

Neither of these is a valid section name. The actual valid names are hard-coded in a switch statement in `update-foundation.ts` (lines 143–215):

| Section Name (+ aliases) | JSON Path |
|---|---|
| `projectName` / `name` | `project.name` |
| `projectVision` / `vision` | `project.vision` |
| `projectType` | `project.projectType` |
| `problemTitle` | `problemSpace.primaryProblem.title` |
| `problemDefinition` / `problemDescription` | `problemSpace.primaryProblem.description` |
| `problemImpact` | `problemSpace.primaryProblem.impact` (enum: high/medium/low) |
| `solutionOverview` / `projectOverview` | `solutionSpace.overview` |

Capabilities and personas are **not** updated via `update-foundation` — they have dedicated `add-capability`, `remove-capability`, `add-persona`, `remove-persona` commands. This distinction is also not called out in the help.

**The fix:**
1. Rewrite the help description: `"Update a field in foundation.json (or foundation.json.draft during discovery)"`
2. Add the full enumerated list of section names directly to the help examples section (remove "etc.")
3. Explicitly mention that capabilities/personas use different commands

### 5. No `list-foundation-sections` Command

The current help text ends with: `"Supported section names: projectName, projectVision, projectType, problemDefinition, problemImpact, solutionOverview"` — buried at line 102 of the help file. There is no first-class discovery mechanism for section names. `show-foundation` has `--list-sections`, but it lists sections for display, not for `update-foundation` input.

**The fix:** Add a dedicated `list-foundation-sections` command (or `update-foundation --list-sections`) that prints the enumerated list with JSON paths and enum constraints for each field.

### 6. Error Message Quality at Finalization

When `--finalize` fails schema validation, the error comes out as:

```
Missing required: project.projectType
```

This is technically wrong for enum failures — the field isn't missing, it contains an invalid value. Reading `discover-foundation.ts` lines 336–365, the error formatter only extracts `instancePath` and `params.missingProperty`. It doesn't handle Ajv's `keyword: 'enum'` case, so enum failures get degraded to generic "missing required" messages.

**The fix:** Extend the error formatter to detect `err.keyword === 'enum'` and emit a message like:

```
Invalid value at project.projectType: "web-saas"
Valid values: web-app, cli-tool, library, sdk, mobile-app, desktop-app, service, api, other

Fix by running: fspec update-foundation projectType "<valid-value>"
```

---

## Correcting the Original Analysis

A previous version of this document made several claims that turned out to be wrong after reading the source code. For the record:

| Original Claim | Reality |
|---|---|
| "`--finalize` doesn't actually finalize, it only validates" | **False.** `--finalize` validates, writes `foundation.json`, deletes the draft, creates a `FOUND-XXX` work unit, and generates `FOUNDATION.md`. See `discover-foundation.ts` lines 384–390. |
| "The validation was checking the wrong file" | **False.** `--finalize` reads the draft (line 301), not the final file. |
| "The schema should accept 'web-saas'" | The 9-value enum covers this case via `"web-app"`. "web-saas" is not an industry-standard category distinct from "web-app". |
| "The two-file system is unnecessary" | Arguable but invasive. The draft/final split exists so validation can be deferred until the workflow is complete. Fixing the real issues (#1-6 above) removes most of the friction without redesigning this. |
| "Rename `--finalize` to `--validate`" | **Wrong.** `--finalize` is correctly named. The problem was that a validation error at finalization gave a misleading message, leading to the mistaken belief that finalization didn't happen. |

The lesson here is important for anyone writing this kind of analysis: **read the source before concluding the tool is broken**. The user experience was genuinely broken, but not for the reasons originally diagnosed.

---

## Proposed Solutions

### Priority 1: Fail-Fast Enum Validation (Highest Impact)

**File:** `src/commands/update-foundation.ts`

Extend `updateJsonField()` to validate enum fields at write time:

```typescript
case 'projectType': {
  const validTypes = ['web-app', 'cli-tool', 'library', 'sdk',
                      'mobile-app', 'desktop-app', 'service', 'api', 'other'];
  if (!validTypes.includes(content)) {
    return false;
  }
  foundation.project = foundation.project || {};
  foundation.project.projectType = content;
  return true;
}
```

Improve the error returned by `updateFoundation()` to include the valid values, matching the pattern already used for `problemImpact` (high/medium/low).

### Priority 2: `show-foundation --draft` Flag

**File:** `src/commands/show-foundation.ts`

Add a `--draft` option that reads `spec/foundation.json.draft` instead of `spec/foundation.json`, using the same rendering pipeline. Gives LLMs (and humans) observability into the working state of discovery.

### Priority 3: Improve `update-foundation` Help Text

**File:** `src/commands/update-foundation-help.ts`

- Change description from "Update section content in FOUNDATION.md" to "Update a field in foundation.json (or foundation.json.draft during discovery)"
- Add the complete enumerated section list to the help body
- Add a note that capabilities and personas use dedicated `add-capability`/`add-persona` commands
- Remove the "etc." from the supported sections list

### Priority 4: Better "Draft Exists" Error Message

**File:** `src/commands/discover-foundation.ts` (around lines 498–500)

When detecting an existing draft, instead of "Failed to create draft - draft already exists", emit a system-reminder offering the three valid next steps: `--finalize`, `show-foundation --draft`, or `--force`.

### Priority 5: Ajv Enum Error Formatting

**File:** `src/commands/discover-foundation.ts` (lines 336–365)

Extend the error formatter to detect `err.keyword === 'enum'` and emit messages like `Invalid value at <path>: "<value>". Valid values: <list>. Fix: fspec update-foundation <section> "<valid-value>"`. This prevents the "Missing required" misleading error from surfacing on enum failures.

### Priority 6 (Optional): `list-foundation-sections` Command

First-class discovery for section names. Can be implemented as either a standalone command or as `update-foundation --list-sections`.

---

## Lessons Learned

### For Tool Designers

1. **Validate at the point of entry, not the point of commit.** If a field has an enum, check it when it's written, not hours later at finalization. This is the difference between a 5-second fix and a 35-minute debugging session.

2. **Expose working state to agents.** LLMs (especially weaker ones) navigate via observe-act-observe loops. Hiding state forces workarounds — in this case, reading files directly, which is exactly what we want agents to avoid.

3. **Error messages should offer next actions.** "Failed to create draft - draft already exists" is a diagnosis, not a next step. Every error should end with "here's what to do next."

4. **Help text should enumerate, not abbreviate.** "etc." is invisible to an LLM that hasn't seen it before.

5. **Read your own source before concluding the tool is broken.** It's embarrassing how often user-facing bug reports diagnose the wrong cause because the reporter didn't verify against the code.

### For Users (Updated Workaround Guide)

Until the fixes ship, here's the reliable workflow:

```bash
# 1. Create draft
fspec discover-foundation

# 2. Populate draft — MAKE SURE projectType is a valid enum value
fspec update-foundation projectName "My Project"
fspec update-foundation projectVision "One-sentence vision"
fspec update-foundation projectType "web-app"  # MUST be one of:
                                                # web-app, cli-tool, library, sdk,
                                                # mobile-app, desktop-app, service, api, other
fspec update-foundation problemTitle "Short problem title"
fspec update-foundation problemDefinition "Full problem description"
fspec update-foundation solutionOverview "Solution description"

fspec add-capability "Capability Name" "Description"
fspec add-persona "Persona Name" "Description" --goal "Goal 1" --goal "Goal 2"

# 3. Finalize (this actually does everything — write, delete draft, create work unit, generate MD)
fspec discover-foundation --finalize
```

**Key rule:** Never invent values for `projectType`. Read the system-reminder carefully when it appears — it lists the 9 valid values explicitly.

---

## Conclusion

The fspec foundation workflow works correctly in the happy path. The bugs are in the **feedback-loop density**: invalid values aren't caught until the end, working state isn't observable mid-workflow, error messages don't suggest next actions, and help text is vague about section names. Fixing these six items (prioritized above) should reduce the time-to-foundation for a weaker LLM from "35 minutes of confusion" to "5 minutes of deterministic work."

The draft/final file split is not the problem, and `--finalize` is not misnamed. The problem is that the tool trusts its inputs too much during discovery and only validates at the end, and when validation fails, the error message degrades useful information (enum violations → "missing required").

**Bottom line:** This is a validation-timing bug and a feedback-quality bug, not a workflow-design bug. The fixes are small, surgical, and high-impact for weaker LLMs.

---

*Document originally written after 35 minutes of foundation workflow confusion, then corrected against the source code in `src/commands/discover-foundation.ts`, `src/commands/update-foundation.ts`, `src/commands/show-foundation.ts`, and `src/schemas/generic-foundation.schema.json`.*
