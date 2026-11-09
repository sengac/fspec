# PROV-176 AST research: where the Codex section is built

Run 2026-11-06 with `fspec astgrep --lang rust --path rust`. Path-qualified calls that the matcher misses are listed from grep at the end.

## `crate::cloud_models::synthesize_codex_section($$$A)`
```
```
## `crate::profile_sections::retain_populated_cloud_sections($$$A)`
```
```
## `crate::profile_sections::cloud_provider_info($$$A)`
```
rust/sessions/src/cloud_models.rs:231:17:crate::profile_sections::cloud_provider_info(

```
## `crate::profile_limits::resolve_missing_profile_limits($$$A)`
```
rust/sessions/src/model_resolution.rs:279:5:crate::profile_limits::resolve_missing_profile_limits(&mut provider_manager, model)

```
## `models_catalog::fetch_for_selection()`
```
```
## `row.is_unreachable`
```
rust/fspec-tui/src/views/model_selector/header.rs:39:28:row.is_unreachable
rust/fspec-tui/src/views/model_selector/header.rs:61:8:row.is_unreachable

```
## `synthesize_codex_section($$$A)`
```
```
## `retain_populated_cloud_sections($$$A)`
```
```
## `fetch_for_selection()`
```
```
## `codex_reparented_models($$$A)`
```
rust/sessions/src/cloud_models.rs:214:24:codex_reparented_models(registry)

```

## grep: call sites astgrep did not match
```
sessions/src/model_resolution.rs:284:    provider_manager.fetch_codex_catalog().await;
sessions/src/handle_impl/handle_impl_5.rs:118:                    crate::cloud_models::synthesize_codex_section(cloud_sections, registry)
sessions/src/handle_impl/handle_impl_5.rs:131:                    crate::profile_sections::retain_populated_cloud_sections(cloud_sections);
sessions/src/handle_impl/handle_impl_5.rs:133:                    crate::profile_sections::retain_populated_cloud_sections(custom_sections);
sessions/src/profile_sections.rs:58:pub fn retain_populated_cloud_sections(sections: Vec<ProviderInfo>) -> Vec<ProviderInfo> {
sessions/src/profile_sections/cloud_section_tests.rs:36:    let kept = super::retain_populated_cloud_sections(sections);
sessions/src/profile_sections/cloud_section_tests.rs:53:    let kept = super::retain_populated_cloud_sections(sections);
sessions/src/profile_sections/cloud_section_tests.rs:76:    let kept = super::retain_populated_cloud_sections(sections);
sessions/src/cloud_models.rs:206:pub fn synthesize_codex_section(
providers/src/codex/models_catalog/fetch.rs:66:pub async fn fetch_for_selection() -> Result<CodexCatalog, String> {
providers/src/manager/codex_catalog.rs:14:        let catalog = crate::codex::models_catalog::fetch_for_selection().await;
```
