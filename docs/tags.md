# Tag Management

fspec provides comprehensive tag management at both feature and scenario levels, backed by a JSON registry.

## Tag Registry (JSON-Backed)

All tag operations work with `spec/tags.json` and automatically regenerate `spec/TAGS.md`:

### Registry Management

```bash
# Register new tag
fspec register-tag @performance "Technical Tags" "Performance-critical features"

# Update existing tag
fspec update-tag @performance --description="Updated description"
fspec update-tag @performance --category="Technical Tags"
fspec update-tag @performance --category="Technical Tags" --description="New description"

# Validate all tags are registered
fspec validate-tags

# List all registered tags
fspec list-tags

# Filter tags by category
fspec list-tags --category "Technical Tags"

# Show tag usage statistics
fspec tag-stats

# Delete tag from registry
fspec delete-tag @deprecated
fspec delete-tag @deprecated --force  # Delete even if used in features
fspec delete-tag @deprecated --dry-run  # Preview what would be deleted

# Rename tags across all files
fspec retag --from=@old-tag --to=@new-tag
fspec retag --from=@old-tag --to=@new-tag --dry-run
```

## Feature-Level Tags

Manage tags directly on feature files:

```bash
# Add tags to a feature file
fspec add-tag-to-feature spec/features/login.feature @critical
fspec add-tag-to-feature spec/features/login.feature @critical @security
fspec add-tag-to-feature spec/features/login.feature @custom-tag --validate-registry

# Remove tags from a feature file
fspec remove-tag-from-feature spec/features/login.feature @wip
fspec remove-tag-from-feature spec/features/login.feature @wip @deprecated

# List tags on a feature file
fspec list-feature-tags spec/features/login.feature
fspec list-feature-tags spec/features/login.feature --show-categories
```

## Scenario-Level Tags

Manage tags on individual scenarios within feature files:

```bash
# Add tags to a specific scenario
fspec add-tag-to-scenario spec/features/login.feature "Login with valid credentials" @smoke
fspec add-tag-to-scenario spec/features/login.feature "Login with valid credentials" @smoke @regression
fspec add-tag-to-scenario spec/features/login.feature "Login" @custom-tag --validate-registry

# Remove tags from a specific scenario
fspec remove-tag-from-scenario spec/features/login.feature "Login with valid credentials" @wip
fspec remove-tag-from-scenario spec/features/login.feature "Login" @wip @deprecated

# List tags on a specific scenario
fspec list-scenario-tags spec/features/login.feature "Login with valid credentials"
fspec list-scenario-tags spec/features/login.feature "Login" --show-categories
```

## Important Notes

- **All tag write operations** (register-tag, update-tag, delete-tag) modify `spec/tags.json` and automatically regenerate `spec/TAGS.md`
- **Feature-level tags** apply to the entire feature
- **Scenario-level tags** apply only to specific scenarios
- **Tag inheritance:** Scenarios inherit feature-level tags when queried
- **Registry validation:** Use `--validate-registry` to ensure tags exist in spec/tags.json before adding
- **Never edit markdown files directly** - Always use fspec commands
- **Work unit ID tags** (e.g., `@AUTH-001`, `@COV-005`) MUST be at feature level only, never at scenario level. Use [coverage files](./coverage-tracking.md) for fine-grained scenario-to-implementation traceability

## Next Steps

- [Coverage Tracking](./coverage-tracking.md) - Link scenarios to tests and implementation
- [Usage Guide](./usage.md) - Detailed command examples
- [Project Management](./project-management.md) - Kanban workflow and work units
