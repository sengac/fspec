# RPC-070 — Reproduction guide

This file documents the exact reproduction we used to confirm the bug shown in the
user-supplied screenshot (`/Users/rquast/Desktop/Screenshot 2026-05-26 at 3.15.56 pm.png`).

## 1. Build the Rust `fspec` binary

```bash
bash /tmp/build_fspec_bin.sh
```

Equivalent inline:

```bash
cd /Users/rquast/projects/fspec/codelet
cargo build -p codelet-fspec --bin fspec --no-default-features
# Produces: codelet/target/debug/fspec
```

## 2. Run the smoke test (control — confirms the binary boots)

```bash
cd /Users/rquast/projects/fspec
bash /tmp/run_rust_smoke3.sh
```

Equivalent:

```bash
npx @microsoft/tui-test e2e/rpc-068-rust-binary-smoke.test.ts
```

Expected output: `2 passed`. The board renders, columns are visible, no panic.

## 3. Run the panic reproduction

```bash
cd /Users/rquast/projects/fspec
bash /tmp/run_work_agent.sh
```

Equivalent:

```bash
npx @microsoft/tui-test e2e/rpc-068-work-agent-panic-repro.test.ts
```

Expected output: `1 failed`, with the captured buffer containing the
`Cannot start a runtime from within a runtime` panic.

## 4. Inspect the captured buffer

The PTY contents are written to `/tmp/rust_fspec_post_enter.txt` and persisted in
this attachment directory as `panic-backtrace-captured.txt` for permanent record.

```bash
sed -n '1,90p' spec/attachments/RPC-070/panic-backtrace-captured.txt
```

## 5. What the test does

```ts
// e2e/rpc-068-work-agent-panic-repro.test.ts
test('pressing Enter on a work unit (Work Agent) panics …', async ({ terminal }) => {
  terminal.write(`cd ${HOME} && ${BIN}\r`);   // launch the Rust binary in a real PTY
  await expect.poll(() => terminal.serialize()).toContain('done');
  terminal.write('\u001b[C\u001b[C\u001b[C'); // ←/→/→/→ to the DONE column
  terminal.write('\r');                       // Enter — should open Work Agent
  await expect.poll(() => terminal.serialize()).toContain('panicked');
});
```

The `expect.poll` for the word `panicked` is the assertion that fails today and
must pass (or be deleted) after RPC-070 lands. Once the bug is fixed the assertion
should be flipped to:

```ts
await expect.poll(() => terminal.serialize()).toContain('Agent');
await expect.poll(() => terminal.serialize()).not.toContain('panicked');
```

## 6. Files involved

| Path | Purpose |
|------|---------|
| `e2e/rpc-068-rust-binary-smoke.test.ts` | Smoke test — board renders (PASSES today) |
| `e2e/rpc-068-work-agent-panic-repro.test.ts` | Panic repro (FAILS today, MUST pass after fix) |
| `/tmp/build_fspec_bin.sh` | Build script |
| `/tmp/run_rust_smoke3.sh` | Smoke-test runner |
| `/tmp/run_work_agent.sh` | Panic-repro runner |
| `spec/attachments/RPC-070/panic-backtrace-captured.txt` | Captured PTY buffer with backtrace |

## 7. Tear-down

The binary opens a daemon socket at `~/.fspec/daemon.sock`. Tests automatically
clean that up; if a stray daemon is left running:

```bash
pkill -f 'fspec' || true
rm -f ~/.fspec/daemon.sock
```
