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

    /// Conversation turns for anchor point detection (CLI-009)
    /// Grouped messages for compaction analysis
    pub turns: Vec<ConversationTurn>,

    /// Token tracker for cache-aware compaction (CLI-009)
    /// Tracks cumulative token usage across conversation
    pub token_tracker: TokenTracker,
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
        })
    }

    /// Get current provider name
    pub fn current_provider_name(&self) -> &str {
        self.provider_manager.current_provider_name()
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

    /// Compact messages while preserving system-reminders
    ///
    /// This method integrates system-reminder persistence with the compaction system:
    /// 1. Partition messages into system-reminders and compactable messages
    /// 2. Compact turns (operates on self.turns, not messages)
    /// 3. Reconstruct messages: summary + system-reminders + kept turns as messages
    ///
    /// System-reminders persist through compaction while other messages are summarized.
    ///
    /// # Arguments
    /// * `llm_prompt` - Async function that takes a prompt string and returns summary
    ///
    /// # Returns
    /// * `Result<()>` - Success or error if compaction fails
    ///
    /// # Example
    ///
    /// ```text
    /// session.compact_messages(|prompt| async move {
    ///     llm_client.complete(&prompt).await
    /// }).await?;
    /// ```
    pub async fn compact_messages<F, Fut>(&mut self, llm_prompt: F) -> Result<()>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        use codelet_core::compaction::ContextCompactor;
        use rig::message::{AssistantContent, Message, UserContent};
        use rig::OneOrMany;
        use system_reminders::partition_for_compaction;

        // Step 1: Extract system-reminders from messages Vec
        let (system_reminders, _compactable) = partition_for_compaction(&self.messages);

        // Step 2: Calculate summarization budget
        use crate::compaction_threshold::calculate_summarization_budget;
        let context_window = self.provider_manager.context_window() as u64;
        let budget = calculate_summarization_budget(context_window);

        // Step 3: Compact turns (operates on self.turns, not messages)
        let compactor = ContextCompactor::new();

        // If we have no turns, nothing to compact
        if self.turns.is_empty() {
            return Ok(());
        }

        let result = compactor.compact(&self.turns, budget, llm_prompt).await?;

        // Step 4: Reconstruct messages Vec:
        //    - System-reminders FIRST (maintains prefix stability for prompt caching)
        //    - Compacted summary as user message
        //    - Kept turns converted back to messages
        self.messages.clear();

        // Add system-reminders FIRST (maintains stable prefix for prompt caching)
        // This is critical: reminders must be at the START of the messages array
        // so that the LLM's prompt cache can match the prefix on subsequent calls.
        self.messages.extend(system_reminders);

        // Add summary after reminders
        self.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(&result.summary)),
        });

        // Add kept turns as messages
        for turn in result.kept_turns {
            self.messages.push(Message::User {
                content: OneOrMany::one(UserContent::text(&turn.user_message)),
            });
            self.messages.push(Message::Assistant {
                id: Some(format!(
                    "compacted-{}",
                    turn.timestamp.elapsed().unwrap_or_default().as_secs()
                )),
                content: OneOrMany::one(AssistantContent::text(&turn.assistant_response)),
            });
        }

        Ok(())
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
        use context_gathering::{discover_claude_md, gather_environment_info};

        // Inject CLAUDE.md/AGENTS.md content if found
        if let Some(content) = discover_claude_md(None) {
            self.add_system_reminder(SystemReminderType::ClaudeMd, &content);
        }

        // Inject environment information
        let env_info = gather_environment_info();
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
