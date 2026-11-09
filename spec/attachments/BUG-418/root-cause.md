# BUG-418 root cause

The source-shape test reads only rust/agent-loop/src/agent_loop.rs via read_source. The agent loop was split into
the agent_loop/ module tree (state.rs, prompt.rs, finalization.rs, ...), so 'loop_abort_retry_count = 0'
(agent_loop/prompt.rs:93) is no longer in the file it reads and the last assertion fails. Same pattern as BUG-393.
Fix: read the composed module tree (codelet_test_helpers::source_composition::read_tree_or_file) for all four assertions.
