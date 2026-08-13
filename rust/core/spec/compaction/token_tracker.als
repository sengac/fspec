/*
 * FV-002: Token Tracker — Formal Model
 *
 * Verifies the invariants documented in:
 *   codelet/core/src/compaction/model.rs   (TokenTracker, CTX-003)
 *   codelet/core/src/token_usage.rs        (ApiTokenUsage, PROV-001)
 *
 * Run with: alloy execute token_tracker.als
 *       or: open in Alloy Analyzer 6 GUI and run all `check` commands.
 *
 * The model uses Alloy 6 temporal logic (`var sig`, `always`, `eventually`)
 * to model the TokenTracker as a state machine that evolves through API
 * responses and compaction events.
 */

module token_tracker

// ────────────────────────────────────────────────────────────────────────────
// SIGNATURES
// ────────────────────────────────────────────────────────────────────────────

/*
 * ApiTokenUsage models a single API response.
 *
 * Per Anthropic docs (PROV-001) the three input-token fields are DISJOINT
 * sets — fresh (input_tokens), cache_read, cache_creation. Their sum is the
 * total context size for input.
 *
 * We use Int as the carrier; Alloy ints are bounded but adequate for
 * checking arithmetic invariants at small scope.
 */
sig ApiTokenUsage {
    input        : one Int,   // fresh tokens (not from cache, not creating)
    cacheRead    : one Int,   // tokens read from existing cache
    cacheCreate  : one Int,   // tokens being written to cache
    output       : one Int,   // output tokens this request
    reasoning    : one Int    // reasoning/thinking tokens this request
}

/*
 * TokenTracker models the persistent session state.
 *
 * `var` makes each field time-varying — Alloy 6 will explore traces where
 * the values change across discrete time steps.
 */
one sig TokenTracker {
    var inputTokens             : one Int,   // ABSOLUTE — current context size
    var outputTokens            : one Int,   // session-wide cumulative output
    var cumulativeBilledInput   : one Int,   // monotone — sum of fresh input
    var cumulativeBilledOutput  : one Int,   // monotone — sum of output
    var cacheRead               : lone Int,  // latest value (Option<u64>)
    var cacheCreation           : lone Int,  // latest value (Option<u64>)
    var reasoningTokens         : one Int    // latest value
}

// ────────────────────────────────────────────────────────────────────────────
// FACTS — structural constraints that always hold
// ────────────────────────────────────────────────────────────────────────────

/*
 * Token counts are non-negative. Rust uses u64; Alloy Int is signed, so we
 * model the unsigned domain explicitly.
 */
fact NonNegativeUsage {
    all u: ApiTokenUsage |
        u.input >= 0 and u.cacheRead >= 0 and u.cacheCreate >= 0
        and u.output >= 0 and u.reasoning >= 0
}

fact NonNegativeTracker {
    always {
        TokenTracker.inputTokens >= 0
        TokenTracker.outputTokens >= 0
        TokenTracker.cumulativeBilledInput >= 0
        TokenTracker.cumulativeBilledOutput >= 0
        TokenTracker.reasoningTokens >= 0
    }
}

/*
 * Initial state: all zero, no cache values yet.
 * Mirrors `TokenTracker::default()` in Rust.
 */
fact Init {
    TokenTracker.inputTokens = 0
    TokenTracker.outputTokens = 0
    TokenTracker.cumulativeBilledInput = 0
    TokenTracker.cumulativeBilledOutput = 0
    TokenTracker.reasoningTokens = 0
    no TokenTracker.cacheRead
    no TokenTracker.cacheCreation
}

// ────────────────────────────────────────────────────────────────────────────
// FUNCTIONS — pure, derived values
// ────────────────────────────────────────────────────────────────────────────

/*
 * total_input() per PROV-001:
 *     input + cache_read + cache_creation
 */
fun totalInput[u: ApiTokenUsage]: Int {
    plus[plus[u.input, u.cacheRead], u.cacheCreate]
}

/*
 * total_context() = total_input + output + reasoning
 */
fun totalContext[u: ApiTokenUsage]: Int {
    plus[plus[totalInput[u], u.output], u.reasoning]
}

// ────────────────────────────────────────────────────────────────────────────
// PREDICATES — state transitions
// ────────────────────────────────────────────────────────────────────────────

