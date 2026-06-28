// Session management for persistent context in multi-turn conversations
//
// This module implements the SessionWrapper pattern matching codelet's architecture:
// - Session owns ProviderManager and messages Vec
// - Messages persist across REPL iterations
// - Provider switching clears context
// - Interruption support with snapshot/restore
//
// Reference: spec/features/persistent-context-in-multi-turn-system.feature

pub mod context_gathering; // CLI-016: Context gathering with CLAUDE.md discovery
pub mod system_reminders; // CLI-012: System-reminder infrastructure

pub use system_reminders::SystemReminderType;

use anyhow::Result;
use codelet_core::compaction::{ConversationTurn, TokenTracker};
use codelet_providers::ProviderManager;
use system_reminders::add_system_reminder;

use codelet_core::compaction::StructuralAnnotation;

/// Session manages persistent context across multi-turn conversations
///
/// Matches codelet's REPL scope pattern where messages array lives in
/// function scope and persists across iterations via closure capture.
///
/// CRITICAL: Uses rig::message::Message for direct compatibility with rig's .with_history() API (CLI-008)
/// CLI-009: Added turns and token_tracker for context compaction
#[derive(Debug)]
pub struct Session {
    /// Provider manager for LLM access
    provider_manager: ProviderManager,

    /// Message history - single source of truth for conversation context
    /// Persists across REPL iterations, cleared on provider switch
    /// Uses rig::message::Message directly for rig integration (CLI-008)
    pub messages: Vec<rig::message::Message>,

    /// Conversation turns for compaction analysis (CLI-009)
    /// Grouped messages representing user/assistant exchanges
    pub turns: Vec<ConversationTurn>,

    /// Token tracker for cache-aware compaction (CLI-009)
    /// Tracks cumulative token usage across conversation
    pub token_tracker: TokenTracker,

    /// Per-turn structural annotations for SessionSearch navigation.
    /// Maps message index (assistant message) → annotations detected for that turn.
    /// Consumed by the persistence layer when writing StoredMessage metadata.
    pub annotations: std::collections::HashMap<usize, Vec<StructuralAnnotation>>,

    /// PROV-041: Count of thinking exhaustion events across turns (not retries).
    /// When this exceeds a threshold (3), the session-level reasoning effort is
    /// progressively downgraded. Resets to 0 after each downgrade.
    /// Persists across turns within the same session.
    pub thinking_exhaustion_cross_turn_count: u32,

    /// PROV-041: Current session-level thinking level for progressive degradation.
    /// Starts at High and downgrades on repeated cross-turn exhaustion events.
    pub session_thinking_level: codelet_tools::facade::ThinkingLevel,
}

impl Session {
    /// Create a new session with the specified provider
    ///
    /// # Arguments
    /// * `provider_name` - Optional provider name (defaults to first available)
    ///
    /// # Returns
    /// * `Result<Session>` - New session or error if provider unavailable
    pub fn new(provider_name: Option<&str>) -> Result<Self> {
        let provider_manager = if let Some(name) = provider_name {
            ProviderManager::with_provider(name)?
        } else {
            ProviderManager::new()?
        };

        Ok(Self {
            provider_manager,
            messages: Vec::new(),
            turns: Vec::new(),
            token_tracker: TokenTracker::default(),
            annotations: std::collections::HashMap::new(),
            thinking_exhaustion_cross_turn_count: 0,
            session_thinking_level: codelet_tools::facade::ThinkingLevel::High,
        })
    }

    /// MODEL-001: Create a new session from an existing ProviderManager
    ///
    /// This allows creating a session with pre-configured model selection.
    ///
    /// # Arguments
    /// * `provider_manager` - Pre-configured ProviderManager
    ///
    /// # Returns
    /// * `Session` - New session with the given provider manager
    pub fn from_provider_manager(provider_manager: ProviderManager) -> Self {
        Self {
            provider_manager,
            messages: Vec::new(),
            turns: Vec::new(),
            token_tracker: TokenTracker::default(),
            annotations: std::collections::HashMap::new(),
            thinking_exhaustion_cross_turn_count: 0,
            session_thinking_level: codelet_tools::facade::ThinkingLevel::High,
        }
    }

    /// Get current provider name
    pub fn current_provider_name(&self) -> &str {
        self.provider_manager.current_provider_name()
    }

    /// Get current model ID (if explicitly selected)
    pub fn current_model_id(&self) -> Option<String> {
        self.provider_manager.selected_model_id()
    }

    /// Switch to a different provider
    ///
    /// CRITICAL: This clears the message history to start fresh with new provider
    ///
    /// # Arguments
    /// * `provider_name` - Name of provider to switch to
    ///
    /// # Returns
    /// * `Result<()>` - Success or error if provider unavailable
    pub fn switch_provider(&mut self, provider_name: &str) -> Result<()> {
        // Clear conversation context before switching (matches codelet behavior)
        self.messages.clear();
        self.turns.clear();
        self.token_tracker = TokenTracker::default();

        // Switch provider
        self.provider_manager.switch_provider(provider_name)?;

        Ok(())
    }

