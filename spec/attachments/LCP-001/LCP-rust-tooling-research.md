# LCP Rust Tooling Research — Existing Crates & Infrastructure

## Executive Summary

fspec's Rust codebase (`codelet/`) already contains **most of the infrastructure needed** for LCP. The key finding is that **no new dependencies are required** — LCP can be built entirely on top of existing crates already in the workspace.

---

## 1. What Already Exists in codelet/ (Reuse Map)

| LCP Need | Existing Infrastructure | Crate(s) | Location |
|----------|----------------------|-----------|----------|
| **Parse @LCP from comments** | ast-grep-core + 25 tree-sitter grammar crates (23 supported languages) already bundled | `ast-grep-core 0.40`, `ast-grep-language 0.40` | `codelet-tools/src/astgrep.rs` |
| **Walk codebase for LCP files** | gitignore-aware file walker already used by Grep/AstGrep/Glob tools | `ignore 0.4` | `codelet-tools/src/glob.rs`, `grep.rs` |
| **Glob-match risk zone paths** | Compiled glob matching with negation (`!` prefix) already used by stage permissions | `globset 0.4` | `codelet-tools/src/stage_permissions/matcher.rs` |
| **Block tool writes on LCP files** | Pre-tool-hook system — per-session decision callbacks on every tool call | Custom (`pre_tool_hook.rs`) | `codelet-tools/src/pre_tool_hook.rs` |
| **ACDD stage check pattern** | Stage permissions global singleton with config hierarchy | Custom (`stage_permissions/`) | `codelet-tools/src/stage_permissions/` |
| **Regex-based pattern matching** | Blocklist system — compiled regex with first-match-wins | `regex 1` | `codelet-tools/src/blocklist/matcher.rs` |
| **Git diff (changed files)** | Pure-Rust gitoxide with status APIs | `gix 0.72` | `codelet-git/src/status.rs` |
| **Run verification commands** | Hook execution engine — process spawning with timeout, exit code parsing | Custom (lifecycle hooks engine) | `codelet-core/src/lifecycle_hooks/engine.rs` |
| **JSON config with schema** | Ajv-style validation via schemars + serde | `serde 1`, `serde_json 1`, `schemars 0.8` | Used everywhere |
| **Config merge (system + project)** | Two-level config hierarchy pattern already established | Custom pattern | `stage_permissions/mod.rs`, `blocklist/middleware.rs` |
| **NAPI bridge to TypeScript** | Full NAPI-RS binding infrastructure | `napi 3`, `napi-derive 3` | `codelet-napi/` (165+ function bindings) |
| **File type from extension** | Extension → type detection | Custom (`file_type.rs`) | `codelet-tools/src/file_type.rs` |

---

## 2. Three Approaches for @LCP Comment Extraction

### Approach A: Tree-Sitter Queries (via existing ast-grep-core)

**Already in the codebase.** Every tree-sitter grammar exposes `comment` and `block_comment` (or language-equivalent) node types. You can query them directly.

**How it works:**
```rust
use ast_grep_core::AstGrep;
use ast_grep_language::SupportLang;

let lang = SupportLang::from_str("typescript")?;
let root = lang.ast_grep(&source_code);

// Tree-sitter query for comment nodes
// TypeScript: (comment) @c captures // and /* */ comments
// Python: (comment) @c captures # comments
// Rust: (line_comment) @c, (block_comment) @c
for node in root.root().children() {
    if node.kind() == "comment" || node.kind() == "block_comment" || node.kind() == "line_comment" {
        let text = node.text();
        if text.contains("@LCP") {
            // Parse the structured LCP block from comment text
        }
    }
}
```

**Pros:**
- Zero new dependencies — uses `ast-grep-core` + `ast-grep-language` already at version 0.40
- AST-accurate — won't match `@LCP` inside string literals or nested comments
- Handles all 23 explicitly supported languages already bundled via tree-sitter grammars (25 grammar crates)
- Can also extract the exact source location (line, column) of the LCP block

**Cons:**
- Parsing the full AST is heavier than needed for just comment extraction
- Comment node type names vary by grammar (`comment`, `line_comment`, `block_comment`, `string_content` for Python docstrings)
- ast-grep wraps tree-sitter in panic-catching (`catch_unwind`) which adds overhead

