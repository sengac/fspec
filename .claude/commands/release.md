# /release - Create Tagged Release with Comprehensive Release Notes

**Feature Specification:** [spec/features/release-command-for-reviewing-changes-and-creating-tagged-releases.feature](../../spec/features/release-command-for-reviewing-changes-and-creating-tagged-releases.feature)

You are implementing the `/release` slash command for creating tagged releases with automated version bumping and comprehensive release notes.

## Critical Requirements

### Git Commit Author
**MANDATORY**: ALL commits created by this command MUST use:
- Author: `Roland Quast <rquast@rolandquast.com>`
- **NOT** Claude's default author
- Use `--author="Roland Quast <rquast@rolandquast.com>"` flag with git commit

### Pre-Release Validation (MUST RUN FIRST)

Before creating the release, you MUST:

1. **Check for uncommitted changes:**
   - Run `git status --porcelain` to check for unstaged/staged files
   - If changes exist, stage ALL files: `git add .`
   - Analyze changes and generate conventional commit message (NOT generic)
   - Create commit with analyzed message and author `Roland Quast <rquast@rolandquast.com>`

2. **Run tests:**
   - Execute `npm test`
   - If tests fail, ABORT release with error message

3. **Run build:**
   - Execute `npm run build`
   - If build fails, ABORT release with error message

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

### Release Commit Creation

1. **Update package.json version:**
   - Read `package.json`
   - Update `"version"` field to new version (without 'v' prefix)
   - Write updated `package.json`

2. **Stage package.json:**
   - Run `git add package.json`

3. **Create release commit:**
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

4. **Create git tag:**
   - Tag name: `v{version}` (with 'v' prefix)
   - Tag annotation: Use same release notes as commit body
   - Run: `git tag -a v{version} -m "{release notes}"`

### Post-Release

- **DO NOT push** to remote (manual step for user)
- Display success message with:
  - New version number
  - Release notes summary
  - Reminder to push: `git push && git push --tags`

## Workflow Summary

```bash
# 1. Pre-release validation
git status --porcelain  # Check for uncommitted changes
git add .  # Stage all if changes exist
# Analyze changes, create commit with conventional message and correct author
npm test  # Abort if fails
npm run build  # Abort if fails

# 2. Version determination
git describe --tags --abbrev=0  # Get last tag
git log <last-tag>..HEAD --oneline  # Get commits
# Parse conventional commits, determine semver bump

# 3. Code review
git diff <last-tag>..HEAD  # Get full diff
# Analyze changes line-by-line for release notes

# 4. Release commit
# Update package.json version
git add package.json
git commit -m "chore(release): v{version}" -m "{body}" --author="Roland Quast <rquast@rolandquast.com>"

# 5. Create tag
git tag -a v{version} -m "{release notes}"

# 6. Display success message (do NOT push)
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

- If tests fail: Display error, ABORT release
- If build fails: Display error, ABORT release
- If no commits since last tag: Warn user, ask for confirmation
- If cannot determine version: Display error with guidance

## Implementation Notes

- Combine conventional commit analysis with actual code diff review
- Do NOT rely solely on commit messages for release notes
- Review code changes to ensure nothing is missed
- Generate meaningful, detailed release notes
- Use author `Roland Quast <rquast@rolandquast.com>` for ALL commits
