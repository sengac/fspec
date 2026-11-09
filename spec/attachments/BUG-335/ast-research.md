# BUG-335 AST research

## fspec astgrep --pattern 'super::lookup::find_model($$$A)' --lang rust --path rust/providers/src/models
rust/providers/src/models/registry.rs:93:35:super::lookup::find_model(&provider_info.models, model)


## fspec astgrep --pattern '$R.get_model($$$A)' --lang rust --path rust/providers/src
rust/providers/src/manager_runtime.rs:108:9:registry
rust/providers/src/manager_runtime.rs:110:26:registry.get_model(provider, &metadata.api_model)
rust/providers/src/manager_runtime.rs:111:26:registry.get_model(&metadata.catalog_provider, &metadata.api_model)
rust/providers/src/models/registry.rs:116:26:self.get_model(provider, model)
rust/providers/src/models/registry.rs:379:21:registry.get_model("anthropic", "claude-sonnet-4")
rust/providers/src/models/registry.rs:389:22:registry.get_model("anthropic", "sonnt")


## fspec astgrep --pattern '$R.get_model($$$A)' --lang rust --path rust/sessions/src
rust/sessions/src/cloud_models.rs:263:37:r.get_model(dev_id, model_id)


## fspec astgrep --pattern '$R.validate_model_for_use($$$A)' --lang rust --path rust/providers/src
rust/providers/src/manager/impl_2.rs:56:26:registry
rust/providers/src/manager/impl_2.rs:61:26:registry.validate_model_for_use(&provider_id, &model_id)


## Findings

- `find_model` has one caller, `ModelRegistry::get_model` (registry.rs:93); every catalog
  lookup goes through it: session limit resolution (manager_runtime.rs:108-111), model
  selection validation (`validate_model_for_use`, manager/impl_2.rs:56-61) and the RPC
  `ModelInfo` display lookup (sessions/src/cloud_models.rs:263).
- All of these receive ids that originate from the catalog itself (selector rows) or from a
  user-typed `provider/model` string, so removing the prefix guess turns a near-miss into an
  explicit "not found / Did you mean" error or an unavailable limit, never a wrong model.
