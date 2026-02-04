# /publish - Conditional NPM Publishing with Version Verification

**Feature Specification:** [spec/features/publish-command-for-conditional-npm-publishing.feature](../../spec/features/publish-command-for-conditional-npm-publishing.feature)

You are implementing the `/publish` slash command for conditionally publishing to npm only if the version differs from the registry.

## Critical Requirements

### Dependencies
- **MUST have run `/release` command first** (CLI-008)
- Assumes tests and build have already passed (validated by `/release`)

### Pre-Publish Validation (MUST CHECK FIRST)

Before publishing, you MUST verify in this EXACT order:

1. **Verify npm authentication:**
   - Run `npm whoami`
   - If not logged in, ABORT with error: "Not logged in to npm. Run `npm login` first."

2. **Get current git tag:**
   - Run `git describe --tags --exact-match 2>/dev/null`
   - Parse version from tag (e.g., `v0.3.1` → `0.3.1`)
   - If no tag at HEAD, ABORT with error: "No git tag found at HEAD. Run /release first."

3. **Verify all versions match:**
   - Read root `package.json` version
   - Read `codelet/napi/package.json` version
   - Compare both with git tag version
   - If ANY mismatch, ABORT with error:
     ```
     Version mismatch detected!
     Git tag: v{tag-version}
     package.json: {root-version}
     codelet/napi/package.json: {napi-version}

     All versions must match. Run /release first.
     ```

4. **Verify codelet-napi binaries exist:**
   - Check that `codelet/napi/*.node` files exist for all 6 platforms:
     - `codelet-napi.darwin-arm64.node`
     - `codelet-napi.darwin-x64.node`
     - `codelet-napi.linux-arm64-gnu.node`
     - `codelet-napi.linux-x64-gnu.node`
     - `codelet-napi.win32-arm64-msvc.node`
     - `codelet-napi.win32-x64-msvc.node`
   - If ANY missing, ABORT with error listing missing binaries

### Publishing @sengac/fspec

1. **Get npm registry version for fspec:**
   - Run `npm view @sengac/fspec version 2>/dev/null || echo ""`
   - If package not found in registry, treat as first publish

2. **Compare versions:**
   - If npm registry version equals package.json version:
     - Display: `@sengac/fspec v{version} already published. Skipping.`
   - If versions differ:
     - Run `npm publish`
     - Display success message

### Publishing @sengac/codelet-napi

1. **Get npm registry version for codelet-napi:**
   - Run `npm view @sengac/codelet-napi version 2>/dev/null || echo ""`
   - If package not found in registry, treat as first publish

2. **Compare versions:**
   - If npm registry version equals codelet/napi/package.json version:
     - Display: `@sengac/codelet-napi v{version} already published. Skipping.`
   - If versions differ:
     - Run `cd codelet/napi && npm publish --access public`
     - Display success message

### Post-Publish

- Display summary with:
  - Which packages were published (or skipped)
  - npm package URLs for published packages
  - Confirmation that packages are live

## Workflow Summary

```bash
# 1. Verify authentication
npm whoami  # ABORT if not logged in

# 2. Verify git tag at HEAD
git describe --tags --exact-match  # ABORT if no tag

# 3. Verify all versions match
# Read package.json version
# Read codelet/napi/package.json version
# Compare both with tag version - ABORT if any mismatch

# 4. Verify codelet-napi binaries exist
ls codelet/napi/*.node  # ABORT if any missing

# 5. Publish fspec if needed
npm view @sengac/fspec version 2>/dev/null || echo ""
# Skip if already published, otherwise:
npm publish

# 6. Publish codelet-napi if needed
npm view @sengac/codelet-napi version 2>/dev/null || echo ""
# Skip if already published, otherwise:
cd codelet/napi && npm publish --access public

# 7. Display summary
```

## Example Scenarios

