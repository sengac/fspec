/*
 * FV-003: DAG Compaction — Formal Model
 *
 * Verifies invariants of the hierarchical DAG-based context compaction.
 *
 * Source files:
 *   codelet/core/src/compaction/model.rs   (DagDepth, DagNodeMeta,
 *                                            parse_dag_nodes, wrap_dag_content,
 *                                            ConversationTurn)
 *   codelet/core/src/compaction_hook.rs
 *   codelet/tools/src/inject_summary.rs
 *
 * Run with: alloy exec -q -f -c <name> dag_compaction.als
 *       or: open in Alloy Analyzer 6 GUI and run all `check` commands.
 *
 * Model approach
 * ──────────────
 * We encode the *guarantees of the compaction algorithm* as facts, then
 * prove derived properties (consistency, contiguity, depth-boundedness,
 * empty-input handling, satisfiability).
 *
 * Each fact corresponds to a property the implementation MUST uphold.
 * If the implementation drifts from a fact, a corresponding `proptest` will
 * catch it (cross-checked via the doc-comment annotations on the Rust side).
 */

module dag_compaction

// ────────────────────────────────────────────────────────────────────────────
// SIGNATURES
// ────────────────────────────────────────────────────────────────────────────

/*
 * Three depth levels per the doc comment in model.rs:
 *   D0 — Detailed (recent work, granular)
 *   D1 — Arc (current work state, promoted from D0)
 *   D2 — Durable (architecture decisions, milestones)
 */
abstract sig DagDepth {}
one sig D0, D1, D2 extends DagDepth {}

/*
 * A conversation turn at index `idx` (0-based).
 * `present` = True means the turn is still in the message log after compaction
 * (possibly trimmed). False means it was REMOVED in favour of a covering
 * DagNode.
 */
sig Turn {
    idx     : one Int,
    present : one Bool
}

abstract sig Bool {}
one sig True, False extends Bool {}

/*
 * A DAG node summarising turns [turnStart, turnEnd] inclusive.
 */
sig DagNode {
    depth     : one DagDepth,
    turnStart : one Int,
    turnEnd   : one Int
}

/*
 * The session: a fixed set of turns and the DAG nodes produced by
 * compaction.
 */
one sig Session {
    turns     : set Turn,
    dagNodes  : set DagNode
}

// ────────────────────────────────────────────────────────────────────────────
// WELL-FORMEDNESS FACTS — domain constraints
// ────────────────────────────────────────────────────────────────────────────

/* Turn indices are non-negative and unique. */
fact TurnsWellFormed {
    all t: Session.turns | t.idx >= 0
    all disj t1, t2: Session.turns | t1.idx != t2.idx
}

/* All Turns belong to the session (no orphans). */
fact TurnsScoped {
    Turn = Session.turns
}

/* All DagNodes belong to the session (no orphans). */
fact NodesScoped {
    DagNode = Session.dagNodes
}

/*
 * DAG node ranges are valid:
 *  - turnStart >= 0
 *  - turnStart <= turnEnd
 *  - turnEnd < message_count (parse_dag_nodes clamps to this)
 */
fact NodeRangesWellFormed {
    all n: Session.dagNodes |
        n.turnStart >= 0
        and n.turnStart <= n.turnEnd
        and n.turnEnd < #Session.turns
}

// ────────────────────────────────────────────────────────────────────────────
// FUNCTIONS
// ────────────────────────────────────────────────────────────────────────────

/* Turns whose idx is in [turnStart, turnEnd]. */
fun coveredBy[n: DagNode]: set Turn {
    { t: Session.turns | t.idx >= n.turnStart and t.idx <= n.turnEnd }
}

/* Turns covered by ANY DAG node. */
fun covered: set Turn {
    { t: Session.turns | some n: Session.dagNodes | t in coveredBy[n] }
}

// ────────────────────────────────────────────────────────────────────────────
// COMPACTION ALGORITHM GUARANTEES (encoded as facts)
// ────────────────────────────────────────────────────────────────────────────
//
// These facts encode the contract of the compaction implementation. If the
// Rust code violates one of these, the corresponding `proptest` will fail —
// the Alloy proof relies on the implementation upholding these guarantees.

/*
 * G1: Every removed turn must be covered by at least one DAG node.
 *
 * If a turn is dropped from the message log, the agent's context must still
 * include a summary covering it — otherwise information is silently lost.
 *
 * Implemented in compaction_hook.rs by ensuring inject_summary completes
 * before the turn is removed.
 */
