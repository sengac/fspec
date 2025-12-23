# /publish-codelet-napi - Push Tag to Trigger CI Publishing

You are implementing the `/publish-codelet-napi` slash command for publishing `@sengac/codelet-napi` to npm via GitHub Actions CI.

**Note:** Unlike fspec, codelet-napi publishing is handled by CI. This command pushes the tag which triggers the build and publish workflow.

## Critical Requirements

### Pre-Publish Validation (MUST CHECK FIRST)

1. **Verify codelet-napi tag exists locally:**
   - Run `git describe --tags --match "codelet-napi-v*" --exact-match 2>/dev/null`
   - Parse version from tag (e.g., `codelet-napi-v0.2.0` → `0.2.0`)
   - If no tag, ABORT with error: "No codelet-napi tag found. Run /release-codelet-napi first."

2. **Verify package.json version matches:**
   - Read `codelet/napi/package.json`
   - Compare git tag version with package.json version
   - If mismatch, ABORT with error

3. **Check if already published to npm:**
   - Run `npm view @sengac/codelet-napi version 2>/dev/null || echo ""`
   - If npm version equals tag version:
     - Display: `Version {version} already published to npm. Skipping.`
     - Exit successfully (no action needed)

4. **Check if tag already pushed:**
   - Run `git ls-remote --tags origin | grep "codelet-napi-v{version}"`
   - If tag exists on remote, check CI status instead of pushing again

### Publishing (Push Tag)

1. **Push commits:**
   - Run `git push`

2. **Push tag:**
   - Run `git push origin codelet-napi-v{version}`

3. **Display CI link:**
   - Show GitHub Actions URL: `https://github.com/sengac/fspec/actions`
   - Explain what will happen

### Post-Push Monitoring (Optional)

After pushing, optionally check CI status:
- Run `gh run list --workflow=build-codelet-napi.yml --limit=1`
- Display current status

## Workflow Summary

```bash
# 1. Version verification
git describe --tags --match "codelet-napi-v*" --exact-match  # Get current tag
# Parse version from tag (codelet-napi-v0.2.0 → 0.2.0)
# Read codelet/napi/package.json version
# Compare (ABORT if mismatch)

# 2. Check npm registry
npm view @sengac/codelet-napi version  # Get published version
# If already published, skip

# 3. Check if tag already on remote
git ls-remote --tags origin | grep "codelet-napi-v{version}"

# 4. Push to trigger CI
git push
git push origin codelet-napi-v{version}

# 5. Display CI link
echo "Monitor CI: https://github.com/sengac/fspec/actions"
```

## Example Scenarios

### Scenario 1: Fresh release ready to publish
```
$ /publish-codelet-napi

Checking versions...
  Git tag: codelet-napi-v0.2.0
  package.json: 0.2.0
  npm registry: 0.1.0

✓ Versions match locally
✓ New version detected (0.2.0 > 0.1.0)

Pushing to remote...
  git push
  git push origin codelet-napi-v0.2.0

✓ Tag pushed successfully!

═══════════════════════════════════════════════════════════
  CI Build Triggered: @sengac/codelet-napi v0.2.0
═══════════════════════════════════════════════════════════

GitHub Actions is now:
  1. Building native binaries for 6 platforms (~5-10 min)
  2. Running smoke tests on each platform
  3. Publishing all packages to npm

Monitor progress:
  https://github.com/sengac/fspec/actions

Once complete, verify:
  npm view @sengac/codelet-napi version
```

### Scenario 2: Already published
```
$ /publish-codelet-napi

Checking versions...
  Git tag: codelet-napi-v0.2.0
  package.json: 0.2.0
  npm registry: 0.2.0

✓ Versions match locally
⚠ Version 0.2.0 already published to npm. Skipping.

No action needed. Package is up to date.
```

### Scenario 3: No tag found
```
$ /publish-codelet-napi

Checking versions...
✗ No codelet-napi tag found on current commit.

Run /release-codelet-napi first to create a release.
```

### Scenario 4: Version mismatch
```
$ /publish-codelet-napi

Checking versions...
  Git tag: codelet-napi-v0.2.0
  package.json: 0.1.0

✗ Version mismatch detected!

Git tag (codelet-napi-v0.2.0) does not match package.json (0.1.0).
Run /release-codelet-napi first to ensure versions are synchronized.
```

### Scenario 5: Tag already pushed, CI in progress
```
$ /publish-codelet-napi

Checking versions...
  Git tag: codelet-napi-v0.2.0
  package.json: 0.2.0
  npm registry: 0.1.0

✓ Versions match locally
✓ Tag already pushed to remote

Checking CI status...
  Workflow: Build codelet-napi
  Status: in_progress
  Started: 3 minutes ago

CI is still running. Monitor progress:
  https://github.com/sengac/fspec/actions
```

## Error Handling

- **No git tag:** ABORT with error "No codelet-napi tag found. Run /release-codelet-napi first."
- **Version mismatch:** ABORT with detailed error showing git tag vs package.json
- **Push fails:** Display git error output, explain potential causes
- **Network issues:** Catch and display connection errors

## Implementation Notes

- This command triggers CI, it doesn't publish directly
- The actual npm publish happens in GitHub Actions after all 6 platforms build
- Use `gh` CLI if available for checking CI status
- Always display the Actions URL for manual monitoring
