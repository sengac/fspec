# Usage Guide

Complete reference for all fspec commands and workflows.

## Feature File Management

### Creating and Managing Features

```bash
# Create new feature file
fspec create-feature "User Authentication"

# Add scenario to existing feature
fspec add-scenario user-authentication "Login with valid credentials"

# Add step to existing scenario
fspec add-step user-authentication "Login with valid credentials" given "I am on the login page"

# Update scenario name
fspec update-scenario user-authentication "Old Name" "New Name"

# Update step in scenario
fspec update-step user-authentication "Login with valid credentials" "I am on the login page" --text "I navigate to the login page"
fspec update-step user-authentication "Login with valid credentials" "I am on the login page" --keyword When

# Delete step from scenario
fspec delete-step user-authentication "Login with valid credentials" "I am on the login page"

# Delete scenario from feature
fspec delete-scenario user-authentication "Login with valid credentials"

# List all features
fspec list-features

# Filter by tag
fspec list-features --tag=@phase1

# Show specific feature
fspec show-feature user-authentication
fspec show-feature user-authentication --format=json
fspec show-feature user-authentication --output=feature.json
```

### Feature File Documentation

```bash
# Add or update architecture notes in feature file
fspec add-architecture user-authentication "Uses JWT tokens for session management"

# Add or update user story (Background) in feature file
fspec add-background user-authentication "As a user\nI want to log in securely\nSo that I can access my account"
```

## Query Operations

```bash
# Get all scenarios matching tags (supports feature-level AND scenario-level tags)
fspec get-scenarios --tag=@phase1
fspec get-scenarios --tag=@phase1 --tag=@critical  # AND logic
fspec get-scenarios --tag=@smoke  # Matches scenario-level tags
fspec get-scenarios --format=json

# Show acceptance criteria for features
fspec show-acceptance-criteria --tag=@phase1
fspec show-acceptance-criteria --tag=@phase1 --format=markdown
fspec show-acceptance-criteria --tag=@phase1 --format=json --output=phase1-acs.md

# Bulk delete scenarios by tag
fspec delete-scenarios-by-tag --tag=@deprecated
fspec delete-scenarios-by-tag --tag=@phase1 --tag=@wip  # AND logic
fspec delete-scenarios-by-tag --tag=@deprecated --dry-run  # Preview deletions

# Bulk delete feature files by tag
fspec delete-features-by-tag --tag=@deprecated
fspec delete-features-by-tag --tag=@phase1 --tag=@wip  # AND logic
fspec delete-features-by-tag --tag=@deprecated --dry-run  # Preview deletions
```

## Formatting & Validation

```bash
# Format all feature files
fspec format

# Format specific file
fspec format spec/features/login.feature

# Run all validation checks (Gherkin syntax, tags, formatting)
fspec check
fspec check --verbose
```

## Architecture Documentation

fspec uses **JSON-backed documentation** where `spec/foundation.json` and `spec/tags.json` serve as the single source of truth. The `FOUNDATION.md` and `TAGS.md` files are automatically generated from their JSON counterparts.

### Foundation Management (JSON-Backed)

All foundation operations work with `spec/foundation.json` and automatically regenerate `spec/FOUNDATION.md`:

```bash
# Add or update Mermaid diagram (with automatic syntax validation)
fspec add-diagram "Architecture Diagrams" "System Context" "graph TD\n  A[User] --> B[API]\n  B --> C[Database]"

# Mermaid validation catches syntax errors before adding
# Example error: "Invalid Mermaid syntax: Parse error on line 3..."

# Delete Mermaid diagram
fspec delete-diagram "Architecture Diagrams" "System Context"

# Update foundation section content
fspec update-foundation "What We Are Building" "A CLI tool for managing Gherkin specifications"

# Display foundation content
fspec show-foundation
fspec show-foundation --section "What We Are Building"
fspec show-foundation --format=json
fspec show-foundation --format=markdown --output=foundation-copy.md
fspec show-foundation --list-sections
fspec show-foundation --line-numbers
```

**Note:** All write operations (add-diagram, delete-diagram, update-foundation) modify `spec/foundation.json` and automatically regenerate `spec/FOUNDATION.md`. Never edit the markdown files directly.

## Development Validation

### Validate fspec's Own Specs

fspec "eats its own dog food" - it manages its own specifications:

```bash
# Validate all fspec feature files
fspec validate

# Validate specific feature
fspec validate spec/features/gherkin-validation.feature

# Validate all tags are registered
fspec validate-tags

# Format all feature files
fspec format

# Show tag statistics
fspec tag-stats
```

## Next Steps

- [Tag Management](./tags.md) - Organize features with tags
- [Coverage Tracking](./coverage-tracking.md) - Link scenarios to tests and implementation
- [Project Management](./project-management.md) - Kanban workflow and work units
- [Lifecycle Hooks](./hooks/configuration.md) - Automate your workflow
