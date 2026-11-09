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
│  fspec daemon ──── binds 127.0.0.1:3777 (loopback default)  │
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

By default the daemon binds a **loopback address** (`127.0.0.1`, `::1`, or
`localhost`). Reaching it from another machine happens either through a transport
that provides the trust boundary — a Tailscale tailnet (below), an SSH tunnel, or
anything else that terminates on loopback — or through the daemon's own TLS
listener (next section).

### Remote daemon over TLS (RPC-456)

A **non-loopback** `--bind` is accepted only when the daemon serves TLS **and**
requires a bearer token; otherwise it refuses to start and names what is missing:

```bash
export FSPEC_DAEMON_TOKEN="$(openssl rand -hex 32)"
fspec daemon --bind 0.0.0.0:3777 \
  --tls-cert /etc/fspec/fullchain.pem \
  --tls-key  /etc/fspec/privkey.pem \
  --tls-name fspec.example.com
```

- `--tls-cert` is a PEM certificate chain and `--tls-key` its PEM private key
  (PKCS#8, PKCS#1 or SEC1). Both flags go together. Unreadable or invalid files stop
  the daemon with the reason.
- `FSPEC_DAEMON_TOKEN` must be set and non-empty. Every connection must send
  `Authorization: Bearer <token>`; a connection without it is rejected with 401.
- Clients connect with `wss://` and the token, e.g. a saved daemon
  (`fspec daemons add …`, `wss://host:3777` with `--token`) or
  `fspec client --connect wss://host:3777 --token …`.
- The client verifies the certificate against the public web PKI roots. For a
  private-CA or self-signed certificate (RPC-458), tell the clients which CA to
  trust beside those roots:
  - `FSPEC_DAEMON_CA=/path/ca.pem` is trusted by every client connect (the daemon
    switcher, `fspec client`, `fspec client mcp`, `fspec status`);
  - `fspec daemons add lab wss://lab.lan:3777 --token … --ca lab-ca.pem` saves the
    CA's absolute path with that daemon (`"ca"` in `~/.fspec/daemons.json`), trusted
    whenever that saved daemon is connected to.
  The certificate is still checked against the dialed host, and there is no
  skip-verification mode: a certificate that does not verify fails with
  `TLS certificate verification failed for <host>`. A CA file that is missing,
  unreadable or holds no PEM certificate is an error naming the file and
  `FSPEC_DAEMON_CA` (or the saved daemon) before any connection is tried; `--ca` is
  checked the same way before it is saved and is refused for a `ws://` daemon.
- `--tls-name` is the host the certificate is for. Local auto-discovery (the daemon
  switcher, `fspec client` and `fspec status` without `--connect`) dials
  `wss://<tls-name>:<port>`. A TLS daemon on an unspecified bind (`0.0.0.0`, `::`)
  needs `--tls-name`; on a specific address it defaults to that address.
- TLS is optional on loopback binds.

### Daemon lifecycle

- Prints the bound port to **stdout** on startup (useful with `--bind 127.0.0.1:0`
  for ephemeral ports; pin a port when you want stable remote access).
- Writes `daemon.json` to `$XDG_RUNTIME_DIR/fspec/daemon.json` (or
  `~/.fspec/daemon.json`) with `port`, `pid`, `workspace`, `started_at`, `version`,
  `scheme` (`ws` or `wss`) and `host` (the host a local client dials: the `--bind`
  host, `127.0.0.1` for a plain daemon on `0.0.0.0`/`::`, or `--tls-name`). An
  entry without `scheme`/`host` (written by an older fspec) means `ws` and
  `127.0.0.1`.
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

- The daemon **refuses non-loopback binds unless it serves TLS and requires a
  bearer token** (RPC-456). On loopback binds without a token, the transport
  (tailnet membership, SSH, VPN) IS the authentication.
- The relay server is open by default; set `RELAY_SERVER_API_KEY` if the relay
  must be reachable beyond your own machines (see `bridge/README.md`).
- A tailnet-only TCP forwarder is reachable only by devices on the same
  tailnet; audit membership with `tailscale whois` and keep the tailnet personal.
- For anything beyond a personal tailnet (public hosts, shared machines), use the
  TLS listener with a long random `FSPEC_DAEMON_TOKEN`, and keep the token secret.
