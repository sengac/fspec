//! Feature: spec/features/project-scaffold.feature
//!
//! Tests for Project Scaffold for Codelet Rust Port
//! Verifies that all 5 bounded context modules are properly structured.

/// Scenario: Project compiles with all bounded context modules
#[test]
fn test_project_compiles_with_all_bounded_context_modules() {
    // @step Given the project has 5 bounded context modules: cli, providers, tools, context, and agent
    // Verify modules exist by importing them
    use codelet::agent::Runner;
    use codelet::cli::Cli;
    use codelet::context::TokenTracker;
    use codelet::providers::ProviderType;
    use codelet::tools::ToolRegistry;

    // Verify types are accessible
    let _ = ProviderType::Anthropic;
    let _ = ToolRegistry::new();
    let _ = TokenTracker {
        input_tokens: 0,
        output_tokens: 0,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
    };
    let _ = Runner::new();
    let _ = std::any::type_name::<Cli>();

    // @step And each module has a mod.rs file that re-exports public types
    // This is verified by the imports above compiling

    // @step When I run 'cargo build'
    // This test running means cargo build succeeded

    // @step Then the build should complete successfully
    // If we reach here, the build completed successfully
    assert!(
        true,
        "Project compiled successfully with all bounded context modules"
    );
}

/// Scenario: Tool Execution module exports Tool trait and ToolRegistry
#[test]
fn test_tool_execution_module_exports() {
    // @step Given the tools module exists at src/tools/mod.rs
    // @step When I import from the tools module
    use codelet::tools::{Tool, ToolOutput, ToolParameters, ToolRegistry};

    // @step Then the Tool trait should be available with name, description, parameters, and execute methods
    // Verify Tool trait has required methods by creating a test implementation
    struct TestTool {
        params: ToolParameters,
    }

    #[async_trait::async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test"
        }
        fn description(&self) -> &str {
            "test tool"
        }
        fn parameters(&self) -> &ToolParameters {
            &self.params
        }
        async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolOutput> {
            Ok(ToolOutput::success("done".to_string()))
        }
    }

    let _tool = TestTool {
        params: ToolParameters::default(),
    };

    // @step And the ToolRegistry struct should be available
    let _registry = ToolRegistry::new();
}

/// Scenario: Provider Management module exports LlmProvider trait and ProviderType
#[test]
fn test_provider_management_module_exports() {
    // @step Given the providers module exists at src/providers/mod.rs
    // @step When I import from the providers module
    use codelet::providers::{LlmProvider, ProviderType};

    // @step Then the LlmProvider trait should be available with complete, supports_caching, and supports_streaming methods
    // Verify LlmProvider trait has required methods
    fn _check_llm_provider_trait<T: LlmProvider>(provider: &T) {
        let _ = provider.supports_caching();
        let _ = provider.supports_streaming();
        // complete is async, verified by trait bounds
    }

    // @step And the ProviderType enum should be available
    let _anthropic = ProviderType::Anthropic;
    let _openai = ProviderType::OpenAI;
    let _google = ProviderType::Google;
}

/// Scenario: Agent Execution module exports Runner and Message types
#[test]
fn test_agent_execution_module_exports() {
    // @step Given the agent module exists at src/agent/mod.rs
    // @step When I import from the agent module
    use codelet::agent::{Message, MessageContent, MessageRole, Runner};

    // @step Then the Runner struct should be available
    let _runner = Runner::new();

    // @step And the Message types with role and content supporting text and tool interactions should be available
    let _msg = Message {
        role: MessageRole::User,
        content: MessageContent::Text("hello".to_string()),
    };
}

/// Scenario: Context Management module exports TokenTracker with effective_tokens
#[test]
fn test_context_management_module_exports() {
    // @step Given the context module exists at src/context/mod.rs
    // @step When I import from the context module
    use codelet::context::TokenTracker;

    // @step Then the TokenTracker struct should be available with input_tokens, output_tokens, cache_read_input_tokens, and cache_creation_input_tokens fields
    let tracker = TokenTracker {
        input_tokens: 0,
        output_tokens: 0,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
    };
    let _ = tracker.input_tokens;
    let _ = tracker.output_tokens;
    let _ = tracker.cache_read_input_tokens;
    let _ = tracker.cache_creation_input_tokens;

    // @step And the TokenTracker should have an effective_tokens method
    let _ = tracker.effective_tokens();
}

/// Scenario: CLI Interface module exports Cli struct with clap derive
#[test]
fn test_cli_interface_module_exports() {
    // @step Given the cli module exists at src/cli/mod.rs
    // @step When I import from the cli module
    use codelet::cli::Cli;

    // @step Then the Cli struct should be available with clap Parser derive
    use clap::Parser;
    // Verify Cli implements Parser by attempting to parse empty args
    // This will fail with missing args, but proves the derive works
    let result = Cli::try_parse_from::<[&str; 0], &str>([]);
    // We expect this to either succeed or fail with clap error, not a compile error
    let _ = result;
}
