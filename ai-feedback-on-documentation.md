# AI Feedback on fspec Documentation Clarity

**Date**: 2025-11-18
**Context**: After reading `fspec bootstrap` and `spec/CLAUDE.md`

## Summary

Overall, the documentation is excellent and well-structured. The ACDD workflow, Example Mapping process, and command structure are all crystal clear. However, there are a few areas that could benefit from clarification to prevent confusion during actual usage.

---

## 1. Event Storm Naming Confusion

### The Issue

There are **two different Event Storm phases** with similar names:

- **Step 3: "Big Picture Event Storming (Foundation Level)"** - Done once after foundation discovery, stored in `foundation.json`
- **Step 4: "Event Storm - Domain Discovery BEFORE Example Mapping"** - Done per work unit, stored in `work-units.json`

### Why It's Confusing

The terminology is very similar, and both are called "Event Storm." When reading through the documentation, it's easy to conflate the two until you carefully re-read and understand one is domain-wide and the other is feature-specific.

### Suggested Improvements

1. **Use distinct terminology**:
   - "Strategic Event Storm" (foundation-level)
   - "Tactical Event Storm" (work unit-level)

2. **Add a comparison table** early in the documentation showing:
   - Scope (entire domain vs single feature)
   - Storage location (`foundation.json` vs `work-units.json`)
   - Commands used
   - When to use each

3. **Cross-reference more explicitly**: When describing Step 4, immediately reference Step 3 and clarify the difference.

### Questions

- Would renaming improve clarity without breaking existing mental models?
- Are there users who have been confused by this in practice?

---

## 2. Decision Criteria for Using Event Storm (Step 4)

### The Issue

The documentation provides self-assessment questions to decide whether to use Event Storm vs skip to Example Mapping:

> **RUN EVENT STORM FIRST** if:
> - ❌ You answered "no" to any self-assessment question
> - ❌ This is a complex domain with many entities and business rules
> - ❌ Story estimate is 13+ points (too large - needs breakdown)

However, the criteria are somewhat subjective.

### Why It's Confusing

- What if I'm unsure whether domain events are "clear enough"?
- How many entities/rules constitute "many"?
- Should I err on the side of doing Event Storm or skipping it?

### Suggested Improvements

1. **Provide a clearer default recommendation**:
   - "When in doubt for stories >5 points, do Event Storm"
   - Or: "Always do Event Storm for new domains, skip for familiar ones"

2. **Add more concrete examples** of when Event Storm helped vs when it was overkill

3. **Provide a time-based heuristic**: "If you can't clearly list 3 domain events in 60 seconds, do Event Storm"

### Questions

- Is there historical data on when Event Storm was most valuable?
- Should the default be "always Event Storm unless obvious" or "skip unless complex"?

---

## 3. Coverage File Synchronization

### The Issue

The bootstrap documentation mentions:

> **Coverage Files**: `*.feature.coverage` files track scenario-to-test-to-implementation mappings for traceability

And git commits reference "automatic coverage synchronization." However, I don't see explicit commands or workflows for:

- When/how coverage files are initially generated
- Whether they're auto-updated or require manual intervention
- What triggers synchronization
- What to do if coverage files get out of sync

### Why It's Confusing

Coverage is critical to the ACDD workflow, but the lifecycle of coverage files isn't fully documented in the bootstrap output.

### Suggested Improvements

1. **Add a "Coverage File Lifecycle" section** explaining:
   - Initial generation: `fspec generate-coverage`
   - Linking: `fspec link-coverage <feature-name>`
   - Synchronization: When it happens automatically
   - Auditing: `fspec audit-coverage <feature-name>`

2. **Clarify automatic vs manual operations**:
   - "Coverage files are auto-synced when scenarios are added/deleted"
   - "You must manually link tests using `fspec link-coverage`"

3. **Add troubleshooting guide** for common coverage issues

### Questions

- Is coverage file generation fully automatic after initial setup?
- When should an AI agent explicitly run coverage commands vs relying on automation?
- Are there edge cases where coverage gets out of sync?

