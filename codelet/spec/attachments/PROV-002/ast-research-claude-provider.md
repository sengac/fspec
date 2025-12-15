{
  "matches": [
    {
      "type": "function_item",
      "name": "new",
      "line": 44,
      "column": 4,
      "text": "pub fn new() -> Result<Self> {\n        let api_key = std::env::var(\"ANTHROPIC_API_KEY\")\n            .or_else(|_| std::env::var(\"CLAUDE_CODE_OAUTH_TOKEN\"))\n            .map_err(|_| {\n                anyhow!(\n                    \"No API key found. Set ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN environment variable\"\n                )\n            })?;\n\n        Self::from_api_key(&api_key)\n    }"
    },
    {
      "type": "function_item",
      "name": "from_api_key",
      "line": 57,
      "column": 4,
      "text": "pub fn from_api_key(api_key: &str) -> Result<Self> {\n        if api_key.is_empty() {\n            return Err(anyhow!(\"API key cannot be empty\"));\n        }\n\n        Ok(Self {\n            api_key: api_key.to_string(),\n            model: DEFAULT_MODEL.to_string(),\n            client: Client::new(),\n        })\n    }"
    },
    {
      "type": "function_item",
      "name": "format_messages",
      "line": 70,
      "column": 4,
      "text": "fn format_messages(&self, messages: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {\n        let mut system_content: Option<String> = None;\n        let mut api_messages: Vec<AnthropicMessage> = Vec::new();\n\n        for msg in messages {\n            let content = match &msg.content {\n                MessageContent::Text(text) => text.clone(),\n                MessageContent::Parts(parts) => {\n                    // Concatenate text parts for now\n                    parts\n                        .iter()\n                        .filter_map(|p| {\n                            if let crate::agent::ContentPart::Text { text } = p {\n                                Some(text.as_str())\n                            } else {\n                                None\n                            }\n                        })\n                        .collect::<Vec<_>>()\n                        .join(\"\\n\")\n                }\n            };\n\n            match msg.role {\n                MessageRole::System => {\n                    // Anthropic requires system as a separate field\n                    system_content = Some(content);\n                }\n                MessageRole::User => {\n                    api_messages.push(AnthropicMessage {\n                        role: \"user\".to_string(),\n                        content,\n                    });\n                }\n                MessageRole::Assistant => {\n                    api_messages.push(AnthropicMessage {\n                        role: \"assistant\".to_string(),\n                        content,\n                    });\n                }\n            }\n        }\n\n        (system_content, api_messages)\n    }"
    },
    {
      "type": "function_item",
      "name": "format_messages_with_tools",
      "line": 117,
      "column": 4,
      "text": "fn format_messages_with_tools(\n        &self,\n        messages: &[Message],\n    ) -> (Option<String>, Vec<AnthropicMessageWithContent>) {\n        let mut system_content: Option<String> = None;\n        let mut api_messages: Vec<AnthropicMessageWithContent> = Vec::new();\n\n        for msg in messages {\n            match msg.role {\n                MessageRole::System => {\n                    // Extract system content\n                    if let MessageContent::Text(text) = &msg.content {\n                        system_content = Some(text.clone());\n                    }\n                }\n                MessageRole::User => {\n                    let content = self.message_content_to_api(&msg.content);\n                    api_messages.push(AnthropicMessageWithContent {\n                        role: \"user\".to_string(),\n                        content,\n                    });\n                }\n                MessageRole::Assistant => {\n                    let content = self.message_content_to_api(&msg.content);\n                    api_messages.push(AnthropicMessageWithContent {\n                        role: \"assistant\".to_string(),\n                        content,\n                    });\n                }\n            }\n        }\n\n        (system_content, api_messages)\n    }"
    },
    {
      "type": "function_item",
      "name": "message_content_to_api",
      "line": 153,
      "column": 4,
      "text": "fn message_content_to_api(&self, content: &MessageContent) -> Vec<ApiContentBlock> {\n        match content {\n            MessageContent::Text(text) => {\n                vec![ApiContentBlock::Text { text: text.clone() }]\n            }\n            MessageContent::Parts(parts) => parts\n                .iter()\n                .map(|part| match part {\n                    ContentPart::Text { text } => ApiContentBlock::Text { text: text.clone() },\n                    ContentPart::ToolUse { id, name, input } => ApiContentBlock::ToolUse {\n                        id: id.clone(),\n                        name: name.clone(),\n                        input: input.clone(),\n                    },\n                    ContentPart::ToolResult {\n                        tool_use_id,\n                        content,\n                        is_error,\n                    } => ApiContentBlock::ToolResult {\n                        tool_use_id: tool_use_id.clone(),\n                        content: content.clone(),\n                        is_error: Some(*is_error),\n                    },\n                })\n                .collect(),\n        }\n    }"
    },
    {
      "type": "function_item",
      "name": "name",
      "line": 184,
      "column": 4,
      "text": "fn name(&self) -> &str {\n        \"claude\"\n    }"
    },
    {
      "type": "function_item",
      "name": "model",
      "line": 188,
      "column": 4,
      "text": "fn model(&self) -> &str {\n        &self.model\n    }"
    },
    {
      "type": "function_item",
      "name": "context_window",
      "line": 192,
      "column": 4,
      "text": "fn context_window(&self) -> usize {\n        CONTEXT_WINDOW\n    }"
    },
    {
      "type": "function_item",
      "name": "max_output_tokens",
      "line": 196,
      "column": 4,
      "text": "fn max_output_tokens(&self) -> usize {\n        MAX_OUTPUT_TOKENS\n    }"
    },
    {
      "type": "function_item",
      "name": "supports_caching",
      "line": 200,
      "column": 4,
      "text": "fn supports_caching(&self) -> bool {\n        true // Claude supports prompt caching\n    }"
    },
    {
      "type": "function_item",
      "name": "supports_streaming",
      "line": 204,
      "column": 4,
      "text": "fn supports_streaming(&self) -> bool {\n        false // Streaming will be implemented in PROV-002\n    }"
    },
    {
      "type": "function_item",
      "name": "complete",
      "line": 208,
      "column": 4,
      "text": "async fn complete(&self, messages: &[Message]) -> Result<String> {\n        let (system, api_messages) = self.format_messages(messages);\n\n        let request = AnthropicRequest {\n            model: self.model.clone(),\n            max_tokens: MAX_OUTPUT_TOKENS,\n            system,\n            messages: api_messages,\n        };\n\n        let response = self\n            .client\n            .post(API_URL)\n            .header(\"x-api-key\", &self.api_key)\n            .header(\"anthropic-version\", API_VERSION)\n            .header(\"content-type\", \"application/json\")\n            .json(&request)\n            .send()\n            .await\n            .map_err(|e| anyhow!(\"Failed to send request: {}\", e))?;\n\n        let status = response.status();\n        let body = response\n            .text()\n            .await\n            .map_err(|e| anyhow!(\"Failed to read response body: {}\", e))?;\n\n        if !status.is_success() {\n            // Try to parse error response\n            if let Ok(error) = serde_json::from_str::<AnthropicError>(&body) {\n                return Err(anyhow!(\n                    \"API error ({}): {} - {}\",\n                    status.as_u16(),\n                    error.error.error_type,\n                    error.error.message\n                ));\n            }\n            return Err(anyhow!(\"API error ({}): {}\", status.as_u16(), body));\n        }\n\n        let response: AnthropicResponse = serde_json::from_str(&body)\n            .map_err(|e| anyhow!(\"Failed to parse response: {} - body: {}\", e, body))?;\n\n        // Extract text from content blocks\n        let text = response\n            .content\n            .iter()\n            .filter_map(|block| {\n                if block.block_type == \"text\" {\n                    block.text.as_ref()\n                } else {\n                    None\n                }\n            })\n            .cloned()\n            .collect::<Vec<_>>()\n            .join(\"\");\n\n        Ok(text)\n    }"
    },
    {
      "type": "function_item",
      "name": "complete_with_tools",
      "line": 269,
      "column": 4,
      "text": "async fn complete_with_tools(\n        &self,\n        messages: &[Message],\n        tools: &[ToolDefinition],\n    ) -> Result<CompletionResponse> {\n        let (system, api_messages) = self.format_messages_with_tools(messages);\n\n        // Convert tool definitions to API format\n        let api_tools: Vec<ApiToolDefinition> = tools\n            .iter()\n            .map(|t| ApiToolDefinition {\n                name: t.name.clone(),\n                description: t.description.clone(),\n                input_schema: t.input_schema.clone(),\n            })\n            .collect();\n\n        let request = AnthropicRequestWithTools {\n            model: self.model.clone(),\n            max_tokens: MAX_OUTPUT_TOKENS,\n            system,\n            messages: api_messages,\n            tools: if api_tools.is_empty() {\n                None\n            } else {\n                Some(api_tools)\n            },\n        };\n\n        let response = self\n            .client\n            .post(API_URL)\n            .header(\"x-api-key\", &self.api_key)\n            .header(\"anthropic-version\", API_VERSION)\n            .header(\"content-type\", \"application/json\")\n            .json(&request)\n            .send()\n            .await\n            .map_err(|e| anyhow!(\"Failed to send request: {}\", e))?;\n\n        let status = response.status();\n        let body = response\n            .text()\n            .await\n            .map_err(|e| anyhow!(\"Failed to read response body: {}\", e))?;\n\n        if !status.is_success() {\n            if let Ok(error) = serde_json::from_str::<AnthropicError>(&body) {\n                return Err(anyhow!(\n                    \"API error ({}): {} - {}\",\n                    status.as_u16(),\n                    error.error.error_type,\n                    error.error.message\n                ));\n            }\n            return Err(anyhow!(\"API error ({}): {}\", status.as_u16(), body));\n        }\n\n        let response: AnthropicResponseWithTools = serde_json::from_str(&body)\n            .map_err(|e| anyhow!(\"Failed to parse response: {} - body: {}\", e, body))?;\n\n        // Convert API response to internal format\n        let content_parts: Vec<ContentPart> = response\n            .content\n            .iter()\n            .map(|block| match block {\n                ResponseContentBlock::Text { text } => ContentPart::Text { text: text.clone() },\n                ResponseContentBlock::ToolUse { id, name, input } => ContentPart::ToolUse {\n                    id: id.clone(),\n                    name: name.clone(),\n                    input: input.clone(),\n                },\n            })\n            .collect();\n\n        let stop_reason = match response.stop_reason.as_str() {\n            \"tool_use\" => StopReason::ToolUse,\n            \"max_tokens\" => StopReason::MaxTokens,\n            _ => StopReason::EndTurn,\n        };\n\n        Ok(CompletionResponse {\n            content: MessageContent::Parts(content_parts),\n            stop_reason,\n        })\n    }"
    }
  ]
}
