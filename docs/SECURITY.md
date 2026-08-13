# Security: Running in a Sandbox

**fspec agents have full access to your file system, network, and shell.** They can read, write, and execute anything your user account can. This is by design—agents need these capabilities to write code, run tests, and manage your project.

However, this means a compromised or misbehaving agent could:
- Read sensitive files (SSH keys, credentials, other projects)
- Make network requests to arbitrary endpoints
- Execute destructive commands

## Recommended: Use ExitBox

[ExitBox](https://github.com/cloud-exit/exitbox) runs AI agents in isolated containers with defense-in-depth security:

- **Network firewall** — Agents can only reach allowlisted domains
- **File isolation** — Only your project directory is mounted
- **Capability restrictions** — No raw sockets, no privilege escalation
- **Credential protection** — SSH keys and cloud credentials are not exposed

## What Gets Restricted

| Resource | Without Sandbox | With ExitBox |
|----------|-----------------|--------------|
| File system | Full access | Only `/workspace` (your project) |
| Network | Unrestricted | Allowlisted domains only |
| SSH keys | Accessible | Hidden (unless `--full-git-support`) |
| Other projects | Accessible | Isolated |
| System commands | Full shell | Restricted capabilities |

## When to Skip the Sandbox

If you're running fspec on throwaway VMs, CI environments, or fully trust the agent, you can run directly. The sandbox adds a small amount of overhead and complexity.

For local development on your primary machine, **the sandbox is strongly recommended**.
