# AST research: REFAC-030

## `pub enum $E { $$$B }` in rust/providers/src/custom/config.rs
```
rust/providers/src/custom/config.rs:81:1:pub enum ToolStyle {
rust/providers/src/custom/config.rs:98:1:pub enum ApiStyle {
rust/providers/src/custom/config.rs:108:1:pub enum AuthConfig {

```

## `pub struct $S { $$$B }` in rust/providers/src/custom/config.rs
```
rust/providers/src/custom/config.rs:167:1:pub struct ModelDef {
rust/providers/src/custom/config.rs:192:1:pub struct Defaults {
rust/providers/src/custom/config.rs:206:1:pub struct SystemPromptConfig {
rust/providers/src/custom/config.rs:217:1:pub struct ProviderConfig {

```

## `fn $F() -> $T { $$$B }` in rust/providers/src/custom/config.rs
```
rust/providers/src/custom/config.rs:29:1:fn default_bearer_prefix() -> String {
rust/providers/src/custom/config.rs:34:1:fn default_api_key_header() -> String {
rust/providers/src/custom/config.rs:47:1:fn default_context_window() -> usize {
rust/providers/src/custom/config.rs:52:1:fn default_max_output_tokens() -> usize {
rust/providers/src/custom/config.rs:56:1:fn default_true() -> bool {
rust/providers/src/custom/config.rs:60:1:fn default_tool_style() -> ToolStyle {
rust/providers/src/custom/config.rs:64:1:fn default_api_style() -> ApiStyle {
rust/providers/src/custom/config.rs:72:1:fn default_auth() -> AuthConfig {

```

## `pub struct $S { $$$B }` in rust/providers/src/custom/management.rs
```
rust/providers/src/custom/management.rs:34:1:pub struct ProviderModelInfo {
rust/providers/src/custom/management.rs:57:1:pub struct ProviderInfo {
rust/providers/src/custom/management.rs:102:1:pub struct ProviderTestResult {

```

## `pub fn $F($$$A) -> $R { $$$B }` in rust/providers/src/custom/management.rs
```
rust/providers/src/custom/management.rs:114:1:pub fn list_providers_info() -> Result<Vec<ProviderInfo>, ProviderError> {
rust/providers/src/custom/management.rs:231:1:pub fn show_provider_info(name: &str) -> Result<ProviderInfo, ProviderError> {
rust/providers/src/custom/management.rs:242:1:pub fn validate_provider_config(name: &str) -> Result<(), ProviderError> {
rust/providers/src/custom/management.rs:325:1:pub fn init_provider_template(
rust/providers/src/custom/management.rs:383:1:pub fn apply_custom_provider_env_vars(
rust/providers/src/custom/management.rs:441:1:pub fn derive_facade_for_custom(name: &str) -> Option<String> {
rust/providers/src/custom/management.rs:476:1:pub fn resolve_custom_model_id(provider_name: &str, model_alias: &str) -> Option<String> {

```

## `pub async fn $F($$$A) -> $R { $$$B }` in rust/providers/src/custom/management.rs
```
rust/providers/src/custom/management.rs:285:1:pub async fn test_provider_connection(name: &str) -> Result<ProviderTestResult, ProviderError> {

```

## `fn $F($$$A) -> $R { $$$B }` in rust/providers/src/custom/management.rs
```
rust/providers/src/custom/management.rs:485:1:fn effective_api_key_env_var(cfg: &ProviderConfig) -> Option<&str> {
rust/providers/src/custom/management.rs:499:1:fn find_provider_config_path(name: &str) -> Result<PathBuf, ProviderError> {
rust/providers/src/custom/management.rs:541:1:fn openai_compatible_template(name: &str) -> serde_json::Value {
rust/providers/src/custom/management.rs:569:5:fn to_provider_error(self) -> ProviderError {

```

## `effective_api_key_env_var($$$A)` in rust/providers/src/custom/management.rs
```
rust/providers/src/custom/management.rs:185:23:effective_api_key_env_var(&cfg)
rust/providers/src/custom/management.rs:403:36:effective_api_key_env_var(&cfg)
rust/providers/src/custom/management.rs:414:36:effective_api_key_env_var(&cfg)

```

## `find_provider_config_path($$$A)` in rust/providers/src/custom/management.rs
```
rust/providers/src/custom/management.rs:243:23:find_provider_config_path(name)
rust/providers/src/custom/management.rs:286:23:find_provider_config_path(name)

```

## `CustomErrorExt::to_provider_error` in rust/providers/src/custom/management.rs
```
rust/providers/src/custom/management.rs:182:55:CustomErrorExt::to_provider_error
rust/providers/src/custom/management.rs:279:61:CustomErrorExt::to_provider_error
rust/providers/src/custom/management.rs:287:63:CustomErrorExt::to_provider_error
rust/providers/src/custom/management.rs:388:55:CustomErrorExt::to_provider_error

```

## `use super::config::$$$I;` in rust/providers/src/custom
```
rust/providers/src/custom/discovery.rs:7:1:use super::config::ProviderConfig;
rust/providers/src/custom/tool_resolve.rs:12:1:use super::config::ProviderConfig;
rust/providers/src/custom/tool_presets.rs:11:1:use super::config::ToolStyle;
rust/providers/src/custom/tool_facade.rs:23:1:use super::config::ProviderConfig;
rust/providers/src/custom/custom_provider.rs:25:1:use super::config::ProviderConfig;

```

## `ModelDef { $$$F }` in rust/providers/tests/rhai_rig_agent_keystone_tests.rs
```
rust/providers/tests/rhai_rig_agent_keystone_tests.rs:134:9:ModelDef {
rust/providers/tests/rhai_rig_agent_keystone_tests.rs:493:9:ModelDef {

```

## `ModelDef { $$$F }` in rust/providers/tests/rhai_scripted_model_limits_tests.rs
```
rust/providers/tests/rhai_scripted_model_limits_tests.rs:160:9:ModelDef {
rust/providers/tests/rhai_scripted_model_limits_tests.rs:303:9:ModelDef {
rust/providers/tests/rhai_scripted_model_limits_tests.rs:315:9:ModelDef {

```

## `ModelDef { $$$F }` in rust/providers/tests/custom_http_lifecycle_tests.rs
```
No matches found
```

## `ModelDef { $$$F }` in rust/providers/tests/bug_139_custom_provider_per_model_limits_test.rs
```
rust/providers/tests/bug_139_custom_provider_per_model_limits_test.rs:337:9:ModelDef {

```

