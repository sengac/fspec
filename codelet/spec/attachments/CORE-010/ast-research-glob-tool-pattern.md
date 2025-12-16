{
  "matches": [
    {
      "type": "function_item",
      "name": "new",
      "line": 24,
      "column": 4,
      "text": "pub fn new() -> Self {\n        Self\n    }"
    },
    {
      "type": "function_item",
      "name": "get_mtime",
      "line": 29,
      "column": 4,
      "text": "fn get_mtime(path: &PathBuf) -> SystemTime {\n        fs::metadata(path)\n            .and_then(|m| m.modified())\n            .unwrap_or(SystemTime::UNIX_EPOCH)\n    }"
    },
    {
      "type": "function_item",
      "name": "default",
      "line": 37,
      "column": 4,
      "text": "fn default() -> Self {\n        Self::new()\n    }"
    },
    {
      "type": "function_item",
      "name": "definition",
      "line": 70,
      "column": 4,
      "text": "async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {\n        rig::completion::ToolDefinition {\n            name: \"glob\".to_string(),\n            description: \"Fast file pattern matching tool that works with any codebase size. \\\n                Supports glob patterns like \\\"**/*.js\\\" or \\\"src/**/*.ts\\\". \\\n                Returns matching file paths one per line.\\n\\n\\\n                Usage:\\n\\\n                - Pattern supports glob syntax: *, **, ?, {a,b}, [abc]\\n\\\n                - Respects .gitignore by default\\n\\\n                - Returns \\\"No matches found\\\" for patterns with no matching files\\n\\\n                - Output is truncated at 30000 characters with truncation warning\"\n                .to_string(),\n            parameters: serde_json::to_value(schemars::schema_for!(GlobArgs))\n                .unwrap_or_else(|_| json!({\"type\": \"object\"})),\n        }\n    }"
    },
    {
      "type": "function_item",
      "name": "call",
      "line": 87,
      "column": 4,
      "text": "async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {\n        // Build glob matcher\n        let matcher = GlobBuilder::new(&args.pattern)\n            .literal_separator(false)\n            .build()\n            .map(|g| g.compile_matcher())\n            .map_err(|e| GlobError::PatternError(e.to_string()))?;\n\n        // Get search path\n        let search_path = args.path.as_deref().unwrap_or(\".\");\n        let path = PathBuf::from(search_path);\n\n        if !path.exists() {\n            return Err(GlobError::PathError(format!(\n                \"Path does not exist: {search_path}\"\n            )));\n        }\n\n        // Collect matching files\n        let mut matches: Vec<PathBuf> = Vec::new();\n        for entry in WalkBuilder::new(&path)\n            .hidden(false)\n            .git_ignore(true)\n            .build()\n            .flatten()\n        {\n            if entry.file_type().is_some_and(|ft| ft.is_file()) && matcher.is_match(entry.path()) {\n                matches.push(entry.path().to_path_buf());\n            }\n        }\n\n        // Sort by modification time (newest first)\n        matches.sort_by(|a, b| {\n            let a_time = GlobTool::get_mtime(a);\n            let b_time = GlobTool::get_mtime(b);\n            b_time.cmp(&a_time)\n        });\n\n        // Format output\n        let lines: Vec<String> = matches.iter().map(|p| p.display().to_string()).collect();\n\n        // Process and truncate output\n        let output_lines = process_output_lines(&lines.join(\"\\n\"));\n        let truncate_result = truncate_output(&output_lines, OutputLimits::MAX_OUTPUT_CHARS);\n\n        let mut final_output = truncate_result.output;\n        let was_truncated = truncate_result.char_truncated || truncate_result.remaining_count > 0;\n\n        if was_truncated {\n            let warning = format_truncation_warning(\n                truncate_result.remaining_count,\n                \"files\",\n                truncate_result.char_truncated,\n                OutputLimits::MAX_OUTPUT_CHARS,\n            );\n            final_output.push_str(&warning);\n        }\n\n        Ok(final_output)\n    }"
    }
  ]
}
