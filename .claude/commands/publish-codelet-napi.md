# /publish-codelet-napi - Publish codelet-napi to npm

You are implementing the `/publish-codelet-napi` slash command for publishing `@sengac/codelet-napi` to npm.

**Note:** This publishes from the local machine where the user is authenticated with npm. The binaries must already be built and committed to the repo by CI.

## Prerequisites

1. User must be logged in to npm (`npm whoami` should return their username)
2. Binaries must exist in `codelet/napi/*.node` (built by CI)
3. Version should be bumped via `/release-codelet-napi` first

## Critical Requirements

### Pre-Publish Validation (MUST CHECK FIRST)

1. **Verify npm authentication:**
   - Run `npm whoami`
   - If not logged in, ABORT with error: "Not logged in to npm. Run `npm login` first."

2. **Verify binaries exist:**
   - Check that `codelet/napi/*.node` files exist for all 6 platforms
   - Required: darwin-arm64, darwin-x64, linux-arm64-gnu, linux-x64-gnu, win32-arm64-msvc, win32-x64-msvc
   - If missing, ABORT with error: "Binaries missing. Wait for CI to build and commit them."

3. **Check if already published to npm:**
   - Run `npm view @sengac/codelet-napi version 2>/dev/null || echo "not-published"`
   - Compare with `codelet/napi/package.json` version
   - If versions match: Display "Version {version} already published. Skipping." and exit successfully

### Publishing

1. **Navigate to codelet/napi directory**

2. **Run npm publish:**
   ```bash
   cd codelet/napi
   npm publish --access public
   ```

3. **Verify publication:**
   - Run `npm view @sengac/codelet-napi version`
   - Confirm it matches the published version

### Post-Publish

1. **Display success message** with the published version
2. **Remind user** to update fspec's package.json if this is the first publish:
   - Change `"codelet-napi": "file:codelet/napi"` to `"@sengac/codelet-napi": "^{version}"`

## Workflow Summary

```bash
# 1. Verify authentication
npm whoami

# 2. Verify binaries exist
ls codelet/napi/*.node

# 3. Check current npm version
npm view @sengac/codelet-napi version 2>/dev/null || echo "not-published"

# 4. Check local version
node -p "require('./codelet/napi/package.json').version"

# 5. Publish
cd codelet/napi && npm publish --access public

# 6. Verify
npm view @sengac/codelet-napi version
```

## Example Scenarios

### Scenario 1: First publish
```
$ /publish-codelet-napi

Checking npm authentication...
  Logged in as: sengac

Checking binaries...
  ✓ codelet-napi.darwin-arm64.node
  ✓ codelet-napi.darwin-x64.node
  ✓ codelet-napi.linux-arm64-gnu.node
  ✓ codelet-napi.linux-x64-gnu.node
  ✓ codelet-napi.win32-arm64-msvc.node
  ✓ codelet-napi.win32-x64-msvc.node

Checking npm registry...
  Package not yet published

Publishing @sengac/codelet-napi@0.1.0...
  npm publish --access public
  ✓ Published successfully!

Verification:
  npm view @sengac/codelet-napi version → 0.1.0

═══════════════════════════════════════════════════════════
  Successfully published @sengac/codelet-napi v0.1.0 to npm
═══════════════════════════════════════════════════════════

IMPORTANT: This is the first publish. Update fspec's package.json:
  Change: "codelet-napi": "file:codelet/napi"
  To:     "@sengac/codelet-napi": "^0.1.0"
```

### Scenario 2: Already published
```
$ /publish-codelet-napi

Checking npm authentication...
  Logged in as: sengac

Checking versions...
  Local version: 0.1.0
  npm version: 0.1.0

✓ Version 0.1.0 already published to npm. No action needed.
```

### Scenario 3: Missing binaries
```
$ /publish-codelet-napi

Checking binaries...
  ✗ Missing: codelet-napi.linux-x64-gnu.node

Binaries are missing. Please wait for CI to build and commit them.
Check: https://github.com/sengac/fspec/actions
```

### Scenario 4: Not logged in
```
$ /publish-codelet-napi

Checking npm authentication...
  ✗ Not logged in to npm

Please run `npm login` first, then retry.
```

## Error Handling

- **Not logged in:** ABORT with instructions to run `npm login`
- **Missing binaries:** ABORT with link to GitHub Actions
- **Publish fails:** Display npm error output, suggest checking npm status
- **Network issues:** Catch and display connection errors

## After First Publish: Setting Up Trusted Publishing (Optional)

After the first publish succeeds, you can set up Trusted Publishing for future CI releases:

1. Go to https://www.npmjs.com/package/@sengac/codelet-napi/access
2. Click "Configure Trusted Publishing"
3. Add GitHub Actions:
   - Repository: sengac/fspec
   - Workflow: build-codelet-napi.yml
4. Future releases can then be published from CI without tokens
