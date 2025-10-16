# Project Management

fspec provides Kanban workflow with work units and epics for ACDD (Acceptance Criteria Driven Development).

## Work Units and Epics

```bash
# Create and manage work units
fspec create-work-unit AUTH "User login feature"
fspec create-work-unit DASH "Dashboard view" -e user-management
fspec list-work-units
fspec list-work-units -s specifying
fspec show-work-unit AUTH-001

# Create and manage epics
fspec create-epic user-management "User Management Features"
fspec list-epics
fspec show-epic user-management
```

## Kanban Workflow States

Work units flow through a 7-state Kanban workflow:

- **Normal flow:** `backlog` → `specifying` → `testing` → `implementing` → `validating` → `done`
- **Blocking:** Any state can transition to `blocked` when work cannot proceed
- **Phase enforcement:** Cannot skip states (e.g., can't jump from specifying directly to implementing)
- **Temporal ordering:** Files must be created AFTER entering required state (prevents retroactive completion). Use `--skip-temporal-validation` for reverse ACDD or importing existing work.

### Moving Work Through States

```bash
# Move work unit through workflow
fspec update-work-unit-status AUTH-001 specifying
fspec update-work-unit-status AUTH-001 testing
fspec update-work-unit-status AUTH-001 implementing
fspec update-work-unit-status AUTH-001 validating
fspec update-work-unit-status AUTH-001 done

# Block work unit with reason
fspec update-work-unit-status AUTH-001 blocked --blocked-reason "Waiting for API documentation"

# Skip temporal validation (for reverse ACDD or importing existing work)
fspec update-work-unit-status LEGACY-001 testing --skip-temporal-validation
```

## Visualize Work

```bash
# Display Kanban board showing all work units across states
fspec board

# Display board with custom item limit per column (default: 25)
fspec board --limit=50

# Export board as JSON for programmatic access
fspec board --format=json
```

## Example Mapping (Discovery)

Before writing specifications, use example mapping to clarify requirements:

```bash
# Add rules (blue cards)
fspec add-rule AUTH-001 "Password must be 8+ characters"
fspec add-rule AUTH-001 "Email must be valid format"

# Add examples (green cards)
fspec add-example AUTH-001 "User logs in with valid email"
fspec add-example AUTH-001 "User login fails with invalid password"

# Add questions (red cards)
fspec add-question AUTH-001 "@human: Should we support OAuth?"

# Answer questions
fspec answer-question AUTH-001 0 --answer "Yes, OAuth 2.0 via Google" --add-to rule

# Add assumptions
fspec add-assumption AUTH-001 "Email verification handled by separate service"

# Add architecture notes
fspec add-architecture-note AUTH-001 "Uses JWT tokens for session management"

# Attach supporting files
fspec add-attachment AUTH-001 diagrams/auth-flow.png --description "Authentication flow diagram"
fspec list-attachments AUTH-001
fspec remove-attachment AUTH-001 auth-flow.png

# Generate Gherkin scenarios from example map
fspec generate-scenarios AUTH-001
```

## Story Point Estimation

```bash
# Set estimate for work unit (Fibonacci: 1, 2, 3, 5, 8, 13)
fspec update-work-unit-estimate AUTH-001 5

# Query estimation accuracy
fspec query-estimate-accuracy

# Get estimation guidance
fspec query-estimation-guide AUTH-001
```

## Dependencies

```bash
# Add dependency (shorthand)
fspec add-dependency AUTH-002 AUTH-001  # AUTH-002 depends on AUTH-001

# Add dependency (explicit)
fspec add-dependency AUTH-002 --depends-on=AUTH-001
fspec add-dependency AUTH-002 --blocks=API-001
fspec add-dependency UI-001 --blocked-by=API-001
fspec add-dependency AUTH-001 --relates-to=SEC-001

# Remove dependency
fspec remove-dependency AUTH-002 AUTH-001
fspec remove-dependency AUTH-002 --blocks=API-001

# Clear all dependencies
fspec clear-dependencies AUTH-002

# Show dependencies
fspec dependencies AUTH-002

# Export dependency graph
fspec export-dependencies --format=mermaid
fspec export-dependencies --format=json
```

## Metrics and Reporting

```bash
# Record metrics
fspec record-metric AUTH-001 time-spent 120
fspec record-tokens AUTH-001 15000
fspec record-iteration AUTH-001

# Query metrics
fspec query-metrics --format=json
fspec query-metrics --metric time-spent

# Generate summary report
fspec generate-summary-report --format=markdown --output=report.md
```

## Data Storage

- Work units are stored in `spec/work-units.json`
- Epics are stored in `spec/epics.json`

For complete command documentation, run:
```bash
fspec help work
fspec help discovery
fspec help metrics
```

## Next Steps

- [Getting Started](./getting-started.md) - Learn the ACDD workflow
- [Coverage Tracking](./coverage-tracking.md) - Link scenarios to tests and implementation
- [Lifecycle Hooks](./hooks/configuration.md) - Automate your workflow