### Approach B: `comment-parser` Crate (Lightweight, Purpose-Built)

**New dependency — but tiny.** The [`comment-parser`](https://crates.io/crates/comment-parser) crate (v0.1) is specifically designed for extracting comments from source code across languages.

**How it works:**
```rust
use comment_parser::CommentParser;

let rules = comment_parser::get_syntax("typescript").unwrap();
// Also: "python", "rust", "ruby", "go", "java", "c", "cpp"
let parser = CommentParser::new(&source_code, rules);

for comment in parser {
    match comment {
        Event::BlockComment(range, text) | Event::LineComment(range, text) => {
            if text.contains("@LCP") {
                // Parse the structured LCP block
                // `range` gives byte offset in source
            }
        }
        _ => {}
    }
}
```

**Supported languages** (via `get_syntax()`): C, C++, C#, CSS, Go, HTML, Java, JavaScript, Kotlin, Less, Objective-C, PHP, Python, Ruby, Rust, Sass, SCSS, Shell, SQL, Swift, TypeScript, XML, YAML.

**Pros:**
- Purpose-built for exactly this use case — extracting comments from code
- Tiny dependency footprint (depends only on `detect-lang 0.1` and `line-span 0.1`)
- Fast — simple state machine parser, no AST construction
- Language detection from file extension via `get_syntax_from_path()`
- Returns `BlockComment` and `LineComment` variants with byte ranges

**Cons:**
- New dependency (but extremely small)
- No AST awareness — would match `@LCP` in a string literal like `"// @LCP"` (unlikely in practice)
- v0.1 — stable but not actively developed
- No line number tracking (only byte ranges, but these are easily converted)

### Approach C: Regex Line Scanner (Simplest, Like Existing @step Pattern)

**Zero dependencies.** The existing `@step` comment detection in TypeScript is pure regex line scanning. The Rust equivalent would be:

```rust
use regex::Regex;

lazy_static! {
    static ref LCP_START: Regex = Regex::new(r"(?://|/\*\*?|#|///|--)\s*@LCP\s*$").unwrap();
    static ref LCP_FIELD: Regex = Regex::new(
        r"(?://|[*#]|///|--)\s*(scope|risk_level|constraints|requires_human_review|verification|do_not_change):\s*(.*)"
    ).unwrap();
}

for (line_num, line) in source.lines().enumerate() {
    if LCP_START.is_match(line.trim()) {
        // Start parsing LCP block
        // Continue reading subsequent comment lines for fields
    }
}
```

**Pros:**
- Zero new dependencies — `regex` already used everywhere in codelet
- Exact same pattern as TypeScript `@step` detection
- Extremely fast — no parsing overhead
- Easy to understand and maintain

**Cons:**
- Fragile — could match `@LCP` in string literals, docstrings, or non-comment contexts
- Must handle every comment syntax variant manually
- Can't distinguish between `// @LCP` inside actual code vs inside a string
- Language-specific rules get messy

### Recommendation: Approach A (tree-sitter) for correctness, with Approach C as fallback

**Primary:** Use tree-sitter via `ast-grep-core` (already in workspace) to find comment nodes, then regex-parse the @LCP fields from comment text. This gives AST-correct comment detection with zero new dependencies.

**Fallback:** For languages not in the tree-sitter grammar bundle, fall back to regex line scanning (Approach C).

**Skip Approach B:** The `comment-parser` crate adds a dependency for marginal benefit over what tree-sitter already provides. If the goal is minimal dependencies, tree-sitter is already there. If the goal is simplicity, regex is simpler.

---

## 3. Git Diff Detection — Changed File Discovery

fspec already has `gix 0.72` (gitoxide) in `codelet-git/`. The existing git module provides:

- `gix::open(path)` — open a repository
- `repo.status()` — get working directory status (staged, unstaged, untracked)
- `repo.diff_tree_to_tree()` — diff between commits
- `repo.head_commit()` — get HEAD for comparison

**For LCP's `verify-lcp` command** (run verification only on changed LCP files):

```rust
// Reuse existing APIs from codelet-git/src/status.rs:
use codelet_git::status::{get_staged_files, get_unstaged_files};

fn get_changed_lcp_files(repo_path: &Path) -> Result<Vec<String>> {
    let mut changed = get_staged_files(repo_path)?;
    changed.extend(get_unstaged_files(repo_path)?);
    changed.sort();
    changed.dedup();
    Ok(changed)
}
```

**What already exists in `codelet-git/`:**
- `get_staged_files()` — files added to the index (staging area)
- `get_unstaged_files()` — modified tracked files not yet staged
- `get_untracked_files()` — new files not tracked by git
- `ghost_commit()` — creates commits without touching working directory (for checkpoints)
- `get_checkpoint_diff_files()` — files changed since a checkpoint
- `get_session_diff()` — unified diff generation via `similar` crate

**Key insight:** The `codelet-git` crate already has `get_staged_files()` and `get_unstaged_files()` which together cover what LCP-005 needs. No new git integration required.

---

## 4. Risk Zone Matching — Glob Infrastructure

The `stage_permissions` module is the **exact pattern** LCP risk zones should follow:

```rust
// Already exists in stage_permissions/matcher.rs (crate-private):
struct CompiledCategory {
    name: String,
    include: GlobSet,  // from globset crate
    exclude: GlobSet,  // patterns prefixed with !
}

impl CompiledCategory {
    pub fn matches(&self, path: &str) -> bool {
        self.include.is_match(path) && !self.exclude.is_match(path)
    }
}
```

**For LCP risk zones:**
```rust
pub struct RiskZone {
    level: RiskLevel,  // High, Medium, Low
    include: GlobSet,
    exclude: GlobSet,
    tags: Vec<String>,
}

pub struct RiskZoneMatcher {
    zones: Vec<RiskZone>,
}

impl RiskZoneMatcher {
    pub fn classify(&self, path: &str) -> Option<RiskLevel> {
        // First match wins (high → medium → low priority)
        for zone in &self.zones {
            if zone.include.is_match(path) && !zone.exclude.is_match(path) {
                return Some(zone.level);
            }
        }
        None
    }
}
```

This is literally copy-adapt from `stage_permissions/matcher.rs`.

---

## 5. Tool-Call Enforcement — Pre-Tool Hook Pattern

The `pre_tool_hook.rs` system is the enforcement mechanism:

```rust
// Already exists:
pub enum PreToolHookDecision {
    Allow,
    Deny(String),  // reason
    Continue,      // no opinion
}

// Every tool calls this at start of call():
pub fn pre_tool_hook_check(session_id: Uuid, tool_name: &str, args: &Value) -> PreToolHookDecision
```

**For LCP enforcement**, the NAPI layer would register a handler that:
1. Checks if tool is `Write`, `Edit`, or `ApplyPatch`
2. Extracts the target file path from `args`
3. Checks if file has an `@LCP` block (via registry lookup)
4. If high-risk LCP with `do_not_change` constraints → check if the proposed change touches constrained regions
5. Return `Deny(reason)` if constraints would be violated, `Continue` otherwise

This follows the exact same pattern as `stage_permissions` and `blocklist`.

---

## 6. Verification Command Execution — Hook Engine Reuse

The lifecycle hook engine in `codelet-core/src/lifecycle_hooks/engine.rs` already has:

```rust
async fn execute_command(
    command: &str,
    timeout_secs: u64,
    env: &HashMap<String, String>,
    stdin_payload: Option<&str>,
) -> Result<HookOutput> {
    // Spawns via sh -c
    // Pipes JSON to stdin
    // Handles timeout (SIGKILL)
    // Parses exit code + stdout/stderr
}
```

**For `verify-lcp`:** Reuse this pattern to execute verification commands from `@LCP` blocks. The commands are shell strings (e.g., `npm run test:payments`), same as hook commands.

---

## 7. Config File Pattern — System + Project Merge

Both `blocklist` and `stage_permissions` use the same config hierarchy:

```
~/.fspec/<config>.json     (system-level defaults)
.fspec/<config>.json       (project-level overrides)
spec/<config>.json         (spec-level, where applicable)
```

With a `merge()` function that combines both, project taking precedence.

**For `lcp-registry.json`:** Follow this pattern exactly. The risk zones section would typically be project-level only (in `spec/lcp-registry.json`), while the protected files cache is generated by scanning.

---

## 8. NAPI Bridge — Exposing to TypeScript

All of the above Rust functionality needs to be exposed to the TypeScript fspec CLI. The existing `codelet-napi/` crate shows the pattern:

```rust
#[napi]
pub fn scan_lcp(project_path: String) -> napi::Result<String> {
    // Walk codebase, find @LCP blocks, return JSON
}

#[napi]
pub fn check_lcp_risk_zone(file_path: String, project_path: String) -> napi::Result<String> {
    // Classify file into risk zone, return JSON
}

#[napi]
pub fn validate_lcp(project_path: String) -> napi::Result<String> {
    // Validate all @LCP blocks, return JSON report
}
```

The existing crate already has 165+ NAPI function bindings for tools, git, sessions, etc.

---

## 9. Summary: Build vs Buy Decision

| Component | Decision | Rationale |
|-----------|----------|-----------|
| **Comment extraction** | BUILD on `ast-grep-core` (existing) | Already bundled, AST-correct, 23 languages |
| **File walking** | REUSE `ignore::WalkBuilder` (existing) | Same pattern as Grep/AstGrep/Glob tools |
| **Glob matching** | REUSE `globset` (existing) | Copy-adapt from `stage_permissions/matcher.rs` |
| **Risk zone classification** | BUILD following `stage_permissions` pattern | Near-identical architecture |
| **Tool-call blocking** | BUILD following `pre_tool_hook` + `blocklist` pattern | Established interception points |
| **Git changed files** | REUSE `codelet-git` status APIs (existing) | `get_staged_files()` + `get_unstaged_files()` via `gix` |
| **Verification execution** | REUSE lifecycle hook engine pattern (existing) | Process spawning + timeout already solved |
| **JSON config + schema** | REUSE `serde` + `schemars` (existing) | Standard approach throughout codebase |
| **Config hierarchy** | REUSE system+project merge pattern (existing) | Established in blocklist + stage_permissions |
| **NAPI bindings** | BUILD in `codelet-napi` (existing crate) | Add new `#[napi]` functions alongside 165+ existing ones |
| **`comment-parser` crate** | SKIP | Tree-sitter already covers this with better accuracy |
| **`rust-code-analysis` crate** | SKIP | Overkill — provides metrics, not comment extraction |

**Net new Rust dependencies: ZERO.**

**Net new Rust code: ~1000-1500 lines** estimated across:
- `codelet-tools/src/lcp/` — new module (parser, matcher, config, mod)
- `codelet-napi/src/lcp.rs` — NAPI bindings
- Integration into existing `pre_tool_hook` handler registration

---

## 10. External Crates Evaluated and Rejected

| Crate | What It Does | Why Rejected |
|-------|-------------|--------------|
| **`comment-parser 0.1`** | Extract comments from source code across languages | Tree-sitter (already bundled) provides same capability with AST correctness |
| **`rust-code-analysis 0.0.25`** | Mozilla's code metrics library (tree-sitter based) | Overkill — provides complexity metrics, not comment extraction. Also brings its own tree-sitter grammars which would conflict with ast-grep-language's |
| **`tree-parser`** | Multi-language code parsing | Overlaps with ast-grep-core which is already integrated |
| **`syn`** | Rust source code parser | Rust-only — not multi-language |
| **`tree-sitter` (direct)** | Parser generator library | Already available transitively via `ast-grep-language` — no need for direct dependency |
| **`tree-sitter-highlight`** | Syntax highlighting | Intentionally removed from the codebase (verified by test) |

---

## 11. Architecture Recommendation

```
codelet-tools/src/lcp/
├── mod.rs              # Public API + global LCP registry singleton
├── config.rs           # LcpRegistryConfig, RiskZone, ProtectedFile structs
├── parser.rs           # @LCP comment block parser (tree-sitter → regex hybrid)
├── matcher.rs          # RiskZoneMatcher (globset) + LcpConstraintChecker
└── scanner.rs          # Codebase walker (ignore::WalkBuilder + parser)

codelet-napi/src/lcp.rs  # NAPI bindings: scan_lcp, validate_lcp, check_risk_zone, etc.
```

This mirrors the exact structure of `stage_permissions/` and `blocklist/`, making it immediately familiar to anyone working in the codebase.
