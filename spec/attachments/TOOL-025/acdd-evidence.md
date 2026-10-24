# TOOL-025 ACDD evidence

## Gate and specification

SESS-003, BUG-230 and BUG-231 were verified done before TOOL-025 implementation.
Only TOOL-025 workflow state was managed by this owner. The current contract requires
a reserved top-level `work_unit_id` string and strict JSON/native handling. Reject
existing business root `work_unit_id` collisions; business targets remain nested or
use `target_work_unit_id`. No wrapper compatibility is supported.
AST research is attached in `ast-research-middleware.md`.

The logs below record prior implementation validation, not a rerun of the revised
flat-field contract. This documentation update does not establish new test results.

The original six scenarios were retained. The provider scenarios were relocated to
`mandatory-tool-work-unit-provider-contract.feature` because fspec enforces one
feature per test target. There are exactly four schema acceptance tests and two
provider acceptance tests. Additional regression matrices live in support modules
called inside those tests, not extra unmapped acceptance targets.

## Meaningful RED before production

All commands used package/target scopes, `--profile ci-test` and `-j4` from `rust/`.

- `/tmp/tool025-schema-red.log`: four original ToolServer boundary tests compile
  and fail because required metadata is absent and a reserved business collision is advertised.
  The later configuration API was added to fixture setup without removing the
  original assertions. Missing symbols were not used as behavioral RED evidence.
- `/tmp/tool025-provider-red.log`: serialized provider requests omit attribution;
  Codex exposes raw/hosted tools instead of attributed JSON functions.
- `/tmp/tool025-custom-red.log`: custom direct and Rig request paths lose metadata.
- `/tmp/tool025-custom-catalog-red.log`: unsupported executable catalogs pass.
- `/tmp/tool025-schema-constraints-red.log`: unsafe dependentRequired extension.
- `/tmp/tool025-nullable-red.log`: nullable root bypasses required metadata.
- `/tmp/tool025-anchor-red.log`: anchored root recursion changes business semantics.
- `/tmp/tool025-hot-replacement-red.log`: replacing a hot tool duplicates its catalog entry.
- `/tmp/tool025-merged-catalog-red.log`: final OpenAI chat, Anthropic and Gemini
  options replace the decorated catalog with unattributed hosted execution.
- `/tmp/tool025-mcp-red.log`: actual loopback MCP connection reports success for a
  reserved collision; actor rejection fails to report registration failure.
- `/tmp/tool025-provider-final-regression.log`: real Gemini catalog comparison
  exposes the lossy OpenAPI Schema conversion. Original JSON Schema now travels
  through the provider's mutually exclusive `parametersJsonSchema` field.

Custom business-schema mutation regressions also precede the full-schema equality
fix: changing business required/type/additionalProperties cannot preserve a valid
catalog merely by retaining the metadata fragment.

## Final GREEN

```sh
cargo test --profile ci-test -j4 -p codelet-tools --test tool025_schema_contract
cargo test --profile ci-test -j4 -p codelet-providers --test tool025_provider_transports
```

Results: four plus two tests pass. Logs: `/tmp/tool025-schema-final.log` and
`/tmp/tool025-provider-final-single.log`.

These tests include 64 generated preservation cases, raw-schema repeatability,
forged harness-shaped collision rejection, strict business references and unions,
unsafe root constraints, embedding/vector selection, immutable scope binding,
default/callback delegation, hot Add/Append/replacement and rollback, and actual
loopback MCP startup/hot registration/failure notification. The provider matrix
uses 16 real main/custom/auxiliary agent catalogs, completion tools and facade
definitions, plus final serializers and post-Rhai destructive-catalog checks.

Real ordinary CLI and compactor request captures assert required metadata without
manual middleware setup by the tests:

```sh
cargo test --profile ci-test -j4 -p codelet-cli --test codex_accepted_completion_batch
cargo test --profile ci-test -j4 -p codelet-agent-loop --test bug211_synthesis_admission scenario_synthesis_respects_provider_requirements
```

Both pass: `/tmp/tool025-cli-final.log`, `/tmp/tool025-aux-wire-final.log`.

Six existing provider regression targets pass, 18 tests total:
`cmpct071_builtin_serializers`, `cmpct071_custom_serializer`,
`prov149_codex_history`, `prov157_codex_custom_tools`,
`prov158_codex_hosted_transport`, `prov158_codex_native_transport`.
Log: `/tmp/tool025-final-provider-regressions.log`.
`codelet-agent-loop --test prov149_codex_reload` passes three tests:
`/tmp/tool025-reload-regression.log`.
The MCP worker additionally ran 68 existing MCP tests successfully.

## Quality and coverage

- Scoped check succeeds for CLI, agent-loop and NAPI, including both live NAPI
  DeepSearch branches: `/tmp/tool025-routes-check.log`.
- Scoped Clippy `--lib -- -D warnings` succeeds for rig-core, tools, providers,
  core, CLI and agent-loop: `/tmp/tool025-final-clippy.log`.
- Schema integration-target Clippy succeeds with `-D warnings`:
  `/tmp/tool025-schema-test-clippy.log`. Provider target Clippy also passes.
- NAPI Clippy succeeds with seven existing warnings:
  `/tmp/tool025-napi-clippy.log`; none are suppressed or repaired in this card.
- Targeted rustfmt check and `git diff --check` pass.
- All new files, including the two coverage sidecars, stay below 300 lines.
- Both feature coverage reports are 100%, with exactly one test mapping and
  implementation mappings per scenario. Both coverage audits report valid files.
- All 2,053 feature files validate: `/tmp/tool025-feature-validation.log`.
- Tag validation reports 1,527 passing files and 526 existing violations elsewhere:
  `/tmp/tool025-tag-validation.log`. Neither TOOL-025 feature is in the violations.

Broader compaction regression runs encounter a fixture calibration assertion in
`agent-loop/tests/support/bug210_completion_metrics.rs:109` (11371 versus 90336),
before reaching the compactor. Logs: `/tmp/tool025-aux-regressions.log` and
`/tmp/tool025-aux-synthesis.log`. No unrelated fixture/token-estimation changes were
made. The provider-compatible compaction scenario passes independently.

Independent read-only reviewer `0774b764-439a-48d5-b262-4d89e3a1ba5f` found no
remaining confirmed blocker and cleared closure after inspecting final code/logs.
No commits, staging or branches were created.

## Scope boundary

TOOL-025 installs schema middleware universally but deliberately does not admit
activity or strip metadata from business execution. Runtime validation, trusted
project/session lookup, permission-preserving focus changes and BUG-231 receipt
scoping belong to TOOL-026. See `middleware-api.md` for its concrete handoff.
