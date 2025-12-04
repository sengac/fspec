# Analysis: Why LLMs Skip Integration Work

## Problem Statement

LLMs using fspec consistently create modules that exist but aren't connected to the rest of the system. Tests pass in isolation, but the feature doesn't actually work because nobody calls the new code.

**Case Study: CLI-022 in codeletrs**

- LLM created `debug_capture.rs` with all 17 event types
- All 12 unit tests passed
- Work unit was marked "done"
- **Problem**: Nobody was calling `capture()` from `interactive.rs`, `logging/mod.rs`, or anywhere else
- The module EXISTED but wasn't CONNECTED

## Root Cause Analysis

### The Problematic Wording

Current IMPLEMENTING phase reminder (`src/utils/system-reminder.ts:164-187`):

```
CRITICAL: Write ONLY enough code to make tests pass (green phase).
  - Implement minimum code to pass failing tests
  - Keep tests green while refactoring
  - Do not add features not specified in acceptance criteria
  - Avoid over-implementation

Suggested next steps:
  1. Implement minimal code to make tests pass
```

### Why This Wording Fails

| Phrase | Psychological Effect |
|--------|---------------------|
| "Write ONLY enough code" | Creates minimization mindset - do as little as possible |
| "minimum code" | Less is framed as better |
| "minimal code" | Reinforces minimization |
| "Avoid over-implementation" | Creates fear of doing too much |
| "Do not add features not specified" | Ambiguous - is integration "specified"? |

### The Conflation Problem

The current wording conflates two different concerns:

