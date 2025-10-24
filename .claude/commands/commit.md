# /commit - Create Conventional Commit with Automated Message Generation

**Feature Specification:** [spec/features/commit-command-with-conventional-commits-and-custom-author.feature](../../spec/features/commit-command-with-conventional-commits-and-custom-author.feature)

You are implementing the `/commit` slash command for creating conventional commits with automated message generation based on code changes.

## Critical Requirements

### Git Commit Author
**MANDATORY**: ALL commits created by this command MUST use:
- Author: `Roland Quast <rquast@rolandquast.com>`
- **NOT** Claude's default author
- Use `--author="Roland Quast <rquast@rolandquast.com>"` flag with git commit

### Clean Working Directory Check (FIRST)

1. **Check for changes:**
   - Run `git status --porcelain`
   - If output is empty (clean working directory):
     - Display: `Nothing to commit, working directory clean.`
     - Exit successfully (exit code 0)
     - **DO NOT create commit**

### Change Analysis

1. **Get unstaged and staged files:**
   - Run `git status --porcelain` to list all changes
   - Parse output to identify:
     - New files (untracked)
     - Modified files
     - Deleted files
     - Renamed files

2. **Analyze each file:**
   - Read file contents (for modified/new files)
   - Determine change type:
     - New feature implementation → `feat`
     - Bug fix → `fix`
     - Refactoring → `refactor`
     - Documentation → `docs`
     - Tests → `test`
     - Build/tooling → `build` or `chore`
     - Performance improvements → `perf`
     - Code style → `style`

3. **Determine scope:**
   - Extract scope from file path
   - Examples:
     - `src/commands/validate.ts` → scope: `commands`
     - `src/hooks/integration.ts` → scope: `hooks`
     - `spec/features/test.feature` → scope: `specs`
     - `README.md` → scope: `docs`
     - `package.json` → scope: `build`

4. **Analyze diffs for detail:**
   - Run `git diff` for unstaged changes
   - Run `git diff --staged` for staged changes
   - Review line-by-line changes to understand intent
   - Identify breaking changes (API changes, removals)

### Conventional Commit Message Generation

1. **Commit message structure:**
   ```
   type(scope): description

   [optional body with detailed changes]

   [optional footer with BREAKING CHANGE if applicable]
   ```

2. **Type determination:**
   - `feat`: New feature or enhancement
   - `fix`: Bug fix
   - `refactor`: Code refactoring (no behavior change)
   - `docs`: Documentation changes
   - `test`: Test additions or modifications
   - `chore`: Build, tooling, dependencies
   - `perf`: Performance improvements
   - `style`: Code formatting (no logic change)

3. **Description (< 72 characters):**
   - Clear, concise summary of changes
   - Use imperative mood ("add" not "added" or "adds")
   - No period at end

4. **Body (optional):**
   - Detailed explanation of changes
   - Wrap lines at 72 characters
   - Use bullet points for multiple changes
   - Explain **why** changes were made, not just **what**

5. **Footer (optional):**
   - Include `BREAKING CHANGE:` if API breaking changes
   - Reference issues if applicable (e.g., `Closes #123`)

### Staging and Committing

1. **Stage all unstaged files:**
   - Run `git add .`

2. **Create commit:**
   - Use generated conventional commit message
   - Use author `Roland Quast <rquast@rolandquast.com>`
   - Run: `git commit -m "{message}" -m "{body}" --author="Roland Quast <rquast@rolandquast.com>"`

3. **DO NOT push:**
   - User must push manually

## Workflow Summary

```bash
# 1. Check for clean working directory
git status --porcelain
if [ -z "$output" ]; then
  echo "Nothing to commit, working directory clean."
  exit 0
fi

# 2. Analyze changes
git status --porcelain  # List all changes
git diff  # Get unstaged diffs
git diff --staged  # Get staged diffs
# Analyze files and diffs to determine change type and scope

# 3. Stage all files
git add .

# 4. Create commit
git commit -m "type(scope): description" -m "{body}" --author="Roland Quast <rquast@rolandquast.com>"

# 5. Display success (do NOT push)
```

## Example Scenarios

### Scenario 1: Clean working directory
```
$ /commit

Checking for changes...

✓ Nothing to commit, working directory clean.
```

### Scenario 2: New feature with multiple files
```
$ /commit

Analyzing changes...
  New file: src/commands/new-feature.ts
  Modified: src/types.ts
  Modified: README.md

Change type: feat (new feature implementation)
Scope: commands
Description: add new feature command

Staging all files...
git add .

Creating commit...
git commit -m "feat(commands): add new feature command" \
  -m "Implement new-feature command with TypeScript types and CLI integration.

- Add new command handler in src/commands/new-feature.ts
- Extend TypeScript type definitions for command options
- Update README with usage examples" \
  --author="Roland Quast <rquast@rolandquast.com>"

✓ Commit created successfully
  Author: Roland Quast <rquast@rolandquast.com>
  Message: feat(commands): add new feature command

Reminder: Run 'git push' to push changes to remote.
```

### Scenario 3: Bug fix
```
$ /commit

Analyzing changes...
  Modified: src/commands/validate.ts

Change type: fix (bug fix)
Scope: validation
Description: resolve syntax error handling

Creating commit...
git commit -m "fix(validation): resolve syntax error handling" \
  -m "Fix edge case where multi-line syntax errors were not properly reported.

- Update error parser to handle newlines in error messages
- Add test coverage for multi-line error scenarios" \
  --author="Roland Quast <rquast@rolandquast.com>"

✓ Commit created successfully
```

### Scenario 4: Breaking change
```
$ /commit

Analyzing changes...
  Modified: src/api/interface.ts (BREAKING CHANGE detected)

Change type: feat (new feature with breaking change)
Scope: api
Description: redesign interface API

Creating commit...
git commit -m "feat(api): redesign interface API" \
  -m "Redesign interface API for better TypeScript support.

- Remove deprecated `validate()` method
- Replace with `validateAsync()` for better async handling
- Update all callers to use new API

BREAKING CHANGE: The `validate()` method has been removed. Use `validateAsync()` instead." \
  --author="Roland Quast <rquast@rolandquast.com>"

✓ Commit created successfully
⚠ This commit contains a BREAKING CHANGE
```

## Implementation Notes

- **Analyze code changes deeply** - Don't rely on filenames alone
- Review diffs to understand the **intent** of changes
- Generate **meaningful, descriptive** commit messages
- Follow strict conventional commits specification
- Use imperative mood in descriptions
- Include detailed body for non-trivial changes
- Flag breaking changes clearly
- Use author `Roland Quast <rquast@rolandquast.com>` for ALL commits
- Exit successfully (code 0) even when nothing to commit

## Conventional Commit Types Reference

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code formatting (no logic change)
- `refactor`: Code refactoring (no behavior change)
- `perf`: Performance improvement
- `test`: Test additions/modifications
- `build`: Build system or dependencies
- `ci`: CI configuration changes
- `chore`: Other changes (tooling, etc.)