fact G1_RemovedTurnsCovered {
    all t: Session.turns | t.present = False implies t in covered
}

/*
 * G2: No two DAG nodes at the same depth share any turn.
 *
 * Promotion (D0 -> D1, D1 -> D2) consumes the source nodes; the same depth
 * is never duplicated for overlapping ranges.
 */
fact G2_SameDepthNonOverlapping {
    all disj n1, n2: Session.dagNodes |
        n1.depth = n2.depth implies no (coveredBy[n1] & coveredBy[n2])
}

// ────────────────────────────────────────────────────────────────────────────
// DERIVED INVARIANTS — properties to verify
// ────────────────────────────────────────────────────────────────────────────

/*
 * INV-1: No turn silently lost.
 *
 * Restatement of G1 in the form "every turn is preserved or summarised".
 * If G1 holds, so does this. Alloy verifies the implication.
 */
assert NoTurnLost {
    all t: Session.turns | t.present = True or t in covered
}
check NoTurnLost for 7

/*
 * INV-2: Coverage at same depth is non-overlapping.
 *
 * Restatement of G2 — a sanity assertion.
 */
assert SameDepthNonOverlapping {
    all disj n1, n2: Session.dagNodes |
        n1.depth = n2.depth implies no (coveredBy[n1] & coveredBy[n2])
}
check SameDepthNonOverlapping for 7

/*
 * INV-3: Coverage is contiguous within a node.
 *
 * Every turn whose idx is in [turnStart, turnEnd] is in coveredBy[n].
 * This holds by construction of `coveredBy`; included as a sanity check
 * that the model is internally consistent.
 */
assert NodeCoverageContiguous {
    all n: Session.dagNodes, t: Session.turns |
        (t.idx >= n.turnStart and t.idx <= n.turnEnd) implies t in coveredBy[n]
}
check NodeCoverageContiguous for 7

/*
 * INV-4: DAG depth is bounded.
 *
 * Every DAG node has depth in {D0, D1, D2} — no other depths exist.
 * This guards against infinite recursion in parse_dag_nodes.
 */
assert DepthBounded {
    all n: Session.dagNodes | n.depth in (D0 + D1 + D2)
}
check DepthBounded for 7

/*
 * INV-5: A "removed" turn (present = False) MUST be covered by a DAG node.
 *
 * Contrapositive of NoTurnLost — explicit form for clarity.
 */
assert RemovedRequiresCoverage {
    all t: Session.turns | t.present = False implies t in covered
}
check RemovedRequiresCoverage for 7

/*
 * INV-6: Empty-session base case.
 *
 * If there are no turns, there are no DAG nodes (nothing to summarise).
 * Follows from NodeRangesWellFormed: turnEnd < #Session.turns means with
 * zero turns no valid range exists.
 */
assert EmptySessionNoDag {
    no Session.turns implies no Session.dagNodes
}
check EmptySessionNoDag for 5

/*
 * INV-7: A node's range size is at most #Session.turns.
 *
 * Sanity check on bounds.
 */
assert NodeRangeBounded {
    all n: Session.dagNodes |
        plus[minus[n.turnEnd, n.turnStart], 1] <= #Session.turns
}
check NodeRangeBounded for 7

// ────────────────────────────────────────────────────────────────────────────
// EXAMPLE RUNS — sanity checks that the constraints are satisfiable
// ────────────────────────────────────────────────────────────────────────────

/*
 * Cross-depth nesting is allowed (D2 may cover turns also covered by D0).
 * Durable summaries subsume detailed ones.
 */
run CrossDepthNestingAllowed {
    some disj n0, n2: Session.dagNodes |
        n0.depth = D0 and n2.depth = D2
        and (some coveredBy[n0] & coveredBy[n2])
} for 6

/*
 * A typical compaction: 5 turns, turns 0..2 summarised by a D0 node and
 * removed; turns 3..4 remain present.
 */
run TypicalCompaction {
    #Session.turns = 5
    some n: Session.dagNodes |
        n.depth = D0 and n.turnStart = 0 and n.turnEnd = 2
        and (all t: coveredBy[n] | t.present = False)
    some t: Session.turns | t.idx >= 3 and t.present = True
} for 6

/*
 * Empty session (zero turns) is satisfiable and produces no DAG nodes.
 */
run EmptySessionInstance {
    no Session.turns
} for 3
