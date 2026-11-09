# REFAC-060 design

Move data_dir, fresh_session, run_still_running, close_exec, drained_chunks, wait_for_stored_request and wait_for_chunks to tests/exec_stdin_push_chunk_bug171_support/mod.rs. The ten tests (~700 lines) need three features: push on fire/clear-slot/child-exit/write (BUG-171 originals), detector clear on output/child-exit/removal, and cooldown/never-fired. Split exec-stdin-push-emission.feature accordingly (each new feature keeps the owning card's @ID tag), one test file each, and move the coverage entries with their scenarios.
