# AST Research: Summary Generation Fix

## Target File: cli/src/interactive_helpers.rs

### Key Functions Identified:

1. **execute_compaction** (line 157)
   - Creates LLM prompt function that uses the provider
   - Uses full agent with tools for summary generation (BUG)

2. **create_conversation_turn_from_last_interaction** (line 13)
   - Helper for extracting turns from messages

### Bug Location:
Lines 163-194 in execute_compaction:
```rust
let llm_prompt = |prompt: String| async move {
    let manager = codelet_providers::ProviderManager::with_provider(provider_name)?;
    match provider_name {
        "claude" => {
            let provider = manager.get_claude()?;
            let rig_agent = provider.create_rig_agent();
            let agent = codelet_core::RigAgent::with_default_depth(rig_agent);
            agent.prompt(&prompt).await  // BUG: Full agent with tools!
        }
        // ... other providers
    }
};
```

### Fix Required:
Replace with deterministic WeightedSummaryProvider that formats turns into structured text WITHOUT calling the LLM. Match TypeScript anchor-point-compaction.ts pattern.
