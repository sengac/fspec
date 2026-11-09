# Anthropic subscription OAuth protocol

Anthropic OAuth requests use the release reported by `claude --version` on
`PATH`. Updating the installed Claude Code updates fspec's advertised release
without rebuilding fspec. fspec does not bundle a replacement release string.

Install Claude Code if it is unavailable; use `claude update` to update an old
installation. The compatibility baseline is `2.1.280`, the minimum reported in
the Opus 5.5 incident. Anthropic may enforce a newer minimum for a model or an
organization; fspec preserves that rejection and its request identifier.

For deployments that manage the compatibility release separately, explicitly set
`FSPEC_CLAUDE_CODE_VERSION` to the supported `major.minor.patch` release managed
by your deployment. This override is authoritative: invalid, empty, non-release,
or too-old values fail rather than falling back to the installed executable.
Remove the override to resume using the installed release. A manually managed
override must be kept current by the deployment operator.

This requirement applies only to subscription OAuth. Ordinary Anthropic API keys
do not require Claude Code or the override.

## Thinking and effort

For adaptive-thinking models, including Opus 5.5, fspec maps low, medium, and high
to `output_config.effort`. Requests use adaptive thinking with summarized display
and no manual `budget_tokens`. Off adds neither thinking nor effort configuration.
Legacy budgeted models retain their existing token budgets.

fspec preserves bearer authorization, the Messages API version and beta query,
`x-app: cli`, applicable OAuth betas, and its Claude Code system identity prefix.
It does not advertise optional SDK context editing, per-turn controls, extended
cache TTL, or fabricate SDK billing/account metadata.

## Failed requests and context

A rejection before usage arrives cannot establish measured occupancy. A new
session may therefore show zero token counters and `[context unknown]` even
though the configured model capacity is known. This is distinct from a measured
zero-percent fill; fspec does not invent usage on failure.

## Verification

The scoped provider tests use local HTTP fixtures and isolated executable/version
sources. A deliberately opt-in authorized smoke check uses stored fspec OAuth
credentials, sends `what is 3 + 2` to Opus 5.5 with medium effort, and verifies
nonzero usage. It may refresh and persist expired credentials, and consumes
subscription usage:

```sh
cd rust
FSPEC_PROV168_LIVE=1 cargo test --profile ci-test -p codelet-providers \
  --features test-support --test prov168_oauth_request \
  current_model_requests_preserve_the_oauth_contract
```

Protocol investigation and official documentation references are recorded in
`spec/attachments/PROV-168/protocol-investigation.md`.
