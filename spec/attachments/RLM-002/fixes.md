# RLM-002 Review Fixes

## 🔴 CRITICAL ISSUES

### 1. `test_default_max_recursion_depth` is a no-op — doesn't test the scenario

**Feature file (line 56-59):**
```gherkin
Scenario: Default max recursion depth is 2
  Given a parent agent calls DeepSearch without specifying max_recursion_depth
  When the sub-agent is constructed
  Then max_recursion_depth defaults to 2
```

**What the test actually does (recursive_tests.rs:75-94):**
```rust
let args: DeepSearchArgs = serde_json::from_value(json).unwrap();
assert!(args.max_depth.is_none(), ...);  // ← tests max_DEPTH, not max_RECURSION_depth!
```

The test checks `max_depth` (tool-call rounds) is `None` — a completely unrelated field. It **never asserts `DEFAULT_MAX_RECURSION_DEPTH == 2`**. And it has a TODO comment admitting this:
> *"This test will be updated when max_recursion_depth is added to DeepSearchArgs."*

**Fix:** Assert `DEFAULT_MAX_RECURSION_DEPTH == 2` directly. Remove the TODO.

---

### 2. `test_system_prompt_without_recursion` doesn't verify the negative assertion

**Feature file (line 118-122):**
```gherkin
Scenario: System prompt omits DeepSearch at max recursion depth
  ...
  Then the prompt does NOT include DeepSearch in the AVAILABLE TOOLS section
```

**What the test does:**
```rust
// @step Then the prompt does NOT include DeepSearch in the AVAILABLE TOOLS section
// Current behavior: prompt doesn't mention DeepSearch (correct for base case).
//                   ^^^^^^^ Comment with NO assertion! ^^^^^^^
```

The `@step` comment exists but is followed by prose and zero assertions.

**Fix:** Add `assert!(!prompt.contains("DeepSearch"), ...)`.

---

### 3. System prompt formatting bug — DeepSearch misaligned in AVAILABLE TOOLS

The rendered prompt has DeepSearch at **10 spaces indent** while all other tools are at indent 0:

```
- SessionSearch: Search and view session conversation history (use recent/search/show actions)
          - DeepSearch: Spawn a recursive sub-agent...
```

Bug is in `mod.rs:201`:
```rust
"\n          - DeepSearch: Spawn a recursive sub-agent..."
//  ^^^^^^^^^^  these 10 spaces are literal in the output
```

**Fix:** Change to `"\n- DeepSearch: ..."` with zero indent.

---

### 4. System prompt doesn't teach Bash/python3 chunking pattern (Rule [5], Example [2])

**Rule [5]:** *"The system prompt must teach the decompose-delegate-aggregate strategy from the RLM paper: explore scope, **chunk via Bash/python3**, delegate via recursive DeepSearch, aggregate results"*

**Example [2]:** *"DeepSearch at depth=0 uses Bash to run python3 -c to split a file list into chunks, then calls DeepSearch once per chunk"*

The actual RECURSIVE DECOMPOSITION STRATEGY section has **zero mention** of using Bash/python3 for programmatic chunking. The entire RLM paper insight — using Bash to programmatically orchestrate sub-calls — is absent.

**Fix:** Add Bash/python3 chunking guidance to the recursion strategy section of the prompt.

---

## 🟡 DRY VIOLATION

### 5. `build_and_run!` macro duplicates execution logic

The if/else branches in the macro duplicate the entire agent execution (streaming/non-streaming) — 8 lines × 2 branches × 5 providers = 80 lines of duplication.

**Fix:** Extract a `run_agent!` helper macro for the execution path.

---

## 🟡 MINOR ISSUE

### 6. `max_recursion_depth` not exposed as configurable parameter

The rules say "**Default** max_recursion_depth is 2" — implying configurability. But `DeepSearchArgs` has no `max_recursion_depth` field. It's hardcoded to `DEFAULT_MAX_RECURSION_DEPTH` in `session_manager.rs`.

The feature file only specifies what happens when NOT specified (defaulting to 2), so technically compliant with scenarios-as-written. But the TODO comment in the test shows this was intended to be configurable.

**Fix:** Add `max_recursion_depth` optional field to `DeepSearchArgs`, default to `DEFAULT_MAX_RECURSION_DEPTH` when not specified.
