//! fspec Workflow Guidance for System Prompts
//!
//! This module contains the core ACDD workflow guidance that gets injected
//! into the system prompt BEFORE any project-specific content (like AGENTS.md).
//!
//! The guidance teaches LLMs how to use fspec for Acceptance Criteria Driven Development.

/// Core fspec ACDD workflow guidance for system prompts
///
/// This constant is injected into every system prompt when fspec tooling is available.
/// It appears BEFORE project-specific content like AGENTS.md.
pub const FSPEC_WORKFLOW_GUIDANCE: &str = r#"<system-reminder>
<!-- type:fspecWorkflow -->
# fspec - Acceptance Criteria Driven Development (ACDD)

## CRITICAL: Use the Fspec Tool

**ALWAYS use the `Fspec` tool for ALL fspec operations.** Do NOT run fspec CLI commands via Bash.

The Fspec tool provides:
- Work unit management (create, update, list, show)
- Example Mapping (rules, examples, questions)
- Feature file management (scenarios, steps)
- Workflow automation (board, status updates)

**Example Fspec tool usage:**
```
command: "list-work-units"
args: {"status": "backlog"}
```

```
command: "show-work-unit"
args: {"_": ["AUTH-001"]}
```

```
command: "update-work-unit-status"
args: {"_": ["AUTH-001", "specifying"]}
```

Use `command: "help"` to get detailed documentation on all available commands.

## ACDD Workflow (MANDATORY ORDER)

```
BACKLOG → SPECIFYING → TESTING → IMPLEMENTING → VALIDATING → DONE
                            ↓
                        BLOCKED (with reason)
```

### Phase 0: DISCOVERY (Before Specifying)
Use Example Mapping to clarify requirements:
- Ask questions one by one to build shared understanding
- Capture rules (business rules), examples (concrete scenarios), questions (unknowns)
- Stop when no more questions remain and scope is clear

**Fspec commands:** `add-rule`, `add-example`, `add-question`, `answer-question`

### Phase 1: SPECIFYING
Write Gherkin feature file (acceptance criteria):
- Define user story, scenarios, and steps based on example map
- Transform examples from discovery into concrete scenarios

**Fspec commands:** `create-feature`, `add-scenario`, `add-step`, `generate-scenarios`

### Phase 2: TESTING
Write failing tests BEFORE any code:
- Create test file with header comment linking to feature file
- Map test scenarios to Gherkin scenarios using @step comments
- EVERY Gherkin step MUST have an @step comment in test
- Tests MUST fail (red phase) - proving they test real behavior

### Phase 3: IMPLEMENTING
Write code AND wire up integration points:
- IMPLEMENTATION = CREATION + CONNECTION
- Ask "WHO CALLS THIS?" - wire up all call sites
- Tests MUST pass AND feature must work end-to-end

### Phase 4: VALIDATING
Ensure all quality checks pass:
- Run ALL tests (not just new ones) to ensure nothing broke
- Run quality checks: typecheck, lint, format
- Validate Gherkin syntax and tag compliance

### Phase 5: DONE
Complete and update kanban:
- Move work unit to done column
- Update feature file tags (@wip → @done)

## Starting a Session

1. **Check the board:** `Fspec command: "board"`
2. **Pick work from backlog:** `Fspec command: "show-work-unit" args: {"_": ["WORK-001"]}`
3. **Move to specifying:** `Fspec command: "update-work-unit-status" args: {"_": ["WORK-001", "specifying"]}`
4. **Follow ACDD phases in order**

## Key Rules

- **Never skip phases** - Each phase validates the previous
- **Move backward when needed** - If tests reveal spec gaps, go back to specifying
- **Use Fspec tool, not Bash** - The tool handles JSON formatting and validation
- **One work unit at a time** - Focus on completing before starting new work

</system-reminder>
"#;

/// Get the fspec workflow guidance for injection into system prompts
pub fn get_fspec_workflow_guidance() -> &'static str {
    FSPEC_WORKFLOW_GUIDANCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guidance_contains_acdd_workflow() {
        let guidance = get_fspec_workflow_guidance();
        assert!(guidance.contains("ACDD"));
        assert!(guidance.contains("BACKLOG"));
        assert!(guidance.contains("SPECIFYING"));
        assert!(guidance.contains("IMPLEMENTING"));
    }

    #[test]
    fn test_guidance_contains_fspec_tool_instruction() {
        let guidance = get_fspec_workflow_guidance();
        assert!(guidance.contains("ALWAYS use the `Fspec` tool"));
        assert!(guidance.contains("Do NOT run fspec CLI commands via Bash"));
    }

    #[test]
    fn test_guidance_is_wrapped_in_system_reminder() {
        let guidance = get_fspec_workflow_guidance();
        assert!(guidance.starts_with("<system-reminder>"));
        assert!(guidance.contains("</system-reminder>"));
    }

    #[test]
    fn test_guidance_contains_example_tool_usage() {
        let guidance = get_fspec_workflow_guidance();
        assert!(guidance.contains("list-work-units"));
        assert!(guidance.contains("show-work-unit"));
        assert!(guidance.contains("update-work-unit-status"));
    }
}
