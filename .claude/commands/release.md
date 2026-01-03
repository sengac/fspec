# /release - Create Tagged Release with Comprehensive Release Notes

**Feature Specification:** [spec/features/release-command-for-reviewing-changes-and-creating-tagged-releases.feature](../../spec/features/release-command-for-reviewing-changes-and-creating-tagged-releases.feature)

You are implementing the `/release` slash command for creating tagged releases with automated version bumping and comprehensive release notes.

## Prerequisites

Before running this command, ensure:
- **Docker Desktop is running** (required for Linux/Windows cross-compilation of codelet-napi)
- **rustup is installed** (not Homebrew Rust) with x86_64-apple-darwin target
- **Node.js 20+** is available

## Critical Requirements

### Git Commit Author
**MANDATORY**: ALL commits created by this command MUST use:
- Author: `Roland Quast <rquast@rolandquast.com>`
- **NOT** Claude's default author
- Use `--author="Roland Quast <rquast@rolandquast.com>"` flag with git commit

### Pre-Release Validation (MUST RUN FIRST)

Before creating the release, you MUST run these checks in this EXACT order:

1. **Verify Docker is running (fail fast):**
   - Run `docker info &>/dev/null` to check Docker is available
   - If not running, ABORT with error: "Docker is not running. Start Docker Desktop and try again."

2. **Check for uncommitted changes:**
   - Run `git status --porcelain` to check for unstaged/staged files
   - If changes exist, stage ALL files: `git add .`
   - **DO NOT commit yet** - we need to test first

3. **Run tests:**
   - Execute `npm test`
   - If tests fail, ABORT release with error message
   - **DO NOT commit if tests fail**

4. **Run build:**
   - Execute `npm run build`
   - If build fails, ABORT release with error message
   - **DO NOT commit if build fails**

5. **Commit staged changes (only if tests/build passed):**
   - If there were uncommitted changes from step 2:
     - Analyze changes and generate conventional commit message (NOT generic)
     - Create commit with analyzed message and author `Roland Quast <rquast@rolandquast.com>`
   - If no uncommitted changes, skip this step

### Version Determination

1. **Get last git tag:**
   - Run `git describe --tags --abbrev=0 2>/dev/null || echo ""`
   - If no tag exists, use `package.json` version as base

2. **Get commits since last tag:**
   - Run `git log <last-tag>..HEAD --oneline` (or all commits if no tag)
   - Parse conventional commit messages

3. **Determine semver bump:**
   - If any commit has `BREAKING CHANGE:` in body/footer → **major** bump
   - Else if any commit starts with `feat:` → **minor** bump
   - Else if any commit starts with `fix:` → **patch** bump
   - Else → **patch** bump (default)

4. **Calculate new version:**
   - Parse base version (from tag or package.json)
   - Apply semver bump to calculate new version

### Code Review Analysis

1. **Get full code diff since last tag:**
   - Run `git diff <last-tag>..HEAD` (or all changes if no tag)

2. **Review changes line-by-line:**
   - Analyze all modified files
   - Identify breaking changes, new features, bug fixes
   - Note significant architectural changes
   - Identify deprecated functionality

### Build codelet-napi Binaries

**CRITICAL STEP**: Build all 6 platform binaries for codelet-napi.

1. **Run the build script:**
   ```bash
   npm run build:codelet-napi:all
   ```

2. **Verify all 6 binaries exist in `codelet/napi/`:**
   - `codelet-napi.darwin-arm64.node`
   - `codelet-napi.darwin-x64.node`
   - `codelet-napi.linux-arm64-gnu.node`
   - `codelet-napi.linux-x64-gnu.node`
   - `codelet-napi.win32-arm64-msvc.node`
   - `codelet-napi.win32-x64-msvc.node`

3. **If any binary is missing, ABORT with error.**

### Release Commit Creation

