macro_rules! auxiliary_agent_builder {
    ($provider:expr, codex) => {
        rig::agent::AgentBuilder::new($provider.completion_model())
    };
    ($provider:expr) => {
        $provider.client().agent($provider.model())
    };
}

pub(crate) use auxiliary_agent_builder;