    /// Get provider manager reference
    pub fn provider_manager(&self) -> &ProviderManager {
        &self.provider_manager
    }

    /// Get mutable provider manager reference
    pub fn provider_manager_mut(&mut self) -> &mut ProviderManager {
        &mut self.provider_manager
    }

    /// Add system-reminder to messages array
    ///
    /// System-reminders are Messages that persist through compaction.
    /// Each type (claudeMd, environment, gitStatus, tokenStatus) has exactly one instance.
    /// Deduplication is automatic via retain+push pattern.
    ///
    /// # Arguments
    /// * `reminder_type` - Type of system reminder
    /// * `content` - Content text for the reminder
    ///
    /// # Example
    /// ```
    /// use codelet_cli::session::{Session, SystemReminderType};
    ///
    /// let mut session = Session::new(None).unwrap();
    /// session.add_system_reminder(SystemReminderType::TokenStatus, "50% tokens used");
    /// ```
    pub fn add_system_reminder(&mut self, reminder_type: SystemReminderType, content: &str) {
        // Use existing add_system_reminder function which implements deduplication
        self.messages = add_system_reminder(&self.messages, reminder_type, content);
    }

    /// Inject context reminders at session start (CLI-016)
    ///
    /// Discovers CLAUDE.md/AGENTS.md files and gathers environment information,
    /// injecting them as system reminders for the LLM.
    ///
    /// This should be called once after Session::new() to provide initial context.
    ///
    /// # Example
    /// ```
    /// use codelet_cli::session::Session;
    ///
    /// let mut session = Session::new(None).unwrap();
    /// session.inject_context_reminders();
    /// ```
    pub fn inject_context_reminders(&mut self) {
        self.inject_context_reminders_with_isolation(None);
    }

    /// GIT-034: Inject context reminders with isolation context
    ///
    /// Same as `inject_context_reminders()` but also includes isolation context
    /// for worktree sessions. When the session is isolated, the environment
    /// reminder will include Isolation, Worktree path, and Base commit fields.
    ///
    /// # Arguments
    /// * `isolation` - Optional isolation context for worktree sessions
    ///
    /// # Example
    /// ```
    /// use codelet_cli::session::Session;
    /// use codelet_cli::session::context_gathering::IsolationContext;
    ///
    /// let mut session = Session::new(None).unwrap();
    /// let isolation = IsolationContext {
    ///     is_isolated: true,
    ///     worktree_path: Some(".fspec/worktrees/abc123/".to_string()),
    ///     base_commit: Some("7a8b9c0d".to_string()),
    /// };
    /// session.inject_context_reminders_with_isolation(Some(&isolation));
    /// ```
    pub fn inject_context_reminders_with_isolation(
        &mut self,
        isolation: Option<&context_gathering::IsolationContext>,
    ) {
        use context_gathering::{discover_claude_md, gather_environment_info_with_isolation};

        // Inject CLAUDE.md/AGENTS.md content if found
        if let Some(content) = discover_claude_md(None) {
            self.add_system_reminder(SystemReminderType::ClaudeMd, &content);
        }

        // Inject environment information with optional isolation context
        let env_info = gather_environment_info_with_isolation(isolation);
        self.add_system_reminder(
            SystemReminderType::Environment,
            &env_info.to_reminder_content(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::message::{Message, UserContent};
    use rig::OneOrMany;

    #[test]
    fn test_session_creation() {
        // This will fail if no providers configured, which is expected in test env
        // In real usage, Session::new requires valid provider configuration
        let _ = Session::new(None);
    }

    #[test]
    fn test_message_persistence() {
        // @step Given I create a new Session
        let session_result = Session::new(None);
        if let Ok(mut session) = session_result {
            // @step When I access the messages vector
            // @step Then the messages vector should be empty initially
            assert_eq!(session.messages.len(), 0);

            // @step And I should be able to add messages to it
            // Add a test message using rig's Message API
            session.messages.push(Message::User {
                content: OneOrMany::one(UserContent::text("test")),
            });

            assert_eq!(session.messages.len(), 1);
        }
    }

    #[test]
    fn test_provider_switch_clears_context() {
        // @step Given I am in an interactive REPL session with Claude provider
        let session_result = Session::new(None);
        if let Ok(mut session) = session_result {
            // @step And I have had a multi-turn conversation with message history
            // Add messages using rig's Message API
            session.messages.push(Message::User {
                content: OneOrMany::one(UserContent::text("test")),
            });

            // @step When I type "/openai" to switch providers
            // Switch provider (may fail if provider doesn't exist, but that's OK for test)
            let _ = session.switch_provider("nonexistent");

            // @step Then the message history should be cleared
            // @step And the session should start fresh with the new provider
            // @step And previous conversation context should not be accessible
            assert_eq!(session.messages.len(), 0);
        }
    }
}
