# /publish - Conditional NPM Publishing with Version Verification

**Feature Specification:** [spec/features/publish-command-for-conditional-npm-publishing.feature](../../spec/features/publish-command-for-conditional-npm-publishing.feature)

You are implementing the `/publish` slash command for conditionally publishing to npm only if the version differs from the registry.

## Critical Requirements

### Dependencies
- **MUST have run `/release` command first** (CLI-008)
- Assumes tests and build have already passed (validated by `/release`)

### Version Verification (MUST CHECK FIRST)

Before publishing, you MUST verify:

1. **Get current git tag:**
   - Run `git describe --tags --exact-match 2>/dev/null`
   - Parse version from tag (e.g., `v0.3.1` → `0.3.1`)
   - If no tag, ABORT with error: "No git tag found. Run /release first."

2. **Get package.json version:**
   - Read `package.json`
   - Extract `"version"` field value

3. **Verify git tag matches package.json:**
   - Compare git tag version with package.json version
   - If mismatch, ABORT with error:
     ```
     Version mismatch: git tag (v{tag-version}) does not match package.json ({pkg-version}).
     Run /release first to ensure versions are synchronized.
     ```

4. **Get npm registry version:**
   - Run `npm view @sengac/fspec version 2>/dev/null || echo ""`
   - If package not found in registry, treat as version `0.0.0` (first publish)

5. **Compare versions:**
   - If npm registry version equals package.json version:
     - Display: `Version {version} already published to npm. Skipping.`
     - Exit successfully (exit code 0)
     - **DO NOT publish**
   - If versions differ:
     - Proceed with publishing

### Publishing

1. **Run npm publish:**
   - Execute `npm publish`
   - **DO NOT run tests or build** (assumes `/release` already validated)
   - Capture output for display

2. **Display success message:**
   - Show published version
   - Show npm package URL
   - Reminder that package is now live

## Workflow Summary

```bash
# 1. Version verification
git describe --tags --exact-match  # Get current tag
# Parse version from tag (v0.3.1 → 0.3.1)
# Read package.json version
# Compare git tag vs package.json (ABORT if mismatch)

# 2. Check npm registry
npm view @sengac/fspec version  # Get published version
# Compare with package.json version

# 3. Conditional publish
if [ "$pkg_version" == "$npm_version" ]; then
  echo "Version already published. Skipping."
  exit 0
else
  npm publish
fi
```

## Example Scenarios

### Scenario 1: Version already published (skip)
```
$ /publish

Checking versions...
  Git tag: v0.3.0
  package.json: 0.3.0
  npm registry: 0.3.0

✓ Versions match locally
⚠ Version 0.3.0 already published to npm. Skipping.

No action needed. Package is up to date.
```

### Scenario 2: Version mismatch (error)
```
$ /publish

Checking versions...
  Git tag: v0.3.1
  package.json: 0.3.0
  npm registry: 0.3.0

✗ Version mismatch detected!

Git tag (v0.3.1) does not match package.json (0.3.0).
Run /release first to ensure versions are synchronized.
```

### Scenario 3: New version to publish (success)
```
$ /publish

Checking versions...
  Git tag: v0.3.1
  package.json: 0.3.1
  npm registry: 0.3.0

✓ Versions match locally
✓ New version detected (0.3.1 > 0.3.0)

Publishing to npm...
npm publish

✓ Successfully published @sengac/fspec@0.3.1 to npm
  📦 https://www.npmjs.com/package/@sengac/fspec

Package is now live on npm registry.
```

### Scenario 4: First publish (no npm version)
```
$ /publish

Checking versions...
  Git tag: v0.1.0
  package.json: 0.1.0
  npm registry: (not found)

✓ Versions match locally
✓ First publish detected

Publishing to npm...
npm publish

✓ Successfully published @sengac/fspec@0.1.0 to npm
  📦 https://www.npmjs.com/package/@sengac/fspec

Package is now live on npm registry.
```

## Error Handling

- **No git tag:** ABORT with error "No git tag found. Run /release first."
- **Version mismatch:** ABORT with detailed error showing git tag vs package.json
- **npm publish fails:** Display npm error output, explain potential causes
- **Network issues:** Catch and display connection errors

## Implementation Notes

- **DO NOT run tests or build** - Assumes `/release` already validated everything
- **Exit successfully (code 0)** even when skipping publish (already published is not an error)
- Check npm registry BEFORE publishing to avoid unnecessary errors
- Display clear, actionable error messages
- Use package name `@sengac/fspec` for npm registry checks
