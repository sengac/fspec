use super::{context_gathering, system_reminders, Session, SessionGoal, SystemReminderType};

impl Session {
    pub fn set_goal(&mut self, text: &str, verify: Option<&str>) {
        self.goal = Some(SessionGoal {
            text: text.to_string(),
            verify: verify.map(str::to_string),
            set_at: std::time::SystemTime::now(),
        });
        self.done_rejections = 0;
        self.continue_nudges_used = 0;
        self.refresh_goal_reminder();
    }

    pub fn clear_goal(&mut self) {
        self.goal = None;
        self.done_rejections = 0;
        self.messages = system_reminders::remove_system_reminders_of_type(
            &self.messages,
            SystemReminderType::CompletionContract,
        );
    }

    pub fn refresh_goal_reminder(&mut self) {
        if let Some(goal) = &self.goal {
            let verify_line = match &goal.verify {
                Some(command) => format!("\nVerify command: {command}"),
                None => String::new(),
            };
            let content = format!(
                "Active goal: {}{}\ndone() requires evidence and a goal_assessment \
                 demonstrating this goal is met.",
                goal.text, verify_line
            );
            self.add_system_reminder(SystemReminderType::CompletionContract, &content);
        }
    }

    /// BUG-581: `working_dir` is the session's working directory (its
    /// worktree when isolated, else its project), named in the
    /// environment reminder.
    pub fn inject_context_reminders(&mut self, working_dir: &std::path::Path) {
        self.inject_context_reminders_with_isolation(working_dir, None);
    }

    pub fn inject_context_reminders_with_isolation(
        &mut self,
        working_dir: &std::path::Path,
        isolation: Option<&context_gathering::IsolationContext>,
    ) {
        use context_gathering::{discover_agents_md, gather_environment_info_with_isolation};

        if let Some(content) = discover_agents_md(working_dir) {
            self.add_system_reminder(SystemReminderType::SystemPrompt, &content);
        }

        let env_info = gather_environment_info_with_isolation(working_dir, isolation);
        self.add_system_reminder(
            SystemReminderType::Environment,
            &env_info.to_reminder_content(),
        );
    }
}
