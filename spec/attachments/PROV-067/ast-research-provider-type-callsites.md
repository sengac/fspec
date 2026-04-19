codelet/providers/tests/copilot_provider_manager_integration_test.rs:8://! provider dispatch layer (`ProviderType::GitHubCopilot`,
codelet/providers/tests/copilot_provider_manager_integration_test.rs:402:        ProviderType::from_str("github-copilot").expect("should parse"),
codelet/providers/tests/copilot_provider_manager_integration_test.rs:403:        ProviderType::GitHubCopilot,
codelet/providers/tests/copilot_provider_manager_integration_test.rs:409:    assert_eq!(ProviderType::GitHubCopilot.as_str(), "github-copilot");
codelet/providers/tests/session_affinity_integration_test.rs:136:    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, None, None);
codelet/providers/tests/session_affinity_integration_test.rs:214:    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, None, None);
codelet/providers/tests/stop_reason_propagation_test.rs:233:    let manager = ProviderManager::for_testing(ProviderType::OpenAI, None, None);
codelet/providers/tests/stop_reason_propagation_test.rs:259:    let manager = ProviderManager::for_testing(ProviderType::OpenAI, None, None);
codelet/providers/tests/stop_reason_propagation_test.rs:275:    let manager = ProviderManager::for_testing(ProviderType::OpenAI, None, None);
codelet/providers/src/copilot/provider.rs:11://! [`ProviderType::GitHubCopilot`]: crate::ProviderType::GitHubCopilot
codelet/providers/src/model_limits.rs:903:            ProviderType::Claude,
codelet/providers/src/model_limits.rs:913:            ProviderType::OpenAI,
codelet/providers/src/model_limits.rs:922:            ProviderType::Gemini,
codelet/providers/src/model_limits.rs:931:            ProviderType::Codex,
codelet/providers/src/model_limits.rs:940:            ProviderType::ZAI,
codelet/providers/src/model_limits.rs:949:            ProviderType::GitHubCopilot,
codelet/providers/src/model_limits.rs:965:            ProviderType::Claude,
codelet/providers/src/model_limits.rs:1028:            ProviderType::Claude,
codelet/providers/src/model_limits.rs:1045:            ProviderType::OpenAI,
codelet/providers/src/model_limits.rs:1058:            ProviderType::GitHubCopilot,
codelet/providers/src/model_limits.rs:1070:            ProviderType::ZAI,
codelet/providers/src/model_limits.rs:1082:            ProviderType::Codex,
codelet/providers/src/model_limits.rs:1098:            ProviderType::OpenAI,
codelet/providers/src/manager.rs:37:            "claude" => Ok(ProviderType::Claude),
codelet/providers/src/manager.rs:38:            "openai" => Ok(ProviderType::OpenAI),
codelet/providers/src/manager.rs:39:            "codex" => Ok(ProviderType::Codex),
codelet/providers/src/manager.rs:40:            "gemini" => Ok(ProviderType::Gemini),
codelet/providers/src/manager.rs:41:            "zai" => Ok(ProviderType::ZAI),
codelet/providers/src/manager.rs:42:            "github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot),
codelet/providers/src/manager.rs:55:            ProviderType::Claude => "claude",
codelet/providers/src/manager.rs:56:            ProviderType::OpenAI => "openai",
codelet/providers/src/manager.rs:57:            ProviderType::Codex => "codex",
codelet/providers/src/manager.rs:58:            ProviderType::Gemini => "gemini",
codelet/providers/src/manager.rs:59:            ProviderType::ZAI => "zai",
codelet/providers/src/manager.rs:60:            ProviderType::GitHubCopilot => "github-copilot",
codelet/providers/src/manager.rs:69:            ProviderType::Claude => credentials.has_claude(),
codelet/providers/src/manager.rs:70:            ProviderType::OpenAI => credentials.has_openai(),
codelet/providers/src/manager.rs:71:            ProviderType::Codex => credentials.has_codex(),
codelet/providers/src/manager.rs:72:            ProviderType::Gemini => credentials.has_gemini(),
codelet/providers/src/manager.rs:73:            ProviderType::ZAI => credentials.has_zai(),
codelet/providers/src/manager.rs:74:            ProviderType::GitHubCopilot => credentials.has_github_copilot(),
codelet/providers/src/manager.rs:189:        let requested_provider = ProviderType::from_str(provider_name)?;
codelet/providers/src/manager.rs:234:        let requested_provider = ProviderType::from_str(provider_name)?;
codelet/providers/src/manager.rs:466:            "anthropic" => Ok(ProviderType::Claude),
codelet/providers/src/manager.rs:467:            "openai" => Ok(ProviderType::OpenAI),
codelet/providers/src/manager.rs:468:            "google" => Ok(ProviderType::Gemini),
codelet/providers/src/manager.rs:469:            "zai" | "z-ai" => Ok(ProviderType::ZAI),
codelet/providers/src/manager.rs:470:            "codex" => Ok(ProviderType::Codex),
codelet/providers/src/manager.rs:471:            "github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot),
codelet/providers/src/manager.rs:487:            return Ok(ProviderType::Claude);
codelet/providers/src/manager.rs:490:            return Ok(ProviderType::Gemini);
codelet/providers/src/manager.rs:493:            return Ok(ProviderType::ZAI);
codelet/providers/src/manager.rs:496:            return Ok(ProviderType::Codex);
codelet/providers/src/manager.rs:499:            return Ok(ProviderType::GitHubCopilot);
codelet/providers/src/manager.rs:502:            return Ok(ProviderType::OpenAI);
codelet/providers/src/manager.rs:522:        if self.current_provider == ProviderType::Claude {
codelet/providers/src/manager.rs:555:        if self.current_provider != ProviderType::OpenAI {
codelet/providers/src/manager.rs:581:        if self.current_provider != ProviderType::Codex {
codelet/providers/src/manager.rs:600:        if self.current_provider != ProviderType::Gemini {
codelet/providers/src/manager.rs:639:        if self.current_provider != ProviderType::GitHubCopilot {
codelet/providers/src/manager.rs:713:        let requested_provider = ProviderType::from_str(provider_name)?;
codelet/providers/src/manager.rs:741:            ProviderType::Claude => Box::new(ConstantResolver {
codelet/providers/src/manager.rs:747:            ProviderType::OpenAI => {
codelet/providers/src/manager.rs:764:            ProviderType::Gemini => Box::new(ConstantResolver {
codelet/providers/src/manager.rs:770:            ProviderType::Codex => Box::new(ConstantResolver {
codelet/providers/src/manager.rs:776:            ProviderType::ZAI => Box::new(ConstantResolver {
codelet/providers/src/manager.rs:782:            ProviderType::GitHubCopilot => Box::new(ConstantResolver {
codelet/providers/src/manager.rs:931:        if self.current_provider != ProviderType::ZAI {
codelet/providers/src/manager.rs:1093:            current_provider: ProviderType::Claude,
codelet/providers/src/manager.rs:1194:        let manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1217:        let manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1389:            current_provider: ProviderType::Claude,
codelet/providers/src/manager.rs:1499:        let manager = test_manager(ProviderType::Claude);
codelet/providers/src/manager.rs:1521:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1540:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1558:        let manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1590:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1620:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1647:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1674:            ProviderType::OpenAI,
codelet/providers/src/manager.rs:1805:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1830:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1883:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1911:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1930:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1949:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:1969:        let mgr = test_manager(ProviderType::Claude);
codelet/providers/src/manager.rs:1973:        let mgr = ProviderManager::for_testing(ProviderType::OpenAI, None, None);
codelet/providers/src/manager.rs:1986:        let mut manager = test_manager(ProviderType::Claude);
codelet/providers/src/manager.rs:2005:        let mut manager = test_manager(ProviderType::Claude);
codelet/providers/src/manager.rs:2024:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:2043:        let manager = test_manager(ProviderType::Codex);
codelet/providers/src/manager.rs:2062:        let mut manager = test_manager(ProviderType::Claude);
codelet/providers/src/manager.rs:2083:        let mut manager = test_manager(ProviderType::OpenAI);
codelet/providers/src/manager.rs:2102:        let mut manager = test_manager(ProviderType::Claude);
codelet/providers/src/manager.rs:2122:        let manager = test_manager(ProviderType::OpenAI);
codelet/tests/project_scaffold_test.rs:20:    let _ = ProviderType::Claude;
codelet/tests/project_scaffold_test.rs:98:    let _claude = ProviderType::Claude;
codelet/tests/project_scaffold_test.rs:99:    let _openai = ProviderType::OpenAI;
codelet/tests/project_scaffold_test.rs:100:    let _gemini = ProviderType::Gemini;
codelet/napi/src/session_manager.rs:7849:    /// `ProviderType::from_str`. `registry_provider` is the models.dev name

## Summary
Total ProviderType:: references: 101

## ProviderCredentials call sites
codelet/providers/tests/copilot_select_model_stale_cache_test.rs:14:// `with_model_support()` snapshots `ProviderCredentials::detect()` once at
codelet/providers/tests/copilot_select_model_stale_cache_test.rs:17:// `ProviderCredentials::detect()` AGAIN as its first action so the new
codelet/providers/tests/copilot_select_model_stale_cache_test.rs:27:use codelet_providers::{ProviderCredentials, ProviderManager};
codelet/providers/tests/copilot_select_model_stale_cache_test.rs:45:    let creds_at_construction = ProviderCredentials::detect();
codelet/providers/tests/copilot_select_model_stale_cache_test.rs:68:    // immediately calls ProviderCredentials::detect() — which is exactly the
codelet/providers/tests/copilot_select_model_stale_cache_test.rs:80:    // @step Then ProviderCredentials::detect() is re-invoked before the has_credentials check
codelet/providers/tests/claude_oauth_routing_test.rs:18:use codelet_providers::ProviderCredentials;
codelet/providers/tests/claude_oauth_routing_test.rs:80:    let credentials = ProviderCredentials::detect();
codelet/providers/tests/claude_oauth_routing_test.rs:113:    let credentials = ProviderCredentials::detect();
codelet/providers/tests/copilot_provider_manager_integration_test.rs:9://! `ProviderCredentials::detect`, `ProviderManager::with_provider`,
codelet/providers/tests/copilot_provider_manager_integration_test.rs:25:use codelet_providers::{ProviderCredentials, ProviderManager, ProviderType};
codelet/providers/tests/copilot_provider_manager_integration_test.rs:56:    let creds_before = ProviderCredentials::detect();
codelet/providers/tests/copilot_provider_manager_integration_test.rs:77:    let creds_after = ProviderCredentials::detect();
codelet/providers/tests/copilot_provider_manager_integration_test.rs:292:    assert!(ProviderCredentials::detect().has_github_copilot());
codelet/providers/tests/copilot_provider_manager_integration_test.rs:301:    let creds = ProviderCredentials::detect();
codelet/providers/tests/copilot_provider_manager_integration_test.rs:414:    let creds = ProviderCredentials {
codelet/providers/src/credentials.rs:9:pub struct ProviderCredentials {
codelet/providers/src/credentials.rs:18:impl ProviderCredentials {
codelet/providers/src/lib.rs:53:pub use credentials::ProviderCredentials;
codelet/providers/src/manager.rs:10:use super::credentials::ProviderCredentials;
codelet/providers/src/manager.rs:67:    pub fn has_credentials(self, credentials: &ProviderCredentials) -> bool {
codelet/providers/src/manager.rs:113:    credentials: ProviderCredentials,
codelet/providers/src/manager.rs:160:        let credentials = ProviderCredentials::detect();
codelet/providers/src/manager.rs:188:        let credentials = ProviderCredentials::detect();
codelet/providers/src/manager.rs:233:        let credentials = ProviderCredentials::detect();
codelet/providers/src/manager.rs:268:        let credentials = ProviderCredentials::detect();
codelet/providers/src/manager.rs:310:        // `ProviderCredentials::detect()` once at construction, which means
codelet/providers/src/manager.rs:317:        self.credentials = ProviderCredentials::detect();
codelet/providers/src/manager.rs:483:        credentials: &ProviderCredentials,
codelet/providers/src/manager.rs:906:            credentials: ProviderCredentials {
codelet/providers/src/manager.rs:985:            credentials: ProviderCredentials {
codelet/providers/src/manager.rs:1085:            credentials: ProviderCredentials {
codelet/providers/src/manager.rs:1109:    // Bug: ProviderManager snapshots ProviderCredentials::detect() once at
codelet/providers/src/manager.rs:1142:        // @step Then ProviderCredentials::detect() is re-invoked before the has_credentials check
codelet/providers/src/manager.rs:1381:            credentials: ProviderCredentials {

## facade_override call sites
codelet/providers/src/manager.rs:136:    facade_override: Option<String>,
codelet/providers/src/manager.rs:181:            facade_override: None,
codelet/providers/src/manager.rs:213:            facade_override: None,
codelet/providers/src/manager.rs:258:            facade_override: None,
codelet/providers/src/manager.rs:293:            facade_override: None,
codelet/providers/src/manager.rs:374:    /// MODEL-004: Accepts optional facade_override for custom models that need
codelet/providers/src/manager.rs:383:    /// * `facade_override` - Optional provider name to dispatch to instead of provider_id
codelet/providers/src/manager.rs:390:        facade_override: Option<String>,
codelet/providers/src/manager.rs:410:        self.facade_override = facade_override;
codelet/providers/src/manager.rs:872:    pub fn facade_override(&self) -> Option<&str> {
codelet/providers/src/manager.rs:873:        self.facade_override.as_deref()
codelet/providers/src/manager.rs:877:    pub fn set_facade_override(&mut self, facade: Option<String>) {
codelet/providers/src/manager.rs:878:        self.facade_override = facade;
codelet/providers/src/manager.rs:921:            facade_override: None,
codelet/providers/src/manager.rs:1000:            facade_override: None,
codelet/providers/src/manager.rs:1100:            facade_override: None,
codelet/providers/src/manager.rs:1396:            facade_override: None,
codelet/providers/src/manager.rs:1909:    fn test_set_model_direct_stores_facade_override() {
codelet/providers/src/manager.rs:1913:        // @step When set_model_direct is called with facade_override=Some("codex")
codelet/providers/src/manager.rs:1923:        // @step Then facade_override() should return Some("codex")
codelet/providers/src/manager.rs:1924:        assert_eq!(manager.facade_override(), Some("codex"));
codelet/providers/src/manager.rs:1932:        // @step When set_model_direct is called with facade_override=None
codelet/providers/src/manager.rs:1942:        // @step Then facade_override() should return None
codelet/providers/src/manager.rs:1943:        assert_eq!(manager.facade_override(), None);
codelet/providers/src/manager.rs:1947:    fn test_set_facade_override_setter() {
codelet/providers/src/manager.rs:1950:        assert_eq!(manager.facade_override(), None);
codelet/providers/src/manager.rs:1952:        // @step When set_facade_override is called with Some("gemini")
codelet/providers/src/manager.rs:1953:        manager.set_facade_override(Some("gemini".to_string()));
codelet/providers/src/manager.rs:1955:        // @step Then facade_override() should return Some("gemini")
codelet/providers/src/manager.rs:1956:        assert_eq!(manager.facade_override(), Some("gemini"));
codelet/providers/src/manager.rs:1958:        // @step When set_facade_override is called with None
codelet/providers/src/manager.rs:1959:        manager.set_facade_override(None);
codelet/providers/src/manager.rs:1961:        // @step Then facade_override() should return None
codelet/providers/src/manager.rs:1962:        assert_eq!(manager.facade_override(), None);
codelet/providers/src/manager.rs:1966:    fn test_facade_override_initialized_none_in_all_constructors() {
codelet/providers/src/manager.rs:1970:        assert_eq!(mgr.facade_override(), None);
codelet/providers/src/manager.rs:1974:        assert_eq!(mgr.facade_override(), None);
codelet/napi/src/session_manager.rs:4345:/// MODEL-004: Uses facade_override() when available, matching the agent_loop
codelet/napi/src/session_manager.rs:4352:    // BUG-132/MODEL-004: Check facade_override first — if set, dispatch to that
codelet/napi/src/session_manager.rs:4355:        .facade_override()
codelet/napi/src/session_manager.rs:4412:/// to verify the facade_override logic and value extraction without needing
codelet/napi/src/session_manager.rs:4418:    let provider = pm.facade_override()
codelet/napi/src/session_manager.rs:4845:                // MODEL-004: Check facade_override first — if set, dispatch to that
codelet/napi/src/session_manager.rs:4849:                    .facade_override()
codelet/napi/src/session_manager.rs:6824:/// MODEL-004: Accepts optional facade_override for custom models that need
codelet/napi/src/session_manager.rs:6828:pub async fn session_set_model_profile(session_id: String, provider_id: String, model_id: String, context_window: Option<u32>, max_output_tokens: Option<u32>, facade_override: Option<String>, compaction_threshold_type: Option<String>, compaction_threshold_value: Option<u32>) -> Result<()> {
codelet/napi/src/session_manager.rs:6829:    tracing::debug!("session_set_model_profile called: session_id={}, provider_id={}, model_id={}, context_window={:?}, max_output_tokens={:?}, facade_override={:?}, compaction_threshold_type={:?}, compaction_threshold_value={:?}",
codelet/napi/src/session_manager.rs:6830:          session_id, provider_id, model_id, context_window, max_output_tokens, facade_override, compaction_threshold_type, compaction_threshold_value);
codelet/napi/src/session_manager.rs:6839:    // MODEL-004: Pass facade_override through to ProviderManager
codelet/napi/src/session_manager.rs:6856:        facade_override,
codelet/napi/src/session_manager.rs:7835:    //! - facade_override is checked before current_provider_name
codelet/napi/src/session_manager.rs:7925:    // Scenario: DeepSearch respects facade_override for custom models
codelet/napi/src/session_manager.rs:7929:    fn test_bug132_deep_search_respects_facade_override() {
codelet/napi/src/session_manager.rs:7930:        // @step Given a session was created with a MODEL-004 custom model registered under "openai" with facade_override "claude"
codelet/napi/src/session_manager.rs:7932:        pm.set_facade_override(Some("claude".to_string()));
codelet/napi/src/session_manager.rs:7938:        assert_eq!(provider, "claude", "facade_override should take precedence over current_provider_name");
codelet/napi/src/session_manager.rs:8033:    // Additional: Verify facade_override=None falls through to current_provider
codelet/napi/src/session_manager.rs:8037:    fn test_bug132_no_facade_override_uses_current_provider() {
codelet/napi/src/session_manager.rs:8038:        // When facade_override is None, extract_deep_search_handler_values
codelet/napi/src/session_manager.rs:8041:        assert!(pm.facade_override().is_none());

## set_model_direct call sites
codelet/providers/tests/session_affinity_integration_test.rs:138:        .set_model_direct("openai", "test-model", None, None, None)
codelet/providers/tests/session_affinity_integration_test.rs:139:        .expect("set_model_direct should succeed");
codelet/providers/tests/session_affinity_integration_test.rs:216:        .set_model_direct("openai", "gpt-4o", None, None, None)
codelet/providers/tests/session_affinity_integration_test.rs:217:        .expect("set_model_direct should succeed");
codelet/providers/src/manager.rs:384:    pub fn set_model_direct(
codelet/providers/src/manager.rs:1584:    // Scenario: set_model_direct stores optional context params
codelet/providers/src/manager.rs:1588:    fn test_set_model_direct_stores_optional_context_params() {
codelet/providers/src/manager.rs:1593:        let result = manager.set_model_direct(
codelet/providers/src/manager.rs:1602:        // @step Then set_model_direct stores model_context_window=32000 and model_max_output_tokens=4096
codelet/providers/src/manager.rs:1623:        let result = manager.set_model_direct(
codelet/providers/src/manager.rs:1632:        // @step Then set_model_direct stores model_context_window=272000 and model_max_output_tokens=4096
codelet/providers/src/manager.rs:1641:    // Scenario: set_model_direct without context params leaves None
codelet/providers/src/manager.rs:1645:    fn test_set_model_direct_without_context_params_leaves_none() {
codelet/providers/src/manager.rs:1649:        // @step When set_model_direct is called without context params
codelet/providers/src/manager.rs:1650:        let result = manager.set_model_direct(
codelet/providers/src/manager.rs:1886:        // (Simulated: direct call to set_model_direct with context params)
codelet/providers/src/manager.rs:1889:        let result = manager.set_model_direct(
codelet/providers/src/manager.rs:1909:    fn test_set_model_direct_stores_facade_override() {
codelet/providers/src/manager.rs:1913:        // @step When set_model_direct is called with facade_override=Some("codex")
codelet/providers/src/manager.rs:1914:        let result = manager.set_model_direct(
codelet/providers/src/manager.rs:1928:    fn test_set_model_direct_without_facade_leaves_none() {
codelet/providers/src/manager.rs:1932:        // @step When set_model_direct is called with facade_override=None
codelet/providers/src/manager.rs:1933:        let result = manager.set_model_direct(
codelet/napi/src/session_manager.rs:3360:            // Profile model: use set_model_direct to bypass registry validation
codelet/napi/src/session_manager.rs:3362:            tracing::info!("PROV-007: Profile model detected, using set_model_direct for {}", model);
codelet/napi/src/session_manager.rs:3363:            provider_manager.set_model_direct(registry_provider, model_part, None, None, None)
codelet/napi/src/session_manager.rs:3367:            tracing::info!("PROV-018: Codex model detected, using set_model_direct for {}", model);
codelet/napi/src/session_manager.rs:3368:            provider_manager.set_model_direct(registry_provider, model_part, None, None, None)
codelet/napi/src/session_manager.rs:6765:        inner.provider_manager_mut().set_model_direct(
codelet/napi/src/session_manager.rs:6837:    // Use set_model_direct which skips registry validation
codelet/napi/src/session_manager.rs:6851:    match inner.provider_manager_mut().set_model_direct(
codelet/napi/src/session_manager.rs:7845:    /// then `set_model_direct()` to configure the model, context_window, and
codelet/napi/src/session_manager.rs:7850:    /// (e.g. "anthropic", "google") used by `set_model_direct`.
codelet/napi/src/session_manager.rs:7862:            pm.set_model_direct(registry_provider, mid, context_window, max_output_tokens, None)
codelet/napi/src/session_manager.rs:7863:                .expect("set_model_direct should succeed for test provider");
codelet/napi/src/session_manager.rs:8014:        // set_model_direct is what session_set_model_profile calls
codelet/napi/src/session_manager.rs:8016:        pm_after.set_model_direct(
codelet/napi/src/session_manager.rs:8022:        ).expect("set_model_direct should succeed");