/*
 * `update_from_usage(usage, cumulative_output)` per Rust impl:
 *
 *   self.input_tokens             = usage.total_input()       (overwrite)
 *   self.output_tokens            = cumulative_output         (overwrite)
 *   self.cumulative_billed_input  += usage.input_tokens        (accumulate)
 *   self.cumulative_billed_output += usage.output_tokens       (accumulate)
 *   self.cache_read_input_tokens  = Some(usage.cache_read)
 *   self.cache_creation_input_tokens = Some(usage.cache_create)
 *   self.reasoning_tokens         = usage.reasoning_tokens
 *
 * Note: cumulative_billed_input accumulates `usage.input_tokens` (raw fresh
 * input), NOT `usage.total_input()`. This is critical — billing should only
 * count fresh tokens, not cache-read tokens.
 *
 * The `cumulativeOutput` parameter must be >= the previous outputTokens
 * (the streaming display reports running total). Saturating subtraction in
 * Rust means it never goes negative.
 */
pred updateFromUsage[u: ApiTokenUsage, cumulativeOutput: Int] {
    cumulativeOutput >= 0
    TokenTracker.inputTokens'            = totalInput[u]
    TokenTracker.outputTokens'           = cumulativeOutput
    TokenTracker.cumulativeBilledInput'  = plus[TokenTracker.cumulativeBilledInput, u.input]
    TokenTracker.cumulativeBilledOutput' = plus[TokenTracker.cumulativeBilledOutput, u.output]
    TokenTracker.cacheRead'              = u.cacheRead
    TokenTracker.cacheCreation'          = u.cacheCreate
    TokenTracker.reasoningTokens'        = u.reasoning
}

/*
 * `reset_after_compaction()` per Rust impl:
 *
 *   self.output_tokens   = 0
 *   self.reasoning_tokens = 0
 *   self.cache_read_input_tokens     = None
 *   self.cache_creation_input_tokens = None
 *   // cumulative_billed_* preserved
 *   // input_tokens set later by execute_compaction (modeled separately)
 */
pred resetAfterCompaction {
    TokenTracker.outputTokens'           = 0
    TokenTracker.reasoningTokens'        = 0
    no TokenTracker.cacheRead'
    no TokenTracker.cacheCreation'
    // preserved fields
    TokenTracker.inputTokens'            = TokenTracker.inputTokens
    TokenTracker.cumulativeBilledInput'  = TokenTracker.cumulativeBilledInput
    TokenTracker.cumulativeBilledOutput' = TokenTracker.cumulativeBilledOutput
}

/*
 * Stutter: nothing changes (allows traces of arbitrary length).
 */
pred stutter {
    TokenTracker.inputTokens'            = TokenTracker.inputTokens
    TokenTracker.outputTokens'           = TokenTracker.outputTokens
    TokenTracker.cumulativeBilledInput'  = TokenTracker.cumulativeBilledInput
    TokenTracker.cumulativeBilledOutput' = TokenTracker.cumulativeBilledOutput
    TokenTracker.cacheRead'              = TokenTracker.cacheRead
    TokenTracker.cacheCreation'          = TokenTracker.cacheCreation
    TokenTracker.reasoningTokens'        = TokenTracker.reasoningTokens
}

/*
 * Trace: at every step, exactly one of the legal transitions occurs.
 */
fact Traces {
    always (
        stutter
        or (some u: ApiTokenUsage, c: Int |
            c >= TokenTracker.outputTokens and updateFromUsage[u, c])
        or resetAfterCompaction
    )
}

// ────────────────────────────────────────────────────────────────────────────
// INVARIANTS — properties to verify
// ────────────────────────────────────────────────────────────────────────────

/*
 * INV-1: total_input() identity (PROV-001).
 *
 * For every ApiTokenUsage, the three input fields sum to total_input().
 * This is by construction — included as a sanity-check assertion.
 */
assert TotalInputIdentity {
    all u: ApiTokenUsage |
        totalInput[u] = plus[plus[u.input, u.cacheRead], u.cacheCreate]
}
check TotalInputIdentity for 5

/*
 * INV-2: cumulative_billed_input is monotonically non-decreasing.
 *
 * Across any execution trace, cumulative billing only grows.
 * Stated by the doc comment: "cumulative_billed_* is NOT reset".
 */
