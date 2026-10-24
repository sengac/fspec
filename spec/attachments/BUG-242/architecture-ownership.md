# BUG-242 early architecture and ownership

Research: `compaction_execution.rs` drops timed-out exploration; Codex commits
terminal output before yielding calls. `rig-core/src/tool/server.rs` awaits tools
in its independent actor, so dropping a requester cannot prove cancellation.

The initial conservative failure-only contract is superseded by full-repair
approval in architecture-approval.md. Missing results now require truthful runtime
outcome-unknown records at the synthesis phase boundary, explicitly distinguished
from tool-produced results. The actor may continue; no success, cancellation or
non-execution is inferred and unknown side effects must not be retried. Genuine
results retain precedence. Synthesis still requires admitted complete lineage and
the unchanged no-fallback full-source DAG publication gates. Invalid lineage fails.
No isolated generation, checkpoint reset, weakened replay or fabricated success.

Planned files: agent-loop compaction_execution/runway, a small settlement helper,
rig CompletionModel pending-identity observation hook and Codex implementation,
new actual-Codex integration tests. Provider history mutation/lineage and cli
session provider code remain BUG-243's ownership. No persistence or background
output edits. A later cancellation-and-ack protocol would be a separate design;
this repair does not change independent tool server execution semantics. Unknown
handoff records are added to synthesis history before normal native serialization;
the ordinary provider replay validator must still accept the complete request.

AST research: `tokio::time::timeout_at($$$ARGS)` identified the exploration and
synthesis boundaries in compaction_execution plus the enclosing builder deadline.
Direct source research identified provider checkpoint publication and rig actor
execution; these establish why cancellation outcomes cannot safely be fabricated.
