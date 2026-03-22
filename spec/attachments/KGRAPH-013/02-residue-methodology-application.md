# Residue Methodology — Application to fspec Learnings Graph

## Overview of the Residue Methodology

The Residue methodology (Aquino-Michaels, 2026) emerged from "Completing Claude's Cycles" — solving the even-case of Knuth's Hamiltonian decomposition problem that Claude Opus 4.6 had left open. The methodology has three layers:

### Layer 1: Structured Exploration Logging
The core prompt prescribes **what to record** (not what to think). Five principles:

1. **Structure the record-keeping, not the reasoning** — Scaffolding for documentation, not for thought
2. **Make failures retrievable** — Each failed exploration records: specific structural reason, eliminated approach class, surviving structure, reformulations
3. **Force periodic synthesis** — Every 5 explorations, scan artifacts for cross-strategy patterns
4. **Bound unproductive grinding** — If Strategy Register unchanged in 5 explorations, diagnose spiral
5. **Preserve session continuity** — At session start, re-read entire log before doing anything

### Layer 2: Complementary Multi-Agent Architecture
Two agents with identical prompts but different model providers:
- **Agent O** (GPT-5.4): Top-down symbolic reasoner → theory, frameworks, proofs
- **Agent C** (Claude Opus 4.6): Bottom-up computational solver → data, examples, verification

Key insight: Same prompt, radically different strategies. Failures were **complementary**.

### Layer 3: Orchestrator-Mediated Cross-Agent Synthesis
Five critical orchestrator decisions:
1. Set up controlled experiment (two agents, same prompt)
2. Direct Agent O to even case after odd success (budget allocation)
3. **Cross-agent data transfer** — Agent C's solutions exported in Agent O's native representation → immediate pattern recognition
4. **Tool transfer** — Agent C's MRV solver given to Agent O → adapted into theory-guided seeded solver
5. Context about external results (calibration)

## Mapping to fspec: How Residue Applies

### What fspec Sessions Are (Structurally)

Each fspec agent session is an **exploration**. The agent:
- Has a goal (work unit)
- Tries approaches (implementation strategies)
- Succeeds, fails, or partially completes
- Generates artifacts (code, tests, configurations)
- Discovers constraints ("this API doesn't support X", "TypeScript won't allow Y")
- Establishes conventions ("always use chalk for CLI output")

Currently, all of this is captured as **raw conversation text** in session history. The Residue methodology says: capture the **structured residue** instead.

### The Strategy Register for Software Development

Adapted for fspec:

#### Eliminated Approach Classes
"Approaches that use `console.log` in production code — eliminated because chalk is required for all CLI output and console.* is ESLint-banned."

"Approaches that rely on dynamic imports for tool loading — eliminated because bundled dist doesn't resolve dynamic paths correctly."

#### Active Structural Constraints
"Lance dataset versions accumulate per-load call — batch loading required for any nanograph integration."

"The Fspec tool MUST NOT run CLI commands via Bash — it has a dedicated NAPI handler."

#### Known Reformulations
"Per-turn extraction → session-boundary extraction — reduces volume by 100x while capturing higher-value insights."

"Single monolithic graph → dual-graph (AST + Learnings) — separates static code structure from accumulated knowledge."

### Extraction Triggers (Not Per-Turn)

The Residue prompt says: "After every **substantive attempt** — not after every keystroke." For fspec:

| Trigger | What to Extract | Analogy |
|---------|----------------|---------|
| Session end/compaction | Learnings from the DAG summary | Exploration log entry |
| Work unit → done | Full retrospective of the work unit | Synthesis entry |
| Explicit `index` command | User-directed extraction | "Read your notes" |
| Scheduled (every N sessions) | Cross-session patterns | Periodic synthesis |

### The Multi-Agent Dimension

fspec already has multi-agent capabilities:
- **AgentManager**: Spawns subordinate agents
- **DeepSearch**: Ephemeral research sub-agents
- **SessionSearch**: Cross-session history access

The Learnings graph enhances all three:

1. **AgentManager spawn**: Inject relevant learnings into subordinate's context
   - "Previous sessions established convention X for this module"
   - "Approach Y was tried and failed because Z"

2. **DeepSearch context**: Add learnings alongside code files
   - "Here are the known constraints for this area"
   - "Here are the established conventions"

3. **Session continuity**: At session start, surface relevant decisions/constraints
   - Replaces the current "index all turns" approach
   - Much more targeted: only inject what's relevant to the current work unit

### Cross-Agent Data Transfer (The Key Innovation)

The Residue paper's most important insight: **cross-agent data transfer at the right moment, in the right representation, is more valuable than any amount of parallel independent work.**

For fspec, this means:
1. When a subordinate discovers a constraint → record in Learnings graph immediately
2. When the supervisor's next subordinate starts → inject that constraint
3. The graph is the **transfer medium** — it normalizes representations across sessions

### Periodic Synthesis as a Scheduled Job

The Residue prompt mandates synthesis every 5 explorations. For fspec:

```
Schedule: every 10 completed sessions (or weekly, whichever is first)
Job type: agent
Role: "You are a knowledge synthesis agent. Review recent session summaries and the current Learnings graph."
Prompt: "
1. Scan Exploration nodes from the last 10 sessions for cross-strategy patterns
2. Check if any Reformulation suggests an approach not yet tried
3. Look for Contradictions between recent Learnings and established Conventions
4. Update the Strategy Register (eliminated approaches, active constraints, reformulations)
5. Write a synthesis node in the graph
"
```

## Volume & Cost Comparison

### Old System (KGRAPH-002 through KGRAPH-012)
- **Extraction**: Per-turn structural + batch LLM scan of ALL turns
- **Volume**: ~100-500 entities per session (Turn nodes, Mentions edges, etc.)
- **LLM cost**: Full session replay for batch extraction
- **Storage**: 7.6GB after 727 turns
- **Value**: "Which turn mentioned concept X?" (low — SessionSearch does this better)

### New Learnings Graph
- **Extraction**: Session-boundary LLM analysis of DAG summary only
- **Volume**: ~5-20 entities per session
- **LLM cost**: ~1 extraction call per session (analyzing the summary, not every turn)
- **Storage**: Estimated <5MB for 1000 sessions
- **Value**: "What was decided about X and why?", "What approaches failed for Y?" (high — unique to graph)

### Cost Reduction
- **50-100x fewer entities** per session
- **10-50x less LLM extraction cost** (summary vs full turns)
- **1500x less storage** (5MB vs 7.6GB)
- **Higher information density** (learnings vs raw mentions)

## The "Residue" Insight Applied

> "The solution may be in the residue of your previous failures."

For fspec, this means: **don't throw away failure information**. The old KGRAPH system treated all conversation equally. The new system specifically values:

1. **Why things failed** (not just what succeeded)
2. **What class of approaches was eliminated** (not just the specific attempt)
3. **What survived from failed attempts** (partial results, intermediate discoveries)
4. **What reformulations emerged** (new ways to think about the problem)

This is the fundamental shift: from **indexing everything** to **curating the valuable residue**.