1. **Update package.json version:**
   - Read `package.json`
   - Update `"version"` field to new version (without 'v' prefix)
   - Write updated `package.json`

2. **Update codelet/napi/package.json version:**
   - Read `codelet/napi/package.json`
   - Update `"version"` field to same new version
   - Write updated `codelet/napi/package.json`

3. **Stage all changes:**
   - Run `git add package.json codelet/napi/package.json codelet/napi/*.node`

4. **Create release commit:**
   - Commit message format: `chore(release): v{version}`
   - Commit body format:
     ```
     ## Breaking Changes
     - [List breaking changes or write "(none)"]

     ## Features
     - [List new features]

     ## Fixes
     - [List bug fixes]

     ## Other Changes
     - [List other significant changes]
     ```
   - Use `--author="Roland Quast <rquast@rolandquast.com>"`
   - Run: `git commit -m "chore(release): v{version}" -m "{body}" --author="Roland Quast <rquast@rolandquast.com>"`

5. **Create git tag:**
   - Tag name: `v{version}` (with 'v' prefix)
   - Tag annotation: Use same release notes as commit body
   - Run: `git tag -a v{version} -m "{release notes}"`

### Post-Release

- **DO NOT push** to remote (manual step for user)
- Display success message with:
  - New version number
  - Release notes summary
  - List of built codelet-napi binaries with sizes
  - Reminder to push: `git push && git push --tags`

## Workflow Summary

```bash
# 1. Pre-release validation (in this exact order!)
docker info  # Verify Docker is running - ABORT if not
git status --porcelain  # Check for uncommitted changes
git add .  # Stage all if changes exist (but DON'T commit yet)
npm test  # ABORT if fails (don't commit broken code)
npm run build  # ABORT if fails (don't commit broken code)
# Only NOW commit staged changes (if any existed)
git commit -m "{conventional message}" --author="Roland Quast <rquast@rolandquast.com>"

# 2. Version determination
git describe --tags --abbrev=0  # Get last tag
git log <last-tag>..HEAD --oneline  # Get commits
# Parse conventional commits, determine semver bump

# 3. Code review
git diff <last-tag>..HEAD  # Get full diff
# Analyze changes line-by-line for release notes

# 4. Build codelet-napi binaries
npm run build:codelet-napi:all
# Verify all 6 .node files exist

# 5. Release commit
# Update package.json version
# Update codelet/napi/package.json version
git add package.json codelet/napi/package.json codelet/napi/*.node
git commit -m "chore(release): v{version}" -m "{body}" --author="Roland Quast <rquast@rolandquast.com>"

# 6. Create tag
git tag -a v{version} -m "{release notes}"

# 7. Display success message (do NOT push)
```

## Example Release Notes Format

```
chore(release): v0.3.1

## Breaking Changes
(none)

## Features
- Add release command for automated version management
- Implement conventional commit analysis for semver bumping

## Fixes
- Resolve git tag parsing edge case
- Fix package.json version update logic

## Other Changes
- Update documentation with release workflow
- Refactor git operations for better error handling
```

## Error Handling

- If Docker is not running: ABORT immediately with instructions to start Docker Desktop
- If tests fail: ABORT release, DO NOT commit any staged changes
- If build fails: ABORT release, DO NOT commit any staged changes
- If codelet-napi build fails: Display error, ABORT release
- If any codelet-napi binary is missing: ABORT with list of missing binaries
- If no commits since last tag: Warn user, ask for confirmation
- If cannot determine version: Display error with guidance

## Implementation Notes

- **CRITICAL**: Test and build BEFORE committing. Never commit untested code.
- Combine conventional commit analysis with actual code diff review
- Do NOT rely solely on commit messages for release notes
- Review code changes to ensure nothing is missed
- Generate meaningful, detailed release notes
- Use author `Roland Quast <rquast@rolandquast.com>` for ALL commits
- Both package.json files (root and codelet/napi) must have matching versions
