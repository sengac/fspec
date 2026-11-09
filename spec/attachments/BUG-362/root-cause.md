# BUG-362 root cause

`write_exec_stdin` (`rust/sessions/src/handle_impl/handle_impl_4.rs`) reports success once the payload is queued on the exec session's `mpsc::Sender<Vec<u8>>`. It never learns whether the bytes reached the process. The forwarding tasks in `rust/tools/src/unified_exec/spawning.rs` discard every error:
- Pipe mode (`spawn_pipe_process`): `let _ = stdin.write_all(&data).await; let _ = stdin.flush().await;`. A command that closed stdin (EPIPE) is never reported, and a missing `child.stdin` silently drops the data.
- PTY mode (`spawn_pty_process`): `let _ = tokio::task::spawn_blocking(..).await` drops a join error, `if let Ok(mut w) = writer.lock()` drops a poisoned-mutex error, and `let _ = w.write_all(&data); let _ = w.flush();` drop the io errors.

So the user and the model are told a write succeeded when it did not. TUI-117 ("a failed write keeps the text and shows a red inline error") cannot work for these failures.

Fix direction: send each write with a `oneshot::Sender<io::Result<()>>` acknowledgement, or switch to a request type that carries one. The forwarder replies with the real result, `write_exec_stdin` and the unified_exec write action await it, and a missing stdin becomes an explicit error.
