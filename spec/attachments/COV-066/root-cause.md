# COV-066 root cause

## Symptom
During MCP-005, re-pointing coverage mappings whose files shifted by a few lines failed for 18 (feature, scenario, test file) pairs:
link-coverage reported STEP VALIDATION FAILED (the test range has no @step comments for the scenario's steps), and
--skip-step-validation is refused for story/bug work units. The mappings were restored to their pre-MCP-005 line ranges.

## Affected mappings

- dynamic-mcp-connect: "Connect to MCP server via HTTP transport" -> rust/tools/src/mcp/connect_tool.rs
- dynamic-mcp-connect: "Connect to MCP server via stdio transport" -> rust/tools/src/mcp/connect_tool.rs
- dynamic-mcp-connect: "ConnectMCP with duplicate server name replaces existing connection" -> rust/tools/src/mcp/tests_02.rs
- dynamic-mcp-connect: "Disconnect an active MCP connection" -> rust/tools/src/mcp/tests_01.rs
- dynamic-mcp-connect: "Handle HTTP authentication failure" -> rust/tools/src/mcp/tests_01.rs
- dynamic-mcp-connect: "Handle connection timeout during initialization" -> rust/tools/src/mcp/tests_01.rs
- dynamic-mcp-connect: "Handle server sampling/createMessage request" -> rust/tools/src/mcp/tests_01.rs
- dynamic-mcp-connect: "Handle spawn failure with structured error" -> rust/tools/src/mcp/connect_tool.rs
- dynamic-mcp-connect: "Multi-server workflow with independent connections" -> rust/tools/src/mcp/connect_tool.rs
- dynamic-mcp-connect: "Receive server tool list changed notification" -> rust/tools/src/mcp/connect_tool.rs
- dynamic-mcp-connect: "Route tool call through cached MCP connection" -> rust/tools/src/mcp/connect_tool.rs
- dynamic-mcp-connect: "Session cleanup kills all MCP connections" -> rust/tools/src/mcp/tests_01.rs
- dynamic-mcp-connect: "Tool list assembly includes MCP tools alongside built-in tools" -> rust/tools/src/mcp/tests_02.rs
- dynamic-mcp-connect: "Tool name collision prevented by double-underscore namespacing" -> rust/tools/src/mcp/tests_02.rs
- mcp-same-turn-tool-availability: "Graceful degradation when ToolServerHandle is not set" -> rust/tools/src/mcp/helper_tests_02.rs
- mcp-same-turn-tool-availability: "Previous-turn tools coexist with same-turn tools" -> rust/tools/src/mcp/helper_tests_03.rs
- mcp-same-turn-tool-availability: "Same-turn tool invocation after connect" -> rust/tools/src/mcp/helper_tests_02.rs
- mcp-same-turn-tool-availability: "ToolServerHandle is stored after agent build" -> rust/tools/src/mcp/helper_tests_02.rs

## Cause
The mappings were created before step validation was enforced (or with validation skipped). Five scenarios point at
rust/tools/src/mcp/connect_tool.rs, which is production code, not a test. The others point at ranges of tests_01.rs,
tests_02.rs, helper_tests_02.rs (no @step comments at all) and helper_tests_03.rs whose @step comments belong to other scenarios.

## Fix
For each scenario: find (or write) the test that exercises it, add one @step comment per Gherkin step with the exact step
text, then fspec unlink-coverage the bad mapping and fspec link-coverage the real test range with its impl mappings.
Keep every touched file under 300 lines (split under a REFAC card first if needed).
