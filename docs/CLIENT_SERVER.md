# Client/Server Architecture: One Daemon, Many Clients

fspec is a client/server tool. The heavy work — agent sessions, LLM streaming, tool
execution, checkpoints — happens in a **daemon** process. The TUIs are just
**clients** that attach to it over WebSocket, using the internal fspec RPC
protocol. Any number of clients can attach to the same daemon at the same time,
all sharing the same live sessions.

> **Two different WebSocket protocols.** The daemon speaks the *internal fspec RPC
> protocol* (bincode-framed envelopes over `ws://`). The [Telegram/relay
> bridge](#the-telegram-and-relay-bridge) is a separate Node.js sidecar that
> speaks its **own custom bridge protocol** (JSON `{type, session_id, data}`
> messages) between a session and Telegram/relay clients. It is NOT a daemon
> client and does not speak the RPC protocol — see the
> [bridge section](#the-telegram-and-relay-bridge) below.

```
┌─────────────────────────────────────────────────────────────┐
│  Machine A (where the code lives)                           │
│                                                             │
│  fspec daemon ──── binds 127.0.0.1:3777 (loopback only)     │
│       │                                                      │
│       │  tailscale serve --tcp 3777 3777  (tailnet only)    │
│       ▼                                                      │
│   ws://100.x.x.x:3777     (internal fspec RPC protocol)     │
└───────────────────────────┬─────────────────────────────────┘
                            │  WireGuard-encrypted tailnet
                ┌───────────┴────────────┐
                │                        │
       ┌────────▼────────┐      ┌────────▼────────┐
       │  Machine B      │      │  Machine C       │
       │  fspec client   │      │  fspec client    │
       │  (TUI)          │      │  (TUI)           │
       └─────────────────┘      └─────────────────┘

  (Separately, on Machine A: each session can also connect to the
   Node.js Telegram/relay bridge via its own custom bridge protocol —
   that is a sidecar to the session, not a client of the daemon.)
```

## The three modes

The single `fspec` binary runs in one of several modes (see `fspec --help`):

| Mode | Command | What it is |
|------|---------|------------|
| **Combined** (default) | `fspec` | TUI + always-on WS server in one process. Everything local — no network involved. |
| **Daemon** | `fspec daemon --bind 127.0.0.1:3777` | Headless WS server only. No TUI. Hosts all sessions. Suited to `tmux`/`screen`/systemd. |
| **Client** | `fspec client --connect ws://…` | Frontend-only. Attaches to a running daemon; can run on ANY machine. |
| **Status** | `fspec status --connect ws://…` | One-shot health probe against the daemon. |

The daemon binds a **loopback address only** (`127.0.0.1`, `::1`, or `localhost` —
non-loopback binds are rejected at startup). That is deliberate: the daemon has no
auth layer, so it must never listen on a public interface. Reaching it from another
machine happens through a transport that provides the trust boundary — a Tailscale
tailnet (below), an SSH tunnel, or anything else that terminates on loopback.

### Daemon lifecycle

- Prints the bound port to **stdout** on startup (useful with `--bind 127.0.0.1:0`
  for ephemeral ports; pin a port when you want stable remote access).
- Writes `daemon.json` to `$XDG_RUNTIME_DIR/fspec/daemon.json` (or
  `~/.fspec/daemon.json`) with `port`, `pid`, `workspace`, `started_at`, `version`.
  `fspec client` without `--connect` auto-discovers the daemon through this file —
  convenient for local use; for remote use always pass `--connect` explicitly.
- `SIGINT`/`SIGTERM`: drains all connections and exits cleanly.
- `SIGHUP`: re-reads the workspace (rebuilds the board watcher) and keeps serving.

### Client lifecycle

The client attaches over a plain `ws://` WebSocket and runs the same TUI as
combined mode. If the connection drops (network hiccup, Tailscale reconnect,
daemon restart) the transport-layer **reconnect supervisor** takes over: the TUI
shows a disconnect state, retries, and restores the session view — open session
tabs are replayed from the daemon's connect-time snapshot, so closing and
reopening the client brings you back to where you were.

## Running locally (one machine)

```bash
# Terminal 1 — the factory
cd /path/to/project
fspec daemon --bind 127.0.0.1:3777

# Terminal 2 — attach a TUI to it
fspec client --connect ws://127.0.0.1:3777
```

You can attach as many local clients as you like; they all see the same sessions.

## Running remotely with Tailscale

This is the standard setup for "laptop client, workstation daemon". Tailscale's
tailnet provides the trust boundary (only your own devices, WireGuard-encrypted)
and the reachability (no ports exposed, no firewall changes).

**Prerequisite:** Tailscale is installed and logged in on every machine
(`tailscale up`). Check the daemon machine's tailnet IP: `tailscale ip -4`
(e.g. `100.106.87.78`).

### 1. Daemon (on the machine with the code)

```bash
cd /path/to/project
fspec daemon --bind 127.0.0.1:3777
```

### 2. Expose the loopback port onto the tailnet

On the daemon machine:

```bash
# One-time: allow your regular user to run `tailscale serve` without sudo
sudo tailscale set --operator=$USER

# Raw TCP forwarder: tailnet port 3777 -> local 127.0.0.1:3777
sudo tailscale serve --bg --tcp 3777 3777

# Verify: should list 3777/tcp
tailscale serve status
```

Notes:
- `--tcp <tailnet-port> <local-target>` — both are `3777` here, but any free
  port works on the tailnet side (e.g. `--tcp 3777 localhost:3777`).
- The forwarder is **raw TCP**: clients connect with plain `ws://` (not
  `wss://`). That's fine because every byte still crosses the
  WireGuard-encrypted tailnet; Tailscale simply does not front-terminate TLS.
- `--bg` detaches it to run in the background; remove it to run in the
  foreground with logs.

### 3. Clients (on any other tailnet machine)

```bash
# Machine B (laptop, second workstation, etc.)
fspec client --connect ws://100.106.87.78:3777

# Health check from anywhere on the tailnet
fspec status --connect ws://100.106.87.78:3777
```

Use the daemon machine's **tailnet IP** (`tailscale ip -4`), or a magic DNS name
like `ws://gx10-d917:3777` if you have one.

### Teardown

```bash
sudo tailscale serve clear   # removes the TCP forwarder
# stop the daemon (Ctrl-C in its terminal, or kill its pid from daemon.json)
```

## What "multiple clients, one daemon" means

Sessions live in the daemon, not in the clients:

- **Shared live sessions** — every client sees the same sessions, the same board,
  and the same streaming output. Start a job from the workstation, watch it
  stream on the laptop.
- **Cross-client lifecycle events** — session lifecycle is pushed over the wire
  (the `SessionEvent` stream: `Created` / `Cleared` / `Removed`), not polled.
  Close, or `/clear`, a session in one client and every other client's tab
  updates live. A `/clear` from the Telegram/relay bridge (see the
  [bridge section](#the-telegram-and-relay-bridge) below) clears through the
  same daemon-side path, so it resets the session in every attached TUI.
- **Reconnect-safe** — open session tabs are replayed at connect time
  (connect-time snapshot), so a dropped connection or a client restart never
  loses your view; the daemon keeps sessions running regardless of how many
  clients are attached or detached.
- **One process does the work** — LLM calls, tool execution, and checkpoints
  happen once, in the daemon. Clients are pure frontends; attaching a second
  client costs the daemon a WebSocket connection, not a second agent runtime.

## The Telegram and relay bridge

The bridge (`bridge/`, a Node.js package) is a **separate, opt-in sidecar** — it is
not part of the daemon and it does not speak the internal fspec RPC protocol. It
uses its own custom WebSocket protocol (JSON messages: `{type, session_id,
data}` envelopes, `camelCase` keys) to connect one or more fspec **sessions** to
external clients (Telegram, mobile apps, custom WS clients). Full setup lives in
[docs/TELEGRAM.md](TELEGRAM.md) and `bridge/README.md`; this section only places
it in the client/server picture.

```
  Session (in fspec / daemon)          Node.js bridge              External
 ┌──────────────────────────┐  custom  ┌──────────────────────┐    clients
 │  Bridge relay task        │ bridge   │  Telegram endpoint   │   (phone)
 │  (rust/tools/bridge_relay)│ protocol ├──────────────────────┤
 │  outbound: session chunks │────────►│  relay endpoint      │◄──────────┐
 │  inbound:  user input,    │◄─────────│  (ws server 8181)   │           │
 │         interrupt/clear   │          └──────────┬───────────┘           │
 └──────────────────────────┘                     │  channel routing      │
                                                  ▼                       │
                                            ┌──────────────┐             │
                                            │  relay server │             │
                                            │  (ws 8765)   │◄────────────┘
                                            └──────────────┘  custom/mobile clients
```

How it fits:

- **Per-session, not per-daemon.** A session connects the bridge to *itself*:
  the agent runs the `bridge` tool (e.g. "Connect to the Telegram bridge at
  `ws://localhost:8181`"), which spawns a per-session relay task
  (`rust/tools/src/bridge_relay.rs`) on the Rust side. That task streams the
  session's chunks **out** to the bridge and injects user input / control
  messages (interrupt, clear) **in** — the same `clear_history` entry point
  the TUI's `/clear` uses, which is why a phone-side clear also fans out to
  the daemon's `SessionEvent::Cleared` stream.
- **Two-hop topology.** The *Telegram endpoint* is a WS server (port 8181) the
  session connects to directly, and talks Telegram Bot API polling on the
  other side. The *relay endpoint* + *relay server* (port 8765) route the same
  JSON bridge protocol to arbitrary custom/mobile clients by `channel_id`
  (with an optional API key), so the relay side can live on a different
  machine and reach the Telegram endpoint over its own network.
- **Protocol boundary.** Across the session-to-bridge hop the messages are
  plain JSON (`text` / `thinking` / `tool_call` / `tool_result` / `error` /
  `done` chunks out; `message`, `action`, `request_id`/`command`/`args_json`
  in) — NOT the bincode RPC envelopes the daemon speaks. The bridge never
  talks to the daemon at all; it only ever talks to a session.

## Security notes

- The daemon **refuses non-loopback binds** — there is no network-level auth, so
  the transport (tailnet membership, SSH, VPN) IS the authentication. Keep it
  that way.
- The relay server is open by default; set `RELAY_SERVER_API_KEY` if the relay
  must be reachable beyond your own machines (see `bridge/README.md`).
- A tailnet-only TCP forwarder is reachable only by devices on the same
  tailnet; audit membership with `tailscale whois` and keep the tailnet personal.
- For anything beyond a personal tailnet (public hosts, shared machines),
  auth/TLS for external binds is deliberately out of scope — run the daemon
  under a trusted transport only.
