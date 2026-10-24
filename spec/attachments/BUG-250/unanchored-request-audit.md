# Valid requests without promoted assistant originals need causal anchors

Independent review of the new canonical archive merge found a normal lifecycle
case that its fail-closed unanchored-interior-request policy does not complete.
A provider can fail before producing any assistant content. Its durable terminal
record then has no promoted assistant message identity in the manifest. After
another request is admitted, the earlier valid request becomes an interior
request without an original anchor and the current archive merge rejects it.
The same occurs with multiple journal-only requests. Such a failure is preferable
to guessed ordering but does not satisfy faithful replay of valid failures.

Add feature-first regressions for two consecutive requests that fail before
assistant output, plus a subsequent successful request. Repeated cold display
and canonical paged export must retain every typed outcome exactly once in
causal order, without timestamp sorting or injecting notices into model replay.

The root contract needs an explicit durable original-prefix/request anchor for
new runtime requests, captured before publication or dispatch. A journal event
can carry this metadata without replacing originals or introducing a second
archive. Existing malformed or genuinely unanchored historical data must remain
an explicit compatibility failure; do not invent original coordinates.
Coordinate the agent-loop writer, journal projection, canonical archive merger
and tests before changing the versioned contract. This is within the approved
incremental-journal architecture, not permission for unrelated schema changes.

This finding is currently source-derived, not a claim of executable RED or GREEN.
