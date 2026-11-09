# PROV-168: Anthropic OAuth request configuration

## Current behavior

- OAuth requests identify themselves with the fixed user agent `claude-cli/2.1.3 (external, cli)` (`rust/providers/src/claude_oauth.rs:54`). Both OAuth client construction paths use that value (`rust/providers/src/claude.rs:299`, `rust/providers/src/claude.rs:398`).
- The Opus 5.5 request fails with HTTP 400 and `claude_code_version_too_old`: the server reports that version `2.1.280` or newer is required.
- OAuth requests use bearer authorization, `x-app: cli`, and model-dependent beta headers (`rust/providers/src/claude.rs:85`).
- Newer `claude-` model IDs default to adaptive thinking (`rust/tools/src/facade/thinking_config.rs:48`). For adaptive models, the selected low, medium, or high level currently does not affect the request configuration (`rust/tools/src/facade/thinking_config.rs:240`).
- The failed turn showed zero token counts and `context unknown`. The latter is displayed when a committed generation has no context threshold (`rust/fspec-tui/src/views/agent/header_build.rs:158`); the reason that threshold was absent has not been established.

## Recommended changes

1. Replace the hard-coded `2.1.3` user-agent version with a maintained, current version source, without a stale hard-coded fallback. Report a clear actionable error when a suitable version is unavailable or rejected. The reported request requires at least `2.1.280`.
2. Preserve bearer authentication and the applicable request headers, and validate the complete request configuration for the target model. Changing the version alone has not been verified to make the request succeed.
3. Verify the target model's supported adaptive-thinking and effort parameters, then map the selected thinking level to the supported request fields rather than silently ignoring it.
4. Add coverage for the version source, missing and outdated versions, model-specific request configuration, and actionable error reporting. Verify the fix with a successful authorized request when possible.
5. Investigate the missing context threshold separately; keep unknown context distinct from a zero token count when a request fails.
