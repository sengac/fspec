# REFAC-032 AST research

## fspec astgrep --pattern 'pub fn $NAME($$$A) $$$R' --lang rust --path rust/tools/src/mcp
rust/tools/src/mcp/stdio_and_routing.rs:272:1:pub fn parse_mcp_tool_name(qualified_name: &str) -> Option<(&str, &str)> {
rust/tools/src/mcp/tool_wrappers.rs:15:1:pub fn set_mcp_tool_server_handle(session_id: uuid::Uuid, handle: ToolServerHandle) {
rust/tools/src/mcp/tool_wrappers.rs:169:1:pub fn gather_mcp_tool_wrappers(session_id: uuid::Uuid) -> Vec<McpToolWrapper> {
rust/tools/src/mcp/tool_wrappers.rs:208:5:pub fn new(session_id: uuid::Uuid) -> Self {
rust/tools/src/mcp/types.rs:139:1:pub fn new_mcp_connection_map() -> McpConnectionMap {
rust/tools/src/mcp/types.rs:174:5:pub fn new(name: String, injection_tx: McpInjectionTx, connections: McpConnectionMap) -> Self {
rust/tools/src/mcp/http_and_session_registry.rs:2:1:pub fn qualified_mcp_tool_name(server: &str, tool: &str) -> String {
rust/tools/src/mcp/http_and_session_registry.rs:7:1:pub fn gather_mcp_tools(connections: &HashMap<String, McpConnection>) -> Vec<McpToolDef> {
rust/tools/src/mcp/http_and_session_registry.rs:35:1:pub fn list_connections(connections: &HashMap<String, McpConnection>) -> Vec<McpConnectionSummary> {
rust/tools/src/mcp/http_and_session_registry.rs:227:1:pub fn init_mcp_session(
rust/tools/src/mcp/http_and_session_registry.rs:251:1:pub fn cleanup_mcp_session(session_id: uuid::Uuid) {
rust/tools/src/mcp/http_and_session_registry.rs:274:1:pub fn get_mcp_connections(session_id: uuid::Uuid) -> Option<McpConnectionMap> {


## fspec astgrep --pattern 'pub struct $NAME { $$$F }' --lang rust --path rust/tools/src/mcp
rust/tools/src/mcp/tool_wrappers.rs:39:1:pub struct McpToolRegistration {
rust/tools/src/mcp/tool_wrappers.rs:89:1:pub struct McpToolWrapper {
rust/tools/src/mcp/tool_wrappers.rs:202:1:pub struct ConnectMcpTool {
rust/tools/src/mcp/types.rs:5:1:pub struct McpConnection {
rust/tools/src/mcp/types.rs:27:1:pub struct McpToolDef {
rust/tools/src/mcp/types.rs:52:1:pub struct McpServerInfo {
rust/tools/src/mcp/types.rs:86:1:pub struct McpConnectArgs {
rust/tools/src/mcp/types.rs:109:1:pub struct McpConnectResult {
rust/tools/src/mcp/types.rs:122:1:pub struct McpConnectionSummary {
rust/tools/src/mcp/types.rs:163:1:pub struct DynMcpHandler {


## fspec astgrep --pattern 'impl $T for $S { $$$B }' --lang rust --path rust/tools/src/mcp
rust/tools/src/mcp/connect_tool.rs:64:1:impl rig::tool::Tool for ConnectMcpTool {
rust/tools/src/mcp/client_handler.rs:4:1:impl rmcp::handler::client::ClientHandler for DynMcpHandler {
rust/tools/src/mcp/tool_wrappers.rs:115:1:impl rig::tool::Tool for McpToolWrapper {


## Findings

- The production items fall into six cohesive groups (types and connection map; the rmcp client
  handler; stdio connect, tool routing and disconnect; tool-name helpers, HTTP connect and the
  per-session MCP_SESSIONS registry; tool wrappers and registrations; the ConnectMCP tool). Several
  helpers are private (`get_mcp_session_state`, `get_mcp_tool_server_handle`,
  `validate_mcp_tool_definitions`, `add_wrappers_to_handle`), so the split must keep them in the
  `mcp` module scope.
- The unit tests are one `tests` module with a nested `helper_tests` module (lines 1437-3375).
