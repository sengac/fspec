# RPC-002 — Ink → ratatui Port Research Index

This directory contains research and recommendations for porting the fspec
TypeScript/Ink (React-for-terminal) TUI to a Rust ratatui frontend over a
tarpc dual-transport (embedded + WebSocket) RPC layer.

The research was produced by four parallel investigation agents, each using
the `DeepSearch` tool exclusively. Their synthesized findings are split into
the documents below for ease of reference during the port.

## Documents

| # | File | Topic | Source agent |
|---|---|---|---|
| 01 | [`01-executive-summary.md`](01-executive-summary.md) | One-page TL;DR + go/no-go + headline recommendations | Synthesis |
| 02 | [`02-fspec-capability-inventory.md`](02-fspec-capability-inventory.md) | Complete capability inventory of the existing Ink/React TUI that the ratatui port must preserve | `d2d49672-e846-4f90-9be8-e1ec2bc41119` |
| 03 | [`03-ratatui-ecosystem-survey.md`](03-ratatui-ecosystem-survey.md) | Catalogue of every relevant ratatui-based crate, capability matrix, gaps, recommended composition | `4419f7de-2a81-4534-9ede-227af14279e9` |
| 04 | [`04-codex-architecture-deep-dive.md`](04-codex-architecture-deep-dive.md) | OpenAI Codex CLI ratatui frontend — files, structs, patterns, why it differs from fspec | `815852ae-744c-4e25-8e3a-6ad6b97cf0e0` |
| 05 | [`05-prior-art-comparable-tuis.md`](05-prior-art-comparable-tuis.md) | tenere, oatmeal, Helix, lazygit, gitui, ratatui-org templates — patterns to copy | `71796099-baf8-4dbe-9ffb-867a101403b3` |
| 06 | [`06-mapping-ink-to-ratatui.md`](06-mapping-ink-to-ratatui.md) | Direct construct-by-construct translation guide (useState → struct field, etc.) | Synthesis |
| 07 | [`07-recommended-architecture.md`](07-recommended-architecture.md) | Proposed architecture: Compositor + Component + mpsc<Action> + tokio loop | Synthesis |
| 08 | [`08-virtuallist-port-spec.md`](08-virtuallist-port-spec.md) | Detailed port plan for `VirtualList.tsx` → Rust | Synthesis |
| 09 | [`09-dialog-and-input-priority-port-spec.md`](09-dialog-and-input-priority-port-spec.md) | Detailed port plan for `Dialog.tsx` + InputPriority manager | Synthesis |
| 10 | [`10-multilineinput-and-mouse-port-spec.md`](10-multilineinput-and-mouse-port-spec.md) | Detailed port plan for `MultiLineInput.tsx` + SGR mouse handling | Synthesis |
| 11 | [`11-open-questions-and-risks.md`](11-open-questions-and-risks.md) | Things to resolve before Example Mapping starts | Synthesis |
| 12 | [`12-suggested-work-unit-breakdown.md`](12-suggested-work-unit-breakdown.md) | Proposed child stories under RPC-002 (ordered by dependency) | Synthesis |
| — | [`rpc-002-feasibility.md`](rpc-002-feasibility.md) | (Pre-existing) Original feasibility analysis | Owner-supplied |

## Methodology

Each agent was given a narrow scope and instructed to use `DeepSearch`
exclusively. DeepSearch is a read-only ephemeral sub-agent that itself can
read files, browse the web, and search session history; it returns
synthesized text answers (it summarises rather than dumping raw evidence).

The four agents ran in parallel:

1. **`4419f7de-…`** — Ratatui ecosystem researcher. Crawled crates.io and
   GitHub for every relevant widget/framework crate; built a capability
   matrix and gap analysis vs our two anchor components (`VirtualList`,
   `Dialog`).
2. **`815852ae-…`** — Codex architecture researcher. Cloned and explored
   `openai/codex` to document files, structs, traits, and design choices
   for chat-history rendering, popups, input handling, and layout.
3. **`d2d49672-…`** — fspec capability inventory analyst. Catalogued the
   existing Ink TUI subsystem-by-subsystem (Input priority manager,
   VirtualList, Dialogs, MultiLineInput, mouse protocol, layout) with
   replicate-difficulty ratings.
4. **`71796099-…`** — Prior-art surveyor. Investigated other ratatui-based
   chat/AI/agent TUIs (tenere, oatmeal, aichat, etc.) and complex stateful
   apps (Helix, lazygit, gitui, atuin, ratatui-org templates) for
   transferable patterns.

After all four returned (one had to be coaxed into emitting a partial
report after DeepSearch sub-timeouts), their findings were cross-checked
and synthesised into the recommendation documents (06-12).

## How to use this dossier

* **For RPC-002 epic owner:** read `01-executive-summary.md` and
  `12-suggested-work-unit-breakdown.md`.
* **For Example Mapping facilitators (when this card is unblocked):**
  read `02-fspec-capability-inventory.md` and the relevant port-spec
  document for the slice being mapped.
* **For Rust developers writing the port:** read `06`, `07`, and the
  port-spec for your slice (`08`/`09`/`10`).
* **For dependency-decision discussions:** read `03` and `11`.

## Status

These documents are **research artefacts**, not implementation specs.
RPC-002 has not yet been broken down or Example-Mapped. The documents
deliberately do not prescribe acceptance criteria — that is the job of
the child work units once they are cut.
