# RPC-072 — RPC-030 Plan Diff & Supersession Notes

> Why RPC-030 is being closed as superseded, and what its intended deliverables
> become inside RPC-072.

---

## 1. RPC-030's Intent

**Title:** "Wire BackgroundSession + agent management (/provider, /providers,
/model) into the Rust AgentView via the SessionManagerHandle trait — NAPI-free
RPC boundary audit + plan"

**Status:** `backlog` since creation (mid-2025).

**Acceptance summary (paraphrased):**

1. Audit every NAPI dependency the Rust agent loop would need.
2. Produce a written plan for a NAPI-free hooks impl.
3. Identify which crate owns the new hooks.
4. Identify provider/model resolution sources.
5. Identify tool injection strategy.

In short, RPC-030 was a **deliverable: a plan**. No code.

---

## 2. Why RPC-030 Was Never Started

Cards RPC-031 through RPC-067 all proceeded to **build** on top of the
assumed plan — the new types, the AgentView, the SessionFooter, the
slash commands, the dialogs, the cross-transport parity tests. Each of
those cards individually delivered a chunk of the puzzle.

But the **central deliverable** — the actual non-NAPI agent loop —
never landed. RPC-030 sat in backlog because:

1. By the time anyone looked at it, RPC-031..RPC-067 had already
   shipped the surface area that depended on it.
2. The hooks abstraction (RPC-040's `SessionManagerHooks` trait) made
   it possible to ship those surface cards even without the impl —
   they just compiled against the `NoopSessionManagerHooks` default.
3. The test suite was structured around unit tests that mocked the
   agent loop, so the missing impl wasn't visible in CI.
4. The cross-frontend integration test (RPC-066) that WOULD have
   surfaced the gap was deferred under RPC-069 (stub provider routing).

The result was a system that compiled, passed its tests, and ran a
visual frontend — but whose **headline feature** (talking to an LLM)
silently no-op'd.

---

## 3. Mapping: RPC-030 Plan → RPC-072 Deliverables

| RPC-030 deliverable | Lives in RPC-072 as |
|---------------------|---------------------|
| NAPI dependency audit | `root-cause-analysis.md` §2, §5 |
| Hooks impl ownership | `architecture.md` §2 + `implementation-plan.md` P1 |
| Provider resolution plan | `architecture.md` §7 + `implementation-plan.md` P5 |
| Tool injection plan | `architecture.md` §3 + `implementation-plan.md` P2 |
| `/provider`, `/providers`, `/model` end-to-end | `implementation-plan.md` P5 |
| Boundary regression tests | `test-plan.md` §3 |

Plus the actual code, which RPC-030 didn't deliver.

---

## 4. Closing RPC-030

Once RPC-072 lands:

```
fspec update-work-unit-status RPC-030 done
fspec add-architecture-note RPC-030 \
  "Superseded by RPC-072 which delivered both the plan AND the impl. \
   See spec/attachments/RPC-072/ for the plan; see codelet/agent-loop/ \
   for the impl."
fspec add-dependency RPC-072 RPC-030 --relatesTo
```

Or alternatively close as a duplicate/rejected:

```
fspec update-work-unit RPC-030 --description \
  "Superseded by RPC-072 which combined the planning deliverable with the \
   implementation deliverable. Closing without separate execution."
fspec update-work-unit-status RPC-030 done
```

The second approach is preferred — RPC-030's planning content has been
absorbed into RPC-072's attachment set, so RPC-030 has no remaining
unique deliverable.

---

## 5. Lessons for Future Cards

1. **Planning-only cards are an anti-pattern when the plan is for a
   user-visible feature.** Combine plan + impl into a single card with
   the plan as an attachment.

2. **Hooks/trait abstractions enable safe deferred implementation, but
   the no-op impl needs an automated "is this still no-op?" check.**
   Add a CI assertion: "if FspecAgentHooks is still NoopSessionManager
   Hooks in production builds, fail the build" — perhaps via a marker
   const or a `#[cfg(not(...))]` compile guard.

3. **Cross-frontend integration tests should NOT be `#[ignore]`'d.**
   RPC-066's existence with all four tests ignored was the early
   warning sign that the binary had no real end-to-end coverage.

4. **The screenshot evidence loop matters.** This entire investigation
   started because the user took a screenshot of the broken behaviour
   and asked which card owned the bug. Without that screenshot, the
   bug could have lived in the codebase indefinitely. **Encourage
   manual binary smoke tests on every release candidate.**