assert CumulativeBilledInputMonotonic {
    always TokenTracker.cumulativeBilledInput' >= TokenTracker.cumulativeBilledInput
}
check CumulativeBilledInputMonotonic for 5 but 10 steps

/*
 * INV-3: cumulative_billed_output is monotonically non-decreasing.
 */
assert CumulativeBilledOutputMonotonic {
    always TokenTracker.cumulativeBilledOutput' >= TokenTracker.cumulativeBilledOutput
}
check CumulativeBilledOutputMonotonic for 5 but 10 steps

/*
 * INV-4: After update_from_usage, input_tokens equals THIS request's
 * total_input — never a sum across requests.
 *
 * This is the "input_tokens is ABSOLUTE" property from the module doc comment.
 */
assert InputTokensAbsolute {
    always (
        all u: ApiTokenUsage, c: Int |
            updateFromUsage[u, c] implies TokenTracker.inputTokens' = totalInput[u]
    )
}
check InputTokensAbsolute for 5 but 10 steps

/*
 * INV-5: After reset_after_compaction, cumulative_billed_input is preserved.
 *
 * Compaction must not zero out cumulative billing — that would lose total
 * session spend information.
 */
assert CompactionPreservesCumulativeBilling {
    always (
        resetAfterCompaction implies (
            TokenTracker.cumulativeBilledInput'  = TokenTracker.cumulativeBilledInput
            and
            TokenTracker.cumulativeBilledOutput' = TokenTracker.cumulativeBilledOutput
        )
    )
}
check CompactionPreservesCumulativeBilling for 5 but 10 steps

/*
 * INV-6: After reset_after_compaction, output_tokens is zero and cache
 * values are cleared.
 */
assert CompactionClearsTransientState {
    always (
        resetAfterCompaction implies (
            TokenTracker.outputTokens' = 0
            and TokenTracker.reasoningTokens' = 0
            and no TokenTracker.cacheRead'
            and no TokenTracker.cacheCreation'
        )
    )
}
check CompactionClearsTransientState for 5 but 10 steps

/*
 * INV-7: cumulativeBilledInput >= sum of fresh-input portions of all
 * updateFromUsage calls so far. By construction in our model this is an
 * equality after every update; we state >= so the property survives any
 * future addition of compaction-triggered "phantom" updates.
 *
 * (In a stronger model this would track the trace explicitly. Here it
 * follows from the monotonicity + accumulator semantics.)
 */
assert CumulativeBilledLowerBound {
    always (
        all u: ApiTokenUsage, c: Int |
            updateFromUsage[u, c] implies
                TokenTracker.cumulativeBilledInput' >=
                plus[TokenTracker.cumulativeBilledInput, u.input]
    )
}
check CumulativeBilledLowerBound for 5 but 10 steps

/*
 * INV-8: input_tokens (current context) is bounded above by
 * cumulative_billed_input + max-cache-tokens-ever-read. Equivalently:
 * input_tokens never exceeds cumulative_billed_input + (any ApiTokenUsage's
 * cacheRead + cacheCreate). Stated more weakly here: input_tokens of the
 * latest update equals total_input of that update — already covered by
 * INV-4 — so we instead assert that input_tokens is non-zero only after at
 * least one updateFromUsage has occurred.
 *
 * Useful as a liveness sanity check: tracker doesn't spontaneously gain
 * tokens.
 */
assert InputTokensRequireUpdate {
    always (
        TokenTracker.inputTokens > 0 implies
        (some u: ApiTokenUsage, c: Int |
            once (updateFromUsage[u, c] and TokenTracker.inputTokens' = totalInput[u]))
    )
}
check InputTokensRequireUpdate for 5 but 10 steps

// ────────────────────────────────────────────────────────────────────────────
// EXAMPLE RUNS — sanity checks that the model is satisfiable
// ────────────────────────────────────────────────────────────────────────────

/*
 * Show a trace where two updates occur, then a compaction reset.
 * If this is unsatisfiable the model itself is broken.
 */
run TwoUpdatesThenCompaction {
    eventually (some u: ApiTokenUsage, c: Int | updateFromUsage[u, c])
    eventually resetAfterCompaction
} for 4 but 8 steps
