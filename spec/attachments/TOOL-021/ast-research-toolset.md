{
  "research_type": "ast",
  "date": "2026-07-17",
  "work_unit": "TOOL-021",
  "summary": "AST analysis of ToolSet and ToolServer in rig-core/src/tool/",
  "entities": [
    {
      "name": "ToolSet",
      "type": "struct",
      "file": "mod.rs:417-419",
      "details": "HashMap<String, ToolType> - case-sensitive keys"
    },
    {
      "name": "ToolSet::contains",
      "type": "method",
      "file": "mod.rs:437-439",
      "details": "Direct HashMap::contains_key - case sensitive"
    },
    {
      "name": "ToolSet::add_tool",
      "type": "method",
      "file": "mod.rs:442-445",
      "details": "Inserts using tool.name() directly - no normalization"
    },
    {
      "name": "ToolSet::add_tool_boxed",
      "type": "method",
      "file": "mod.rs:448-450",
      "details": "Inserts using tool.name() directly - no normalization"
    },
    {
      "name": "ToolSet::delete_tool",
      "type": "method",
      "file": "mod.rs:452-454",
      "details": "HashMap::remove with raw tool_name - case sensitive"
    },
    {
      "name": "ToolSet::get",
      "type": "method",
      "file": "mod.rs:461-463",
      "details": "Direct HashMap::get - case sensitive"
    },
    {
      "name": "ToolSet::call",
      "type": "method",
      "file": "mod.rs:475-485",
      "details": "Direct HashMap::get - case sensitive, returns ToolNotFoundError with original name"
    },
    {
      "name": "ToolSetBuilder::build",
      "type": "method",
      "file": "mod.rs:560-568",
      "details": "Uses tool.name() as key without normalization"
    },
    {
      "name": "ToolServer::handle_message",
      "type": "method",
      "file": "server.rs:108-154",
      "details": "CallTool and RemoveTool use raw name/tool_name strings"
    }
  ],
  "conclusion": "All tool name lookups use case-sensitive HashMap operations. Need to normalize to lowercase at registration and lookup points."
}
