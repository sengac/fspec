# Research Papers — Context Compaction for LLM Agents

Papers reviewed for CMPCT-005 architecture design. Ordered by relevance.

---

## 1. CMV — Contextual Memory Virtualisation: DAG-Based State Management and Structurally Lossless Trimming for LLM Agents

- **Authors:** Cosmo Santoni
- **Affiliation:** Imperial College London
- **Date:** February 2026
- **ArXiv:** https://arxiv.org/abs/2602.22402
- **Code:** https://github.com/CosmoNaught/claude-code-cmv

**Key contributions:**
- Models session history as a DAG (Directed Acyclic Graph) with snapshot, branch, and trim primitives
- Three-pass structurally lossless trimming algorithm: preserves every user message and assistant response verbatim, strips raw tool outputs, base64 images, and metadata
- Mean 20% token reduction, up to 86% for sessions with significant tool output overhead
- Trimming remains economically viable under prompt caching — break-even within 10 turns for mixed tool-use sessions (39% average reduction)
- Agent-agnostic architecture applicable to any system using tool-call schemas

**Relevance to CMPCT-005:** Primary inspiration for Layer 0 (structurally lossless trimming). Demonstrates the highest-ROI intervention is stripping mechanical bloat BEFORE any lossy compression. In our design, trimming is applied to SessionSearch results during DAG construction, so the agent sees condensed content without consuming excess context budget during the rebuild phase.

---

## 2. LCM — Lossless Context Management

- **Authors:** Clint Ehrlich, Andrew Blackman
- **Affiliation:** Voltropy PBC
- **Date:** February 2026
- **Paper:** https://papers.voltropy.com/LCM
- **Code:** https://github.com/martian-engineering/lossless-claw (OpenClaw plugin), https://github.com/martian-engineering/volt (coding agent)
- **Explainer:** https://www.losslesscontext.ai/

**Key contributions:**
- Hierarchical summary DAG with depth-aware prompts (D0: minutes/details, D1: hours/arcs, D2: days/narrative)
- Incremental compaction: only messages outside the "fresh tail" are eligible for summarization
- Condensation: when enough D0 summaries accumulate, they are synthesized into D1 nodes; D1 into D2, etc.
- DAG navigation tools: `lcm_describe` (inspect subtree), `lcm_grep` (search across all depths), `lcm_expand_query` (bounded sub-agent expansion with token budget)
- Sub-agent expansion: spawns a bounded sub-agent to strategically traverse the DAG and retrieve specific details
- Benchmarked: LCM-augmented "Volt" agent outperforms Claude Code on OOLONG long-context evaluation from 32K to 1M tokens

**Relevance to CMPCT-005:** Inspired the hierarchical D0/D1/D2 structure of the agent-written DAG summary. In our design, instead of separate LLM calls building D0/D1/D2 nodes through an automated pipeline, the agent writes the entire hierarchical summary itself during in-view DAG construction. The DAG navigation tools (`lcm_grep`, `lcm_expand_query`) are replaced by `SessionSearch`, which already provides regex search and targeted retrieval across persisted history.

---

## 3. Focus — Active Context Compression: Autonomous Memory Management in LLM Agents

- **Authors:** Nikhil Verma
- **Date:** January 2026
- **ArXiv:** https://arxiv.org/abs/2601.07190

**Key contributions:**
- Agent-centric architecture inspired by Physarum polycephalum (slime mold) exploration strategy
- Two primitives: `start_focus` (declare investigation) and `complete_focus` (consolidate learnings, prune raw logs)
- "Sawtooth" context pattern: context grows during exploration, collapses during consolidation
- Agent has FULL AUTONOMY over when to compress — no external timers or heuristics
- Key insight: "Biological systems do not retain a perfect record of every muscle movement used to navigate a maze; they retain the learned map"
- Evaluated on SWE-bench Lite: 22.7% token reduction (14.9M → 11.5M) with identical accuracy (3/5 = 60%)
- 6.0 autonomous compressions per task on average, up to 57% savings on individual instances

**Relevance to CMPCT-005:** Core validation for agent-controlled compression. Our design takes this further: instead of Focus's two primitives (start/complete), the agent performs the entire summarization itself. The agent doesn't just signal "I'm done with this block" — it writes the summary of what was learned. This eliminates the separate summarization LLM call that Focus still requires.

---

## 4. ACON — Agent Context Optimization

- **Authors:** Minki Kang, Wei-Ning Chen, Dongge Han, Huseyin A. Inan, Lukas Wutschitz, Yanzhi Chen, Robert Sim, Saravan Rajmohan
- **Affiliation:** KAIST, Microsoft, University of Cambridge
- **Date:** October 2025
- **ArXiv:** https://arxiv.org/abs/2510.00615

**Key contributions:**
- Unified framework for compressing both environment observations and interaction histories
- Compression guideline optimization: given paired trajectories (full context succeeds, compressed fails), an LLM analyzes failure causes and updates the compression guideline
- Gradient-free — works with closed-source production models
- Distillation: optimized LLM compressor distilled into smaller models (95% accuracy preserved)
- Results: 26–54% peak token reduction while maintaining task performance; smaller LMs improve by up to 46% when using compressed context (less distraction)
- Benchmarked on AppWorld, OfficeBench, Multi-objective QA (15+ interaction steps each)

