/*
 * FV-001: Compaction Trimmer — Formal Model
 *
 * Verifies the invariants of the structurally lossless trimmer (Layer 0 of
 * hierarchical compaction).
 *
 * Source files:
 *   codelet/core/src/compaction/trimmer.rs
 *   codelet/core/src/compaction/trimmer_metadata.rs
 *
 * Run with: alloy execute trimmer.als
 *       or: open in Alloy Analyzer 6 GUI and run all `check` commands.
 *
 * Domain
 * ──────
 * The Trimmer maintains a registry mapping `tool_use_id` to (tool_name, input).
 * Assistant messages REGISTER tool uses; user messages may carry tool RESULTS
 * that are looked up in the registry.
 *
 * Critical safety property: a ToolResult must never be misinterpreted because
 * the registry was modified mid-stream. Specifically:
 *  - The registry is append-only within a session (no overwrites with
 *    different tool names)
 *  - A user ToolResult with an unknown tool_use_id must fall back to safe
 *    heuristics, never to a tool-specific path.
 */

module trimmer

// ────────────────────────────────────────────────────────────────────────────
// SIGNATURES
// ────────────────────────────────────────────────────────────────────────────

/* Tool kinds the Trimmer recognises. */
abstract sig ToolName {}
one sig Read, Write, Edit, Bash, Grep, AstGrep, Glob, Ls, Other extends ToolName {}

/*
 * A unique tool_use_id. Each ToolUse has exactly one id; ids never collide
 * within a session (Anthropic generates UUIDs).
 */
sig ToolUseId {}

/*
 * A registered tool use. Inserted into the registry by an assistant message.
 */
sig ToolUse {
    id   : one ToolUseId,
    name : one ToolName
}
fact UniqueIds {
    all disj t1, t2: ToolUse | t1.id != t2.id
}

/*
 * Message kinds.
 */
abstract sig Message {}

/* Assistant message that registers zero or more tool uses. */
sig AssistantMessage extends Message {
    registers : set ToolUse
}

/* User message carrying a ToolResult. */
sig UserToolResult extends Message {
    refsId  : one ToolUseId,    // tool_use_id this result references
    isImage : one Bool
}

/* User message with plain content (no tool result). */
sig UserPlain extends Message {}

abstract sig Bool {}
one sig True, False extends Bool {}

/*
 * Registry state. `entries` is the set of ToolUses currently registered.
 */
one sig Registry {
    var entries : set ToolUse
}

/*
 * Outcome of trimming a UserToolResult — used to verify safety of the
 * lookup path.
 */
abstract sig TrimOutcome {}
one sig
    TrimReadPath,        // matched Read in registry
    TrimBashPath,        // matched Bash in registry
    TrimGrepPath,        // matched Grep/AstGrep/Glob/Ls in registry
    TrimImagePath,       // base64 image content
    TrimHeuristicPath    // fell back to content-based heuristics (safe default)
extends TrimOutcome {}

// ────────────────────────────────────────────────────────────────────────────
// FACTS
// ────────────────────────────────────────────────────────────────────────────

fact Init {
    no Registry.entries
}

/*
 * Trace: at every step, exactly one message is processed (or a stutter).
 *
 * Modelled abstractly: either an AssistantMessage adds its `registers` set
 * to the registry, or a UserMessage is processed (registry unchanged), or
 * stutter.
 */
pred processAssistant[m: AssistantMessage] {
    Registry.entries' = Registry.entries + m.registers
    // No overwrite of existing IDs with conflicting tool names.
    all t: m.registers | no other: Registry.entries | t.id = other.id and t != other
}

pred processUser {
    Registry.entries' = Registry.entries
}

pred stutter {
    Registry.entries' = Registry.entries
}

fact Traces {
    always (
        stutter
        or (some m: AssistantMessage | processAssistant[m])
        or processUser
    )
}

// ────────────────────────────────────────────────────────────────────────────
// FUNCTIONS — model the trim_user_message dispatch logic
// ────────────────────────────────────────────────────────────────────────────

/*
 * Lookup result: the ToolUse registered for this id, if any.
 * Empty set means "not registered" — falls back to heuristics.
 */
fun lookup[useId: ToolUseId, reg: set ToolUse]: set ToolUse {
    { t: reg | t.id = useId }
}

/*
 * Model trim_user_message dispatch:
 *   if image -> TrimImagePath
 *   else look up tool_use_id:
 *     Read -> TrimReadPath
 *     Bash -> TrimBashPath
 *     Grep|AstGrep|Glob|Ls -> TrimGrepPath
 *     unknown / Other -> TrimHeuristicPath
 *     Write|Edit registered but result reaching here -> TrimHeuristicPath
 *       (Write/Edit are trimmed in the assistant path, not here)
 */
