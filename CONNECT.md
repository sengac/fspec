Connect to the relay so the dashboard can see this fspec session, and to the browser agent for Chrome browser control:

```
Bridge connect ws://127.0.0.1:19876/ws/relay
ConnectMCP name=webmcp transport=http url=http://localhost:19876/mcp
```