### Scenario 1: Both packages need publishing
```
$ /publish

Checking npm authentication...
  ✓ Logged in as: sengac

Checking git tag...
  ✓ Tag at HEAD: v0.3.1

Verifying versions...
  package.json: 0.3.1
  codelet/napi/package.json: 0.3.1
  ✓ All versions match tag

Checking codelet-napi binaries...
  ✓ codelet-napi.darwin-arm64.node
  ✓ codelet-napi.darwin-x64.node
  ✓ codelet-napi.linux-arm64-gnu.node
  ✓ codelet-napi.linux-x64-gnu.node
  ✓ codelet-napi.win32-arm64-msvc.node
  ✓ codelet-napi.win32-x64-msvc.node

Publishing @sengac/fspec...
  npm registry: 0.3.0
  Local version: 0.3.1
  ✓ Publishing new version...
  ✓ Published @sengac/fspec@0.3.1

Publishing @sengac/codelet-napi...
  npm registry: 0.3.0
  Local version: 0.3.1
  ✓ Publishing new version...
  ✓ Published @sengac/codelet-napi@0.3.1

═══════════════════════════════════════════════════════════
  Successfully published:
  📦 @sengac/fspec@0.3.1
     https://www.npmjs.com/package/@sengac/fspec
  📦 @sengac/codelet-napi@0.3.1
     https://www.npmjs.com/package/@sengac/codelet-napi
═══════════════════════════════════════════════════════════
```

### Scenario 2: Already published (skip both)
```
$ /publish

Checking npm authentication...
  ✓ Logged in as: sengac

Checking git tag...
  ✓ Tag at HEAD: v0.3.0

Verifying versions...
  ✓ All versions match tag (0.3.0)

Checking codelet-napi binaries...
  ✓ All 6 binaries present

Publishing @sengac/fspec...
  npm registry: 0.3.0
  Local version: 0.3.0
  ⚠ Already published. Skipping.

Publishing @sengac/codelet-napi...
  npm registry: 0.3.0
  Local version: 0.3.0
  ⚠ Already published. Skipping.

✓ Both packages already up to date on npm.
```

### Scenario 3: Version mismatch (error)
```
$ /publish

Checking git tag...
  Tag at HEAD: v0.3.1

Verifying versions...
  package.json: 0.3.1
  codelet/napi/package.json: 0.3.0  ← MISMATCH

✗ Version mismatch detected!

Git tag: v0.3.1
package.json: 0.3.1
codelet/napi/package.json: 0.3.0

All versions must match. Run /release first.
```

### Scenario 4: Missing binaries
```
$ /publish

Checking codelet-napi binaries...
  ✓ codelet-napi.darwin-arm64.node
  ✓ codelet-napi.darwin-x64.node
  ✗ codelet-napi.linux-arm64-gnu.node  ← MISSING
  ✓ codelet-napi.linux-x64-gnu.node
  ✗ codelet-napi.win32-arm64-msvc.node  ← MISSING
  ✓ codelet-napi.win32-x64-msvc.node

✗ Missing binaries! Run /release first to build all platforms.
```

### Scenario 5: No tag at HEAD
```
$ /publish

Checking git tag...
  ✗ No git tag found at HEAD

Run /release first to create a tagged release.
```

### Scenario 6: Not logged in
```
$ /publish

Checking npm authentication...
  ✗ Not logged in to npm

Please run `npm login` first, then retry.
```

## Error Handling

- **Not logged in:** ABORT with instructions to run `npm login`
- **No git tag at HEAD:** ABORT with error "Run /release first"
- **Version mismatch:** ABORT with detailed error showing all versions
- **Missing binaries:** ABORT with list of missing binaries
- **npm publish fails:** Display npm error output, suggest checking npm status
- **Network issues:** Catch and display connection errors

## Implementation Notes

- **DO NOT run tests or build** - Assumes `/release` already validated everything
- **Exit successfully (code 0)** when skipping already-published packages (not an error)
- Check npm registry BEFORE publishing to avoid unnecessary errors
- Display clear, actionable error messages
- Verify ALL versions match (tag, root package.json, codelet/napi/package.json)
- Both packages should always have the same version number
