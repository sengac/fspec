# Windows Notes

Platform-specific issues that can occur when building or using fspec on Windows,
with the diagnostics and fixes that were verified against them.

---

## SSH / Git: `Permission denied (publickey)` over an SSH remote

The most common Windows gotcha: an `https://` remote was switched to
`git@github.com:...` (or any other SSH remote) and every fetch/pull/push fails
with:

```
git@github.com: Permission denied (publickey).
```

Even though the same key works fine on macOS/Linux (or even in WSL on the
same machine).

### Why it happens — three separate issues stack up

1. **Git for Windows bundles its own ssh.** `git` does not use
   `C:\Windows\System32\OpenSSH\ssh.exe`; it uses the msys2 build at
   `C:\Program Files\Git\usr\bin\ssh.exe` (a different, newer OpenSSH).
   Anything you configure or troubleshoot against the *system* ssh (agent
   service, `known_hosts` quirks, KEX) does not automatically apply to the
   one git actually invokes.

2. **Git's bundled ssh cannot reach the Windows ssh-agent *service*.**
   Keys loaded into the service (`ssh-add` → `ssh-agent` service,
   AutoStart) are invisible to the bundled msys2 ssh: it treats
   `SSH_AUTH_SOCK=ssh-agent:` as a Unix socket path and fails with
   `ssh_get_authentication_socket: No such file or directory`. The system
   ssh auto-discovers the service and works.

3. **Passphrase-protected keys + `BatchMode`.** If the private key is
   protected by a passphrase and no agent holds the *unlocked* key, the
   client offers the public half ("Server accepts key") but cannot sign,
   and GitHub rejects the connection. This looks identical to "wrong key"
   but the key is correct — the verbose trace is the only way to tell:

   ```
   debug1: Offering public key: /c/Users/.../id_ed25519 ...
   debug1: Server accepts key: ...        <- fingerprint IS on GitHub
   debug1: No more authentication methods to try.
   git@github.com: Permission denied (publickey).
   ```

   "Server accepts key" + "Permission denied" = recognized public key,
   failed signature = **passphrase/agent problem, not a registration problem**.

### Diagnostic sequence (run these first)

```powershell
# 1. Which ssh is git actually using, and what does its trace say?
& 'C:\Program Files\Git\usr\bin\ssh.exe' -vT -o BatchMode=yes git@github.com
#    (system client: C:\Windows\System32\OpenSSH\ssh.exe -V)

# 2. Is the key passphrase-protected? (EOF on stdin => "incorrect passphrase" if yes)
ssh-keygen -y -f $env:USERPROFILE\.ssh\id_ed25519

# 3. What does the agent hold?
ssh-add -L
```

### Fix (verified)

Use the **system** ssh client, which auto-discovers the Windows
`ssh-agent` service (key must be loaded there once per service/session):

```powershell
# one-time: make git use the system ssh for all remotes (forward slashes!
# git parses core.sshCommand through the msys shell, which eats backslashes)
git config --global core.sshCommand "C:/Windows/System32/OpenSSH/ssh.exe"

# load the key into the running agent service (types the passphrase once)
Start-Service ssh-agent      # if not already running
ssh-add $env:USERPROFILE\.ssh\id_ed25519

# verify through git, not just ssh
git ls-remote origin HEAD
```

Notes:

- `core.sshCommand` values containing backslashes arrive as
  `C:WindowsSystem32...` (stripped) — always use forward slashes.
- The agent load is **not persistent** across the `ssh-agent` service
  restart or logoff (the service's key cache is in RAM). For persistence,
  a startup hook that re-runs `ssh-add` (e.g. via Task Scheduler with the
  passphrase cached by `ssh-add --use-ssh-agent` + winsshd) is the standard
  remedy; out of scope for this doc.
- GitHub offers post-quantum KEX algorithms
  (`sntrup761x25519-sha512@openssh.com`) that older OpenSSH cannot
  implement (client must be 9.6+ to use them; it falls back to the
  classical KEXes when it's not offered/unsupported). If you see
  `choose_kex: unsupported KEX method sntrup761x25519-sha512@openssh.com`,
  update the ssh client rather than working around it.

---

## PowerShell script encoding (build/install scripts)

Windows PowerShell 5.1 (`powershell.exe`) decodes BOM-less `.ps1` files with
the system **ANSI code page** (CP1252 on en-US). Any UTF-8 non-ASCII byte in
a string literal can corrupt the parse — e.g. a checkmark `✓` (U+2713)
contains byte `0x93`, which CP1252 maps to a **double-quote**, breaking
every string on that line.

- All `scripts/*.ps1` (build.ps1, build-install.ps1, install.ps1) are
  therefore **pure ASCII on purpose**. Keep them that way.
- If a script suddenly fails with baffling parse errors
  (`The '<' operator is reserved for future use`, `The string is missing
  the terminator`), check for non-ASCII bytes first:

  ```powershell
  $b = [IO.File]::ReadAllBytes('scripts\install.ps1')
  ($b | Where-Object { $_ -gt 127 }).Count   # 0 expected
  ```

- `pwsh` (PowerShell 7+) defaults to UTF-8, so the same file may parse
  fine there — **test scripts on `powershell.exe` (5.1)**, the one
  `irm ... | iex` one-liners use by default on stock Windows.

---

## Build / install specifics (cross-reference)

- Full Windows build/install instructions: [BUILD.md](BUILD.md)
  (`build.ps1`, `build-install.ps1`, `install.ps1`).
- The Windows binary is ~275 MB (release-slim) vs ~150 MB on macOS —
  expected (MSVC CRT, static linking), not a sign of corruption.
- `protoc` is a build-time-only dependency; get it from
  <https://github.com/protocolbuffers/protobuf/releases>
  (`protoc-<version>-win64.zip`) and add its `bin\` to PATH.
- Memory: `rust/.cargo/config.toml` caps `build.jobs = 4` to avoid OOM on
  large machines; override with `CARGO_BUILD_JOBS` if you have headroom.
