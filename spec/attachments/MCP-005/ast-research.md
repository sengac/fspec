# MCP-005 AST research

## fspec astgrep --pattern 'connect_stdio($$$A)' --lang rust --path rust/tools/src
rust/tools/src/mcp/connect_tool.rs:125:25:connect_stdio(
rust/tools/src/mcp/tests_03.rs:14:18:connect_stdio(
rust/tools/src/mcp/host_config.rs:133:17:connect_stdio(
rust/tools/src/mcp/tests_01.rs:206:18:connect_stdio(
rust/tools/src/mcp/helper_tests_02.rs:40:22:connect_stdio("test", "", None, 10, tx, conns)


## fspec astgrep --pattern 'command.split_whitespace()' --lang rust --path rust/tools/src
rust/tools/src/facade/wrapper.rs:1316:38:command
rust/tools/src/mcp/host_config.rs:200:53:command
rust/tools/src/mcp/stdio_and_routing.rs:17:28:command.split_whitespace()


## fspec astgrep --pattern 'HostMcpTransport::Stdio { $$$F }' --lang rust --path rust/tools/src
No matches found

## fspec astgrep --pattern 'stdio($A, $B)' --lang rust --path rust/rpc-server/tests
No matches found

## fspec astgrep --pattern 'McpConnectArgs { $$$F }' --lang rust --path rust/tools/src
rust/tools/src/mcp/tests_02.rs:213:16:McpConnectArgs {
rust/tools/src/mcp/tests_03.rs:50:16:McpConnectArgs {
rust/tools/src/mcp/tests_01.rs:98:16:McpConnectArgs {
rust/tools/src/mcp/tests_01.rs:242:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_03.rs:7:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:102:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:124:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:146:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:170:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:194:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:218:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:242:16:McpConnectArgs {