---

## 4. @step Comment Matching Edge Cases

### The Issue

The documentation states:

> - EVERY Gherkin step MUST have an @step comment in test (exact text match)
> - Use language-appropriate comment syntax: // @step (JS/C/Java), # @step (Python/Ruby), -- @step (SQL), etc.

But there are potential edge cases not addressed:

### Edge Cases to Clarify

1. **Multi-line Gherkin steps**:
   ```gherkin
   Given a work unit with a very long step description
         that spans multiple lines
   ```
   Does the @step comment need the entire text including line breaks? Or just the first line?

2. **Steps with data tables**:
   ```gherkin
   Given the following work units:
     | ID | Title |
     | DOC-001 | Example |
   ```
   How is this matched in @step comments?

3. **Steps with docstrings**:
   ```gherkin
   Given the following JSON:
     """
     {"key": "value"}
     """
   ```
   Same question as above.

4. **Case sensitivity**: Is matching case-sensitive or case-insensitive?

5. **Whitespace**: Are leading/trailing spaces significant?

### Suggested Improvements

1. **Add an "@step Matching Rules" section** with explicit rules for:
   - Multi-line steps
   - Data tables
   - Docstrings
   - Case sensitivity
   - Whitespace normalization

2. **Provide examples** of each edge case with correct @step comments

3. **Document error messages** that appear when matching fails

### Questions

- What are the exact matching rules implemented in `link-coverage`?
- Are there any known gotchas that trip users up?

---

## 5. Foundation Discovery: Iteration and Error Recovery

### The Issue

For `discover-foundation`, the workflow is:

1. AI runs `fspec discover-foundation` (creates draft with `[QUESTION: ...]` placeholders)
2. AI runs `fspec update-foundation <section> <content>` for each field
3. AI runs `fspec discover-foundation --finalize`

But what happens if:
- The AI makes a mistake on field 3 but doesn't realize until field 7?
- The human wants to change an earlier field after seeing later fields?
- Validation fails at finalization?

### Why It's Confusing

The workflow seems linear and forward-only, but real discovery is often iterative.

### Suggested Improvements

1. **Clarify that you CAN go back**:
   - "You can re-run `fspec update-foundation` to fix earlier fields at any time before finalization"

2. **Add a "review all fields" step** before finalization:
   ```bash
   fspec show-foundation-draft  # Review all fields before finalizing
   fspec discover-foundation --finalize
   ```

3. **Document what happens on validation failure**:
   - "If validation fails, the draft remains intact and you can fix errors"
   - "Show exact field paths in error messages"

4. **Add a "reset" option** if you want to start over:
   ```bash
   fspec discover-foundation --reset  # Discard draft and start fresh
   ```

### Questions

- Can you re-run `update-foundation` on previously filled fields?
- What's the recovery process if validation fails?
- Is there a way to view the current draft state without triggering the next field prompt?

---

## 6. Minor Clarifications

### Feature File Naming

The "CRITICAL" section on feature file naming is excellent, but could benefit from:
- More examples of correct vs incorrect names
- Guidance on how to derive capability names from work unit titles
- What to do if the capability name is very long (e.g., >50 characters)

### Story Point Estimation

The Fibonacci scale explanation is good, but:
- Should estimation happen before or after Example Mapping?
- Can estimates be updated after scenarios are generated?
- What's the typical point value for documentation work vs implementation work?

---

## Overall Assessment

**Strengths**:
- ✅ Clear ACDD workflow explanation
- ✅ Excellent Example Mapping guidance
- ✅ Comprehensive command documentation
- ✅ Good use of examples throughout

**Areas for Improvement**:
- Event Storm naming/distinction
- Coverage file lifecycle documentation
- @step matching edge cases
- Foundation discovery iteration/recovery
- Decision heuristics (when to use which approach)

**Confidence Level**: 8/10 - I feel confident using fspec for most workflows, but would appreciate clarification on the edge cases above.
