# COV-066 AST research

## fspec astgrep --pattern 'connect_stdio($$$A)' --lang rust --path rust/tools/src/mcp
rust/tools/src/mcp/connect_tool.rs:129:25:connect_stdio(
rust/tools/src/mcp/tests_03.rs:14:18:connect_stdio(
rust/tools/src/mcp/host_config.rs:140:17:connect_stdio(
rust/tools/src/mcp/tests_01.rs:207:18:connect_stdio(
rust/tools/src/mcp/helper_tests_02.rs:40:22:connect_stdio("test", &crate::mcp::stdio_command::McpStdioCommand::from_command_line(""), None, 10, tx, conns)


## fspec astgrep --pattern 'McpConnectArgs { $$$F }' --lang rust --path rust/tools/src/mcp
rust/tools/src/mcp/tests_02.rs:213:16:McpConnectArgs {
rust/tools/src/mcp/tests_03.rs:50:16:McpConnectArgs {
rust/tools/src/mcp/tests_01.rs:98:16:McpConnectArgs {
rust/tools/src/mcp/tests_01.rs:243:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_03.rs:7:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:102:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:125:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:148:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:173:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:198:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:223:16:McpConnectArgs {
rust/tools/src/mcp/helper_tests_02.rs:248:16:McpConnectArgs {


## grep '@step' counts
- rust/tools/src/mcp/connect_tool.rs: 0
- rust/tools/src/mcp/tests_01.rs: 27
- rust/tools/src/mcp/tests_02.rs: 34
- rust/tools/src/mcp/helper_tests_02.rs: 0
- rust/tools/src/mcp/helper_tests_03.rs: 22