**Relevance to CMPCT-005:** Validates that LLM-based compression with proper guidelines preserves quality. In our design, the "compression guideline" is embedded in the system instruction that guides the agent during DAG construction. ACON's failure-analysis approach could be applied to iteratively improve this system instruction over time.

---

## 5. HiAgent — Hierarchical Working Memory Management for Solving Long-Horizon Agent Tasks

- **Authors:** Mengkang Hu, Tianxing Chen, Qiguang Chen, Yao Mu, Wenqi Shao, Ping Luo
- **Venue:** ACL 2025 (long paper)
- **URL:** https://aclanthology.org/2025.acl-long.1575/
- **Code:** https://github.com/HiAgent2024/HiAgent

**Key contributions:**
- Uses subgoals as natural memory chunk boundaries
- Agent proactively replaces previous subgoal blocks with summarized observations
- Retains only action-observation pairs relevant to the CURRENT subgoal in working memory
- Twofold increase in success rate across five long-horizon tasks
- 3.8 fewer steps required on average
- Key insight: letting the agent decide subgoal boundaries produces better compression than external heuristics

**Relevance to CMPCT-005:** Validates that agent-controlled compression boundaries outperform external heuristics. In our design, the agent decides what to include at each DAG depth level based on its understanding of subgoal boundaries — which work is complete, which is in progress, and which decisions are still relevant. The 2× success rate improvement from HiAgent strongly supports giving the agent control over its own memory.

---

## 6. SimpleMem — Efficient Lifelong Memory for LLM Agents

- **Authors:** Jiaqi Liu, Yaofeng Su, Peng Xia, Siwei Han, Zeyu Zheng, Cihang Xie, Mingyu Ding, Huaxiu Yao
- **Venue:** ICLR 2026 Workshop LLA Poster / Workshop MemAgents / Workshop RSI Spotlight (March 2026)
- **OpenReview:** https://openreview.net/forum?id=dbZAi4hmwg
- **ArXiv:** https://arxiv.org/abs/2601.02553
- **Code:** https://github.com/aiming-lab/SimpleMem

**Key contributions:**
- Three-stage pipeline: (1) Semantic Structured Compression — distills interactions into compact multi-view indexed memory units; (2) Online Semantic Synthesis — instantly integrates related context to eliminate redundancy during write phase; (3) Intent-Aware Retrieval Planning — infers search intent to determine retrieval scope
- Semantic density gating: LLM judges information gain relative to history, preserving only high-utility content
- Multi-index retrieval: dense semantic embeddings + sparse lexical features + symbolic metadata
- Results: 26.4% F1 improvement over baselines (including Mem0), 30× token reduction vs full-context models
- Inspired by Complementary Learning Systems (CLS) theory from neuroscience

**Relevance to CMPCT-005:** Validates semantic compression with density gating. In our design, the agent performs implicit semantic density gating when writing its DAG — it naturally prioritizes high-information-density content (decisions, errors, architecture) over low-density content (routine tool outputs, unchanged file reads). The multi-index retrieval concept is partially realized through SessionSearch's regex + time-filter capabilities.

---

## 7. H-MEM — Hierarchical Memory for High-Efficiency Long-Term Reasoning in LLM Agents

- **Authors:** Haoran Sun, Shaoning Zeng
- **Date:** July 2025
- **ArXiv:** https://arxiv.org/abs/2507.22925

**Key contributions:**
- Multi-level memory organization based on degree of semantic abstraction
- Positional index encoding: each memory vector is embedded with an index pointing to semantically related sub-memories in the next layer
- Index-based routing: efficient layer-by-layer retrieval WITHOUT exhaustive similarity computations
- Consistently outperforms five baseline methods on LoCoMo dataset (long-term dialogue)
- Key insight: hierarchical organization is a NECESSARY CONDITION for robust long-term reasoning, not optional

**Relevance to CMPCT-005:** Validates the necessity of hierarchical structure over flat memory. In our design, the agent writes its DAG with explicit depth levels (D0/D1/D2) and includes turn range references on each node — these serve the same role as H-MEM's positional indices, enabling targeted drilldown via SessionSearch to the specific turns that a DAG node summarizes.

---

## Survey Paper

## Memory in the Age of AI Agents: A Survey

- **Authors:** Yuyang Hu, Shichun Liu, et al. (47 authors total)
- **Date:** December 2025 (v1), January 2026 (v2)
- **ArXiv:** https://arxiv.org/abs/2512.13564
- **Paper List:** https://github.com/Shichun-Liu/Agent-Memory-Paper-List

Comprehensive survey that distinguishes agent memory from LLM memory, RAG, and context engineering. Provides taxonomy of: memory formation (encoding), management (storage/organization), and utilization (retrieval/application). Positions our work in the "intra-session working memory management" category within the broader agent memory landscape.
