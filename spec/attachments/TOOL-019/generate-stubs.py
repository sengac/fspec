#!/usr/bin/env python3
"""
TOOL-019 / Worker C: bulk-generate the remaining 161 stub files,
update canonical.rs with real RPC-XXX work-unit IDs, and rewrite
commands/mod.rs + the dispatch match arm to cover all 162 commands.

This script is idempotent — running it twice produces the same output.
"""
import json
import re
from pathlib import Path

ROOT = Path("/Users/rquast/projects/fspec")
CORE = ROOT / "codelet" / "fspec-core"
CANONICAL_JSON = ROOT / "spec/attachments/TOOL-019/canonical-commands.json"
MAPPING_JSON = ROOT / "spec/attachments/TOOL-019/command-to-rpc-mapping.json"

canonical = json.loads(CANONICAL_JSON.read_text())["commands"]
mapping = json.loads(MAPPING_JSON.read_text())["mapping"]
assert len(canonical) == 162, f"expected 162 commands, got {len(canonical)}"
assert len(mapping) == 162, f"expected 162 mappings, got {len(mapping)}"


def snake(name: str) -> str:
    return name.replace("-", "_")


# ---- 1. canonical.rs: replace RPC-PENDING with actual RPC IDs ----
canonical_rs = CORE / "src/canonical.rs"
text = canonical_rs.read_text()

for entry in canonical:
    cmd = entry["name"]
    rpc = mapping[cmd]
    # Each line for this command: name: "cmd", ts_file: "...", work_unit: "RPC-PENDING"
    pattern = re.compile(
        r'(name: "' + re.escape(cmd) + r'", ts_file: "[^"]*", work_unit: ")RPC-PENDING(")'
    )
    new_text, n = pattern.subn(r"\1" + rpc + r"\2", text, count=1)
    if n != 1:
        raise SystemExit(f"failed to update canonical.rs for {cmd}: {n} matches")
    text = new_text

# Also update the "RPC-PENDING" mentions in the module docstring to clarify
# the placeholder is now resolved.
text = text.replace(
    'until the\n//! per-command child cards under RPC-003 are created — at which point a\n//! follow-up step rewrites each entry with its real porting work-unit ID.',
    "(real RPC-165..RPC-326 IDs resolved per the\n//! per-command mapping at spec/attachments/TOOL-019/command-to-rpc-mapping.json).",
)
text = text.replace(
    'pub work_unit: &\'static str,\n    /// until the per-command child cards under RPC-003 are created.',
    "pub work_unit: &'static str,",
)

canonical_rs.write_text(text)
print(f"✓ canonical.rs: replaced 162 RPC-PENDING placeholders")


# ---- 2. Generate 161 stub files (skip add_rule which exists) ----
commands_dir = CORE / "src/commands"
generated = 0
for entry in canonical:
    cmd = entry["name"]
    ts_file = entry["ts_file"]
    rpc = mapping[cmd]
    snake_name = snake(cmd)
    stub_path = commands_dir / f"{snake_name}.rs"

    body = f'''//! Stub for the `{cmd}` fspec command. See {rpc} for the port work unit.
//! Original TypeScript implementation: {ts_file}

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {{
    Err(FspecCoreError::NotYetPorted {{
        command: "{cmd}",
        work_unit: "{rpc}",
    }})
}}
'''
    stub_path.write_text(body)
    generated += 1
print(f"✓ commands/: wrote {generated} stub files (overwrote existing add_rule.rs)")


# ---- 3. commands/mod.rs: declare all 162 modules ----
mod_lines = [
    "//! Per-command stub modules.",
    "//!",
    "//! TOOL-019: every fspec CLI command has a stub here returning",
    "//! `FspecCoreError::NotYetPorted` with the porting work-unit ID. As",
    "//! individual ports land (per the RPC-165..RPC-326 child cards under",
    "//! RPC-003), each stub is replaced with the real implementation.",
    "",
]
for entry in canonical:
    mod_lines.append(f"pub mod {snake(entry['name'])};")
mod_text = "\n".join(mod_lines) + "\n"
(commands_dir / "mod.rs").write_text(mod_text)
print(f"✓ commands/mod.rs: declares {len(canonical)} modules")


# ---- 4. dispatch.rs: rewrite the match arms ----
dispatch_path = CORE / "src/dispatch.rs"
dispatch = dispatch_path.read_text()

match_arms = []
for entry in canonical:
    cmd = entry["name"]
    snake_name = snake(cmd)
    match_arms.append(f'            "{cmd}" => commands::{snake_name}::run(args_json).await,')

new_match_block = (
    "        match name {\n"
    + "\n".join(match_arms)
    + "\n"
    + "            // Unreachable: canonical lookup already validated the\n"
    + "            // command exists, and every canonical entry has a stub.\n"
    + '            other => Err(FspecCoreError::UnknownCommand { command: other.to_string() }),\n'
    + "        }\n"
)

# Replace the existing block-on body. The previous body had:
# match name { "add-rule" => ..., other => Err(NotYetPorted{...}) }
# We replace the whole match { ... } region with the new exhaustive form.
pattern = re.compile(
    r"        match name \{\n.*?\n        \}\n",
    re.DOTALL,
)
new_dispatch, n = pattern.subn(new_match_block, dispatch, count=1)
if n != 1:
    raise SystemExit(f"failed to rewrite dispatch match: {n} matches")

# Remove the now-unused leak_static fn (canonical lookup never goes through
# the unreachable arm in the new exhaustive match — we use the command name
# directly).
new_dispatch = re.sub(
    r"\n/// Promote a `&str` whose lifetime we know is `'static`.*?fn leak_static\(s: &str\) -> &'static str \{\n.*?\n\}\n",
    "\n",
    new_dispatch,
    count=1,
    flags=re.DOTALL,
)

dispatch_path.write_text(new_dispatch)
print("✓ dispatch.rs: rewrote match to cover 162 commands, removed leak_static helper")

print("\nAll done. Now run: cd codelet && cargo build -p codelet-fspec-core && cargo test -p codelet-fspec-core")
