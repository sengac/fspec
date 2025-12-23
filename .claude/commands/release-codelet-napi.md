# /release-codelet-napi - Create Tagged Release for codelet-napi

You are implementing the `/release-codelet-napi` slash command for creating tagged releases of the `@sengac/codelet-napi` native module.

**Note:** This is separate from the main fspec release. codelet-napi has its own version and release cycle.

## Critical Requirements

### Git Commit Author
**MANDATORY**: ALL commits created by this command MUST use:
- Author: `Roland Quast <rquast@rolandquast.com>`
- Use `--author="Roland Quast <rquast@rolandquast.com>"` flag with git commit

### Pre-Release Validation (MUST RUN FIRST)

1. **Check for uncommitted changes:**
   - Run `git status --porcelain` to check for unstaged/staged files
   - If changes exist in `codelet/` directory, stage them: `git add codelet/`
   - Create commit with analyzed message and author `Roland Quast <rquast@rolandquast.com>`

2. **Run E2E publish test:**
   - Execute `npm run test:napi-publish`
   - This tests the full publish → install → load flow locally
   - If test fails, ABORT release with error message

### Version Determination

1. **Get last codelet-napi git tag:**
   - Run `git describe --tags --match "codelet-napi-v*" --abbrev=0 2>/dev/null || echo ""`
   - If no tag exists, use `codelet/napi/package.json` version as base

2. **Get commits since last tag:**
   - Run `git log <last-tag>..HEAD --oneline -- codelet/` (only codelet changes)
   - Parse conventional commit messages

3. **Determine semver bump:**
   - If any commit has `BREAKING CHANGE:` in body/footer → **major** bump
   - Else if any commit starts with `feat:` → **minor** bump
   - Else if any commit starts with `fix:` → **patch** bump
   - Else → **patch** bump (default)

4. **Calculate new version:**
   - Parse base version (from tag or package.json)
   - Apply semver bump to calculate new version

### Version Update

1. **Update codelet/napi/package.json:**
   - Update `"version"` field to new version
   - Update ALL versions in `optionalDependencies` to match:
     ```json
     "optionalDependencies": {
       "@sengac/codelet-napi-darwin-arm64": "{new-version}",
       "@sengac/codelet-napi-darwin-x64": "{new-version}",
       "@sengac/codelet-napi-linux-arm64-gnu": "{new-version}",
       "@sengac/codelet-napi-linux-x64-gnu": "{new-version}",
       "@sengac/codelet-napi-win32-arm64-msvc": "{new-version}",
       "@sengac/codelet-napi-win32-x64-msvc": "{new-version}"
     }
     ```

2. **Stage and commit:**
   - Run `git add codelet/napi/package.json`
   - Commit message: `chore(codelet-napi): release v{version}`
   - Use `--author="Roland Quast <rquast@rolandquast.com>"`

3. **Create git tag:**
   - Tag name: `codelet-napi-v{version}` (with `codelet-napi-v` prefix)
   - Run: `git tag -a codelet-napi-v{version} -m "Release @sengac/codelet-napi v{version}"`

### Post-Release

- **DO NOT push** to remote (manual step for user)
- Display success message with:
  - New version number
  - Tag name created
  - Reminder to push: `git push && git push origin codelet-napi-v{version}`
  - Explanation that CI will build all 6 platforms and publish to npm

## Workflow Summary

```bash
# 1. Pre-release validation
git status --porcelain
git add codelet/  # Stage codelet changes if any
npm run test:napi-publish  # E2E test (ABORT if fails)

# 2. Version determination
git describe --tags --match "codelet-napi-v*" --abbrev=0  # Get last tag
git log <last-tag>..HEAD --oneline -- codelet/  # Get codelet commits
# Parse conventional commits, determine semver bump

# 3. Version update
# Update codelet/napi/package.json version AND optionalDependencies
git add codelet/napi/package.json
git commit -m "chore(codelet-napi): release v{version}" --author="Roland Quast <rquast@rolandquast.com>"

# 4. Create tag
git tag -a codelet-napi-v{version} -m "Release @sengac/codelet-napi v{version}"

# 5. Display success message (do NOT push)
```

## Example Output

```
$ /release-codelet-napi

Running E2E publish test...
✓ E2E TEST PASSED

Analyzing commits since codelet-napi-v0.1.0...
  - feat: add new persistence API
  - fix: memory leak in session handling

Determined version bump: minor (0.1.0 → 0.2.0)

Updating codelet/napi/package.json...
  - version: 0.2.0
  - optionalDependencies: all updated to 0.2.0

Creating release commit...
✓ Committed: chore(codelet-napi): release v0.2.0

Creating tag...
✓ Created tag: codelet-napi-v0.2.0

═══════════════════════════════════════════════════════════
  Release Ready: @sengac/codelet-napi v0.2.0
═══════════════════════════════════════════════════════════

To publish, run:
  git push && git push origin codelet-napi-v0.2.0

This will trigger GitHub Actions to:
  1. Build native binaries for all 6 platforms
  2. Run smoke tests on each platform
  3. Publish to npm:
     - @sengac/codelet-napi@0.2.0
     - @sengac/codelet-napi-darwin-arm64@0.2.0
     - @sengac/codelet-napi-darwin-x64@0.2.0
     - @sengac/codelet-napi-linux-arm64-gnu@0.2.0
     - @sengac/codelet-napi-linux-x64-gnu@0.2.0
     - @sengac/codelet-napi-win32-arm64-msvc@0.2.0
     - @sengac/codelet-napi-win32-x64-msvc@0.2.0
```

## Error Handling

- If E2E test fails: Display error, ABORT release
- If no codelet commits since last tag: Warn user, ask for confirmation
- If cannot determine version: Display error with guidance
