export function getEstimationSection(): string {
  return `## Step 6: Story Point Estimation (After Generating Scenarios)

**CRITICAL**: After generating scenarios from Example Mapping, you MUST estimate story points based on feature file complexity to help with prioritization and velocity tracking.

**Workflow Order**: Example Mapping → Generate Scenarios → Estimate

### Story Point Scale (Fibonacci Sequence)

Use the Fibonacci sequence for estimation to reflect increasing uncertainty at larger sizes:

- **1 point** - Trivial (< 30 minutes)
  - Simple text changes, documentation updates
  - Adding a tag, updating a work unit description
  - Running existing commands to verify something
  - Example: "Update README with new command"

- **2 points** - Simple (30 min - 1 hour)
  - Small feature additions following known patterns
  - Basic validation or formatting logic
  - Single file changes with clear requirements
  - Example: "Add new tag category to registry"

- **3 points** - Moderate (1-2 hours)
  - Medium features with some complexity
  - Multiple file changes with clear integration points
  - Writing tests + implementation for straightforward features
  - Example: "Add new fspec command with 2-3 options"

- **5 points** - Complex (2-4 hours)
  - Complex features requiring some research or experimentation
  - Multiple integrated components with dependencies
  - New architectural patterns or significant refactoring
  - Example: "Implement dependency graph visualization"

- **8 points** - Very Complex (4-8 hours)
  - Major features with multiple unknowns
  - Significant refactoring affecting multiple systems
  - Integration with external APIs or libraries
  - Example: "Add CI/CD pipeline with multiple stages"

- **13 points** - Large (8+ hours)
  - Upper limit for single work units
  - Acceptable but at the edge of complexity
  - Consider breaking down if approaching this size

- **21+ points** - Epic (very large)
  - **TOO LARGE** - MUST break down into smaller work units (1-13 points each)
  - If a story is 21 points, it's actually multiple stories
  - Use Example Mapping to identify natural split points
  - Create parent work unit with dependencies between child units
  - **AUTOMATIC WARNING**: When you estimate story/bug > 13 points, \`fspec show-work-unit\` displays a system-reminder warning with step-by-step guidance for breaking down the work unit
  - Warning persists until estimate ≤ 13 or status = done
  - Tasks are exempt from this warning (can be legitimately large)

### How to Estimate Story Points

**Ask yourself these questions after generating scenarios from Example Mapping:**

1. **Scope Clarity**: Do I fully understand what needs to be built?
   - Clear requirements → Lower points
   - Many unknowns → Higher points

2. **File Impact**: How many files will I need to create/modify?
   - 1 file → 1-2 points
   - 2-3 files → 2-3 points
   - 4-6 files → 3-5 points
   - 7+ files → 5-8 points (or split the story)

3. **Dependencies**: Are there blockers or external dependencies?
   - No blockers → Estimate as-is
   - Blocked by other work → Add to \`dependsOn\` relationship
   - External API/library → +2-3 points for integration complexity

4. **Familiarity**: Have I done something similar before?
   - Familiar pattern → Lower points
   - New technology/approach → Higher points

5. **Testing Requirements**: What test coverage is needed?
   - No tests (documentation) → Points as-is
   - Unit tests only → +1 point
   - Integration tests → +2 points
   - E2E tests → +3 points

6. **Risk**: What could go wrong?
   - Low risk (well-understood) → Lower points
   - High risk (many edge cases) → Higher points

### Setting the Estimate

**After Example Mapping, immediately set the estimate:**

\`\`\`bash
# Estimate based on your analysis
fspec update-work-unit-estimate <work-unit-id> <points>

# Example:
fspec update-work-unit-estimate EXAMPLE-006 3
\`\`\`

### Re-estimation Triggers

**You MUST re-estimate if:**

1. **Scope changes** during implementation (discovered hidden complexity)
2. **Blockers appear** that weren't anticipated
3. **Example Mapping reveals** the story is larger/smaller than initially thought
4. **After testing phase** if implementation was much easier/harder than expected

\`\`\`bash
# Update estimate when scope changes
fspec update-work-unit-estimate EXAMPLE-006 5  # Was 3, now 5 due to complexity
\`\`\`

### Estimation Anti-Patterns (AVOID THESE)

❌ **Don't estimate in hours** - Use relative story points (Fibonacci)
❌ **Don't estimate without Example Mapping** - You'll be wildly inaccurate
❌ **Don't skip estimation** - Velocity tracking requires estimates
❌ **Don't let stories > 13 points exist** - Always break them down (13 points is acceptable, 21+ is too large)
❌ **Don't estimate in a vacuum** - Use Example Mapping to inform estimates

### Estimation Best Practices

✅ **Estimate after Example Mapping** - Use rules/examples/questions to inform size
✅ **Compare to previous work** - "Is this bigger or smaller than EXAMPLE-006?"
✅ **When in doubt, round up** - It's better to overestimate slightly
✅ **Track actual vs estimated** - Use \`fspec query-estimate-accuracy\` to improve
✅ **Break down large stories** - Stories > 13 points = multiple work units (13 is acceptable, 21+ must be split)
✅ **Re-estimate when scope changes** - Keep estimates accurate throughout

### Example Estimation Flow

\`\`\`bash
# 1. After Example Mapping
fspec show-work-unit EXAMPLE-006
# Review: 3 rules, 5 examples, 1 question answered
# Analysis: 2 files to modify, familiar patterns, unit tests needed
# Decision: 3 points (1-2 hours)

fspec update-work-unit-estimate EXAMPLE-006 3

# 2. During implementation (scope change discovered)
# Found: Need to refactor existing code + add integration tests
# Re-analysis: Now 4-5 files, integration complexity, more tests
# Decision: Re-estimate to 5 points

fspec update-work-unit-estimate EXAMPLE-006 5
\`\`\`

### Velocity Tracking

**Once you have estimates, track velocity:**

\`\`\`bash
# Check estimation accuracy
fspec query-estimate-accuracy

# See velocity trends
fspec query-metrics --format=json

# Get estimation guidance based on history
fspec query-estimation-guide EXAMPLE-006
\`\`\`

**Reference**: Story points help with sprint planning and predicting completion dates. Track your velocity over time to improve accuracy.

`;
}
