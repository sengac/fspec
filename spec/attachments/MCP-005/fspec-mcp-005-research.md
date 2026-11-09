# Research: structured stdio command arguments

## Problem

The Paseo provider contract sends stdio server definitions as `{ type: "stdio", command, args?, env? }` (`/Users/rquast/projects/fspec-paseo/docs/paseo-provider-contract.md:794`). fspec's `McpConnectArgs` instead exposes a single `command` string (`rust/tools/src/mcp.rs:110`). `connect_stdio` splits it using `split_whitespace` (`rust/tools/src/mcp.rs:353`) before passing the pieces to `tokio::process::Command`. This loses argument boundaries for spaces, empty strings and quoting; executable paths with spaces are also unsafe to reconstruct. The issue is argument fidelity, not a request to execute through a shell.

Snapshots: fspec `344d00b4`, fspec-paseo `f64171f`; recheck before implementation. The separate MCP-003 card will expose provider-driven session setup.

## Expected behavior

- Accept executable and an explicit ordered argument vector as structured values and pass each value literally to the spawned process. Preserve spaces, empty args, quotes, Unicode and platform-specific paths; do not shell-expand or interpret metacharacters.
- Preserve the existing `ConnectMCP` single-string behavior for callers that do not opt into a structured form. Specify unambiguous validation and precedence if both representations are supplied. Avoid silently changing existing clients.
- Forward environment overrides to the child and keep credentials out of diagnostic output. Maintain session-specific lifecycle, timeout, cleanup and tool discovery.
- Use a real fixture executable or MCP test server to record received argv and env safely, exercising multiple args including one with spaces, an empty arg, quote characters and a Unicode argument. Test missing executable, malformed input, spawn/handshake failure, teardown and compatibility with a legacy command.

## Design considerations

Avoid joining argv into one string and then reparsing it; add a typed internal command representation used both by the agent tool and daemon API. Consider cross-platform process launching and working-directory semantics explicitly. Confirm whether current callers rely on whitespace splitting; a new optional argument field may need versioning and a clear compatibility policy. This card does not change fspec-paseo: that provider must separately forward its input to the new RPC.
