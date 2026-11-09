# REFAC-030 design: split custom-provider config, management and limit tests

## Problem

PROV-170 must change `ModelDef.context_window` / `max_output_tokens` and
`ProviderModelInfo`, and every test that constructs a `ModelDef`. AGENTS.md caps
files at 300 lines, and six of the files PROV-170 touches already exceed it:

| File | Lines | Evidence |
|---|---|---|
| rust/providers/src/custom/config.rs | 358 | `ToolStyle` :81, `ApiStyle` :98, `AuthConfig` :108-163, `ModelDef` :167, `ProviderConfig` :217, `impl ProviderConfig` :276-358 |
| rust/providers/src/custom/management.rs | 572 | `ProviderModelInfo` :34, `list_providers_info` :114-226, `validate_provider_config` :242, `test_provider_connection` :285, `init_provider_template` :325, `apply_custom_provider_env_vars` :383, `derive_facade_for_custom` :441, `resolve_custom_model_id` :476, private helpers :485, :499, :541, :564 |
| rust/providers/tests/rhai_rig_agent_keystone_tests.rs | 758 | fixtures :39-213, nine tests :215-758 (`ModelDef {` at :134, :493) |
| rust/providers/tests/rhai_scripted_model_limits_tests.rs | 594 | fixtures :40-171, ten tests :173-594 (`ModelDef {` at :160, :303, :315) |
| rust/providers/tests/custom_http_lifecycle_tests.rs | 661 | helper :38-42, thirteen tests :44-661 |
| rust/providers/tests/bug_139_custom_provider_per_model_limits_test.rs | 399 | guards/fixture :43-173, four tests :175-399 (`ModelDef {` at :337) |

## Chosen split

Source files become parent modules with child file modules, re-exported with
`pub use`, so every public path (`codelet_providers::custom::{ModelDef,
ProviderConfig, ProviderModelInfo, list_providers_info, ...}` and
`codelet_providers::custom::config::*` / `custom::management::*`) is unchanged.
Code moves verbatim at item boundaries.

- `custom/config.rs` keeps `ModelDef`, `Defaults`, `SystemPromptConfig`,
  `ProviderConfig` + its `impl`, and the serde default functions they use.
  - `custom/config/auth.rs`: `AuthConfig` with its private serde defaults
    (`default_bearer_prefix`, `default_api_key_header`).
  - `custom/config/styles.rs`: `ToolStyle`, `ApiStyle`.
- `custom/management.rs` keeps `ProviderModelInfo`, `ProviderInfo`,
  `ProviderTestResult`, `list_providers_info`, `show_provider_info`, and the
  private helpers shared by several children (`effective_api_key_env_var`,
  `CustomErrorExt`). Child modules can see their parent's private items, so no
  visibility changes are needed.
  - `custom/management/provider_files.rs`: `validate_provider_config`,
    `test_provider_connection`, `init_provider_template` and their private
    helpers `find_provider_config_path`, `openai_compatible_template`.
  - `custom/management/facade_env.rs`: `apply_custom_provider_env_vars`,
    `derive_facade_for_custom`, `resolve_custom_model_id`.
- Each test target keeps its top-level file (same Cargo target name, same
  crate-level `#![allow]`, same imports and fixtures) and loads its test functions
  from `#[path = "<dir>/<name>.rs"] mod <name>;` submodules that start with
  `use super::*;`. Test bodies, comments and `@step` lines are byte-identical;
  test names gain the submodule prefix only.

Coverage mappings in `spec/features/*.feature.coverage` that pointed into the
moved lines are re-pointed by item to the new file and line range.

## Alternatives rejected

- `include!` fragments (REFAC-028's approach for `impl ProviderManager`): needed
  there to keep one `impl` block's privacy; here the items are free-standing, so
  real modules are clearer, rustfmt-visible and keep privacy without tricks.
- Making helpers `pub(super)` and spreading them across children: unnecessary
  visibility churn; keeping shared helpers in the parent avoids it.
- New test targets per group: would change the Cargo target names the brief
  requires to stay the same and duplicate fixtures.
- Changing `ModelDef` now: out of scope (PROV-170).

## API changes

None. No type, field, signature, visibility or behaviour change.
