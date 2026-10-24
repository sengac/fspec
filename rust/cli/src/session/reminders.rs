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

    pub fn inject_context_reminders(&mut self) {
        self.inject_context_reminders_with_isolation(None);
    }

    pub fn inject_context_reminders_with_isolation(
        &mut self,
        isolation: Option<&context_gathering::IsolationContext>,
    ) {
        use context_gathering::{discover_agents_md, gather_environment_info_with_isolation};

        if let Some(content) = discover_agents_md(None) {
            self.add_system_reminder(SystemReminderType::SystemPrompt, &content);
        }

        let env_info = gather_environment_info_with_isolation(isolation);
        self.add_system_reminder(
            SystemReminderType::Environment,
            &env_info.to_reminder_content(),
        );
    }
}
