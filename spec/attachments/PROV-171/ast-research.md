# AST research: PROV-171

## `std::env::var($V).ok().and_then($F).unwrap_or($D)` in rust/providers/src/openai/construction.rs
```
rust/providers/src/openai/construction.rs:100:30:std::env::var("OPENAI_CONTEXT_WINDOW")
rust/providers/src/openai/construction.rs:105:33:std::env::var("OPENAI_MAX_OUTPUT_TOKENS")

```

## `std::env::var($V).ok().and_then($F).unwrap_or($D)` in rust/providers/src/manager/impl_4.rs
```
rust/providers/src/manager/impl_4.rs:155:27:std::env::var("OPENAI_CONTEXT_WINDOW")
rust/providers/src/manager/impl_4.rs:159:27:std::env::var("OPENAI_MAX_OUTPUT_TOKENS")

```

## `Some(self.context_window)` in rust/providers/src/openai/llm_provider.rs
```
rust/providers/src/openai/llm_provider.rs:34:9:Some(self.context_window)
rust/providers/src/openai/llm_provider.rs:88:9:Some(self.context_window)

```

## `Some(self.max_output_tokens)` in rust/providers/src/openai/llm_provider.rs
```
rust/providers/src/openai/llm_provider.rs:40:9:Some(self.max_output_tokens)
rust/providers/src/openai/llm_provider.rs:92:9:Some(self.max_output_tokens)

```

## `openai::$C` in rust/providers/src/manager
```
rust/providers/src/manager/impl_4.rs:158:32:openai::CONTEXT_WINDOW
rust/providers/src/manager/impl_4.rs:162:32:openai::MAX_OUTPUT_TOKENS

```

## `$B.max_tokens(self.max_output_tokens as u64)` in rust/providers/src/openai
```
rust/providers/src/openai/llm_provider.rs:123:27:CompletionRequestBuilder::new(self.completion_model.clone(), prompt)
rust/providers/src/openai/agent.rs:103:13:openai::completion::CompletionModel::new(self.rig_client.clone(), &self.model_name)

```

## `OpenAIProvider::$F($$$A)` in rust/providers/src/manager
```
rust/providers/src/manager/impl_3.rs:183:13:OpenAIProvider::from_api_key_with_session(&api_key, &model_id, session_id)

```

## `fn refresh_runtime_options(&mut self) -> $R { $$$B }` in rust/providers/src/manager_runtime.rs
```
No matches found
```

## `std::env::set_var("OPENAI_CONTEXT_WINDOW", $V)` in rust/sessions/src
```
rust/sessions/src/profile_environment.rs:28:9:std::env::set_var("OPENAI_CONTEXT_WINDOW", context_window.to_string())

```

## `apply_profile_env_vars($$$A)` in rust/sessions/src
```
No matches found
```
