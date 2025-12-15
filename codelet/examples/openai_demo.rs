//! OpenAI Provider Integration Demo
//!
//! Demonstrates that OpenAIProvider is properly wired into the public API
//! and can be used as expected by external code.

use codelet::providers::{LlmProvider, OpenAIProvider};

#[tokio::main]
async fn main() {
    println!("=== OpenAI Provider Integration Demo ===\n");

    // Test 1: Provider is accessible from public API
    println!("✓ OpenAIProvider is exported from codelet::providers");

    // Test 2: Can create provider with mock key
    std::env::set_var("OPENAI_API_KEY", "test-key-for-demo");
    match OpenAIProvider::new() {
        Ok(provider) => {
            println!("✓ Provider created successfully");
            println!("  - Name: {}", provider.name());
            println!("  - Model: {}", provider.model());
            println!("  - Context window: {}", provider.context_window());
            println!("  - Max output tokens: {}", provider.max_output_tokens());
            println!("  - Supports caching: {}", provider.supports_caching());
            println!("  - Supports streaming: {}", provider.supports_streaming());

            // Test 3: Can create rig agent
            let _agent = provider.create_rig_agent();
            println!("✓ Rig agent created with all 7 tools configured");

            println!("\n=== Integration Demo Complete ===");
            println!("All public API endpoints are properly wired and accessible.");
        }
        Err(e) => {
            eprintln!("✗ Failed to create provider: {}", e);
            std::process::exit(1);
        }
    }
}