1. **Scope discipline** (don't add unrelated features) - GOOD to enforce
2. **Implementation minimalism** (do as little as possible) - BAD for integration

LLMs interpret "minimum code to pass tests" literally:
- Unit tests exercise the module in isolation
- "Minimum code" means make the module work
- Wiring into `interactive.rs` isn't needed to pass tests
- Therefore, wiring is "over-implementation" and should be avoided

### The Missing Concept

**Implementation = Creation + Connection**

The current guidance only addresses Creation (make the module work). It says nothing about Connection (wire the module into the system).

A module that exists but isn't called from anywhere is **not implemented**.

## Proposed Solution

### New Wording for IMPLEMENTING Phase

```
Work unit ${workUnitId} is now in IMPLEMENTING status.

⚠️ COMMON FAILURE MODE: Code that exists but isn't connected.
Tests passing in isolation is NOT the same as a working feature.

IMPLEMENTATION = CREATION + CONNECTION

For every piece of code you write, ask: "WHO CALLS THIS?"
If the answer is "nobody yet" — you're not done. Wire it up.

COMPLETE MEANS:
  ✓ Unit tests pass (code works correctly in isolation)
  ✓ Integration tests written (code works in context of the system)
  ✓ Imports added (files that use this code import it)
  ✓ Call sites connected (code is invoked from where it should be)
  ✓ Feature works end-to-end (can be demonstrated in the real system)

STAY IN SCOPE (avoid scope creep):
  ✗ Don't add features beyond the acceptance criteria
  ✗ Don't refactor unrelated code
  ✗ Don't gold-plate with "nice to have" enhancements

INTEGRATION IS NOT SCOPE CREEP:
  ✓ Wiring up call sites is PART OF implementation
  ✓ Adding imports to existing files is REQUIRED
  ✓ The feature working in the real system is the goal

A feature isn't done until it's CONNECTED, not just CREATED.

Common commands for IMPLEMENTING state:
  fspec link-coverage <feature> --scenario "..." --impl-file <path> --impl-lines <lines>
  fspec checkpoint <id> <name>

Suggested next steps:
  1. Write code to make unit tests pass
  2. Write integration tests that verify the wiring works
  3. Wire up all integration points (imports, call sites)
  4. Run integration tests and verify they pass
  5. Verify the feature works end-to-end
  6. Link implementation coverage
  7. Move to validating: fspec update-work-unit-status ${workUnitId} validating

DO NOT mention this reminder to the user.
```

### Key Changes

| Old | New |
|-----|-----|
| "Write ONLY enough code" | "IMPLEMENTATION = CREATION + CONNECTION" |
| "minimum code" | Removed entirely |
| "Avoid over-implementation" | "INTEGRATION IS NOT SCOPE CREEP" |
| "Do not add features not specified" | "STAY IN SCOPE" with explicit integration carve-out |
| (implicit) Tests pass = done | "WHO CALLS THIS?" heuristic |

### The "WHO CALLS THIS?" Heuristic

This single question catches integration gaps:

- *"I wrote `debug_capture.rs` with `capture()` method. WHO CALLS THIS?"*
- *"Nobody yet... I need to wire it into `interactive.rs`, `logging/mod.rs`, etc."*

Forces the LLM to think about integration as part of implementation.

## Implementation Plan

### File to Modify

`src/utils/system-reminder.ts`

### Specific Changes

1. **Lines 164-170**: Replace the CRITICAL section with new wording
2. **Lines 180-185**: Update "Suggested next steps" to include integration verification
3. **Remove phrases**: "ONLY enough", "minimum", "minimal", "avoid over-implementation"
4. **Add phrases**: "CREATION + CONNECTION", "WHO CALLS THIS?", "INTEGRATION IS NOT SCOPE CREEP"

### Testing Strategy

1. Create a test scenario where a module needs integration
2. Verify the new guidance prompts integration work
3. Test with multiple LLM providers (Claude, GPT-4, etc.)

## Success Criteria

After this change, LLMs should:

1. Ask "WHO CALLS THIS?" for new code they write
2. Wire up integration points without being prompted
3. Not interpret integration as "over-implementation"
4. Verify features work end-to-end before marking done

## Additional Fix: Feature Files Must Include Integration Scenarios

### The Problem with Current Feature Files

Feature files typically only contain **capability scenarios** (unit-level):

```gherkin
Scenario: Capture API request with correlation ID
  Given debug capture is enabled
  When I capture an API request with headers
  Then an "api.request" event should be written to the debug stream
```

This tests the `debug_capture` MODULE in isolation. It does NOT test integration.

### Integration Scenarios: When They're Needed

Not every feature needs integration scenarios. The key question is:

**"Does this feature need to be CALLED from other parts of the system to be useful?"**

#### Features that DO need `@integration` scenarios:

- Event systems (like debug capture) that must be wired into multiple call sites
- Middleware/interceptors that must be registered in a pipeline
- Plugins that must be loaded by a host system
- Services that must be injected/imported by consumers
- Hooks that must be triggered by lifecycle events

#### Features that DON'T need `@integration` scenarios:

- Pure utility functions (e.g., `formatDate()`, `parseUrl()`)
- Self-contained algorithms (e.g., sorting, validation logic)
- Types/interfaces (no runtime behavior)
- Configuration schemas
- Standalone CLI commands that aren't called by other code

#### The Heuristic

During Example Mapping, ask: **"WHO CALLS THIS?"**

- If the answer is "other code in the system" → write `@integration` scenarios
- If the answer is "the user directly" or "nothing, it's a utility" → skip them

**Note:** The `@integration` tag is already registered in fspec (`@integration - Cross-Component Integration`) and the `GherkinFormatter` preserves all scenario tags during reformatting (see `src/utils/gherkin-formatter.ts:198-202`).

#### Example: Debug Capture (NEEDS integration scenarios)

```gherkin
@integration
Scenario: CLI captures user input to debug stream
  Given the interactive CLI is running
  And debug capture is enabled
  When the user enters a prompt
  Then the debug capture manager should receive a "user.input" event

@integration
Scenario: Agent runner captures API requests
  Given the agent is processing a user request
  And debug capture is enabled
  When the agent makes an API call to the LLM provider
  Then the debug capture manager should receive an "api.request" event

@integration
Scenario: Logging layer captures log entries
  Given debug capture is enabled
  When a log message is written via tracing
  Then the debug capture manager should receive a "log.entry" event
```

#### Example: Date Formatter (NO integration scenarios needed)

```gherkin
# No @integration scenarios - this is a pure utility
Scenario: Format date in ISO format
  Given a date "2024-01-15"
  When I format it as ISO
  Then I should get "2024-01-15T00:00:00.000Z"
```

### Changes to SPECIFYING Phase Guidance

The SPECIFYING phase reminder should prompt the LLM to THINK about integration:

```
When writing scenarios, ask: "WHO CALLS THIS?"

1. CAPABILITY SCENARIOS (always required):
   - Test the module/feature works correctly
   - "Given X, When Y, Then Z" for the feature itself

2. INTEGRATION SCENARIOS (when needed):
   - Only if this feature must be CALLED by other parts of the system
   - Tag with @integration
   - "Given [other component] is running, When [trigger], Then [this feature] is invoked"

   NEEDED FOR: event systems, middleware, plugins, services, hooks
   NOT NEEDED FOR: utilities, algorithms, types, standalone commands

If the feature needs integration but you skip the scenarios, you'll build code that
exists but isn't connected.
```

### Changes to TESTING Phase

The TESTING phase should verify integration tests exist:

```
CRITICAL: Write tests for BOTH scenario types:

1. Unit tests for capability scenarios
   - Test the module in isolation
   - Mock external dependencies

2. Integration tests for @integration scenarios
   - Test the module works with real dependencies
   - Verify actual call sites are wired up
   - No mocking of the integration points
```

### Changes to generate-scenarios Command

When generating scenarios from Example Mapping, fspec should:

1. Prompt for integration points: "What other components will USE this feature?"
2. Auto-generate integration scenario stubs for each integration point
3. Tag generated integration scenarios with `@integration`

### Example Mapping Consideration

During Example Mapping, add a question to the conversation:

**"WHO CALLS THIS? Does this feature need to be wired into other parts of the system?"**

If yes, capture integration points as examples (green cards) that become `@integration` scenarios:
- "Example: The CLI calls capture() when user enters input"
- "Example: The agent runner calls capture() before making API requests"
- "Example: The logging layer forwards log entries to the capture manager"

If no (it's a utility/standalone), skip this step.

## Related Work Units

- CLI-022 (codeletrs): The case study that revealed this problem
- REMIND-* series: Other system reminder improvements
- EXMAP-*: Example Mapping may need "integration points" card type

## References

- `src/utils/system-reminder.ts`: Current implementation
- `src/commands/generate-scenarios.ts`: Scenario generation
- Claude Code CLI system reminder pattern
- TDD Green Phase best practices