pred dispatch[msg: UserToolResult, reg: set ToolUse, outcome: TrimOutcome] {
    msg.isImage = True implies outcome = TrimImagePath
    else (
        let t = lookup[msg.refsId, reg] |
            (no t and outcome = TrimHeuristicPath)
            or (one t and t.name = Read    and outcome = TrimReadPath)
            or (one t and t.name = Bash    and outcome = TrimBashPath)
            or (one t and t.name in (Grep + AstGrep + Glob + Ls) and outcome = TrimGrepPath)
            or (one t and t.name in (Write + Edit + Other)        and outcome = TrimHeuristicPath)
    )
}

// ────────────────────────────────────────────────────────────────────────────
// INVARIANTS
// ────────────────────────────────────────────────────────────────────────────

/*
 * INV-1: Tool-registry append-only. Once an id is registered with a name,
 * the registry never replaces it with a different name.
 *
 * This is enforced by the model — assert it for clarity.
 */
assert RegistryAppendOnly {
    always (
        all t: Registry.entries |
            t in Registry.entries' or t.id not in Registry.entries'.id
    )
}
check RegistryAppendOnly for 5 but 10 steps

/*
 * Stronger: no two distinct ToolUses with the same id can ever both be in
 * the registry simultaneously. (Implied by UniqueIds + monotonic add, but
 * worth stating explicitly.)
 */
assert NoDuplicateIds {
    always (
        all disj t1, t2: Registry.entries | t1.id != t2.id
    )
}
check NoDuplicateIds for 5 but 10 steps

/*
 * INV-2: No ToolResult trimmed by a tool-specific path without a matching
 * registry entry.
 *
 * If dispatch chooses TrimReadPath, the registry MUST contain a Read entry
 * for the referenced id. Same for Bash, Grep, etc.
 */
assert ToolPathRequiresRegistration {
    all msg: UserToolResult, reg: set ToolUse, o: TrimOutcome |
        dispatch[msg, reg, o] implies (
            (o = TrimReadPath  implies some t: reg | t.id = msg.refsId and t.name = Read)
            and
            (o = TrimBashPath  implies some t: reg | t.id = msg.refsId and t.name = Bash)
            and
            (o = TrimGrepPath  implies some t: reg | t.id = msg.refsId and t.name in (Grep + AstGrep + Glob + Ls))
        )
}
check ToolPathRequiresRegistration for 5

/*
 * INV-3: Unknown id => heuristic fallback (safe default).
 *
 * If the referenced id is NOT in the registry, the outcome must be either
 * TrimImagePath (because of base64 detection, which doesn't need the
 * registry) or TrimHeuristicPath. Never a tool-specific path.
 */
assert UnknownIdFallsBackSafely {
    all msg: UserToolResult, reg: set ToolUse, o: TrimOutcome |
        (dispatch[msg, reg, o] and msg.refsId not in reg.id) implies
            (o = TrimImagePath or o = TrimHeuristicPath)
}
check UnknownIdFallsBackSafely for 5

/*
 * INV-4: Image content always takes the image path, regardless of registry.
 * This matches the implementation where `is_base64_image(content)` is
 * checked BEFORE the registry lookup.
 */
assert ImageAlwaysImagePath {
    all msg: UserToolResult, reg: set ToolUse, o: TrimOutcome |
        (dispatch[msg, reg, o] and msg.isImage = True) implies o = TrimImagePath
}
check ImageAlwaysImagePath for 5

/*
 * INV-5: Dispatch is deterministic — for any (msg, reg) there is exactly
 * one outcome.
 */
assert DispatchDeterministic {
    all msg: UserToolResult, reg: set ToolUse |
        one o: TrimOutcome | dispatch[msg, reg, o]
}
check DispatchDeterministic for 5

// ────────────────────────────────────────────────────────────────────────────
// EXAMPLE RUNS
// ────────────────────────────────────────────────────────────────────────────

/* Show a non-trivial trace: register a Read tool use, then a UserToolResult
 * referencing it, dispatched on the Read path. */
run RegisterThenLookup {
    some m: AssistantMessage, msg: UserToolResult, t: ToolUse |
        t in m.registers
        and t.name = Read
        and msg.refsId = t.id
        and msg.isImage = False
        and eventually (t in Registry.entries and dispatch[msg, Registry.entries, TrimReadPath])
} for 4 but 8 steps
