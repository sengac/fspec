# /release-codelet-napi - Create Tagged Release for codelet-napi

You are implementing the `/release-codelet-napi` slash command for creating tagged releases of the `@sengac/codelet-napi` native module.

**Note:** This is separate from the main fspec release. codelet-napi has its own version and release cycle.

## Flow Overview

1. `/release-codelet-napi` - bumps version, creates tag (this command)
2. Push changes - triggers CI to build binaries and commit them
3. Pull the committed binaries
4. `/publish-codelet-napi` - publishes to npm from local machine

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

2. **Verify binaries are up to date (optional but recommended):**
   - Check if CI has recently built binaries
   - If binaries are stale, warn user to push and wait for CI

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
  - Next steps

## Workflow Summary

```bash
# 1. Pre-release validation
git status --porcelain
git add codelet/  # Stage codelet changes if any

# 2. Version determination
git describe --tags --match "codelet-napi-v*" --abbrev=0  # Get last tag
git log <last-tag>..HEAD --oneline -- codelet/  # Get codelet commits
# Parse conventional commits, determine semver bump

# 3. Version update
# Update codelet/napi/package.json version
git add codelet/napi/package.json
git commit -m "chore(codelet-napi): release v{version}" --author="Roland Quast <rquast@rolandquast.com>"

# 4. Create tag
git tag -a codelet-napi-v{version} -m "Release @sengac/codelet-napi v{version}"

# 5. Display success message (do NOT push)
```

## Example Output

```
$ /release-codelet-napi

Checking for uncommitted changes...
  ✓ Working directory clean

Analyzing commits since codelet-napi-v0.1.0...
  - feat: add new persistence API
  - fix: memory leak in session handling

Determined version bump: minor (0.1.0 → 0.2.0)

Updating codelet/napi/package.json...
  - version: 0.2.0

Creating release commit...
✓ Committed: chore(codelet-napi): release v0.2.0

Creating tag...
✓ Created tag: codelet-napi-v0.2.0

═══════════════════════════════════════════════════════════
  Release Ready: @sengac/codelet-napi v0.2.0
═══════════════════════════════════════════════════════════

Next steps:

1. Push to trigger CI build:
   git push && git push origin codelet-napi-v0.2.0

2. Wait for CI to build binaries and commit them (~10 min)
   Monitor: https://github.com/sengac/fspec/actions

3. Pull the committed binaries:
   git pull

4. Publish to npm:
   /publish-codelet-napi
```

## Error Handling

- If no codelet commits since last tag: Warn user, ask for confirmation
- If cannot determine version: Display error with guidance
