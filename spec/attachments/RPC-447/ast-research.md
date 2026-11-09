# AST research: RPC-447
```
fspec-tui/src/transport/websocket/construction.rs:61:5:pub async fn connect(url: url::Url) -> Result<Self> {

rpc-server/src/server.rs:143:    let ws = tokio_tungstenite::accept_hdr_async(
fspec-tui/src/transport/websocket/reconnect.rs:19:async fn run_supervisor(
fspec/src/status.rs:73:fn resolve_url(explicit: Option<String>) -> Result<Url> {
fspec/src/client.rs:150:fn resolve_connect_url(explicit: Option<String>) -> Result<Url> {
```
