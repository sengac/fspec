/// Byte-exact TS reference output of
/// `node dist/index.js register-tag --help` piped to non-TTY.
///
/// The TS reference has NO custom `-help.ts` for register-tag — Commander.js
/// emits its default Usage/Arguments/Options block, which the earlier Rust
/// port's `configs::register_tag::CONFIG` mismatched. Mirroring the bare
/// Commander output here keeps the help fixture byte-stable against the
/// upstream TS binary.
/// Captured fixture: `rust/fspec/tests/fixtures/help/register-tag.txt`.
const REGISTER_TAG_HELP: &str = "\
Usage: fspec register-tag [options] <tag> <category> <description>

Register a new tag in TAGS.md registry

Arguments:
  tag          Tag name (e.g., \"@my-tag\")
  category     Category name (e.g., \"Technical Tags\")
  description  Tag description

Options:
  -h, --help   Display help for command
";

/// Byte-exact TS reference output of
/// `node dist/index.js delete-features --help` piped to non-TTY.
///
/// The TS reference (`src/commands/delete-features-by-tag.ts`) has NO custom
/// `-help.ts`; Commander.js emits its default Usage/Description/Options block.
/// Captured fixture: `rust/fspec/tests/fixtures/help/delete-features.txt`.
const DELETE_FEATURES_HELP: &str = "\
Usage: fspec delete-features [options]

Bulk delete feature files by tag

Options:
  --tag <tag>  Filter by tag (can specify multiple times for AND logic)
  --dry-run    Preview deletions without making changes
  -h, --help   Display help for command
";

/// Byte-exact TS reference output of
/// `node dist/index.js delete-scenarios --help` piped to non-TTY.
///
/// The TS reference (`src/commands/delete-scenarios-by-tag.ts`) has NO custom
/// `-help.ts`; Commander.js emits its default Usage/Description/Options block.
/// Captured fixture: `rust/fspec/tests/fixtures/help/delete-scenarios.txt`.
const DELETE_SCENARIOS_HELP: &str = "\
Usage: fspec delete-scenarios [options]

Bulk delete scenarios by tag across multiple files

Options:
  --tag <tag>  Filter by tag (can specify multiple times for AND logic)
  --dry-run    Preview deletions without making changes
  -h, --help   Display help for command
";

/// Byte-exact TS reference output of
/// `node dist/index.js board --help` piped to non-TTY.
///
/// The TS reference (`src/commands/display-board.ts:90-96`) registers `board`
/// with plain Commander.js and has NO custom `-help.ts`, so `fspec board
/// --help` emits Commander's default Usage/Description/Options block. The
/// earlier Rust port wrongly introduced a rich `CommandHelpConfig`; this
/// reproduces the real TS bytes (RPC-199 parity fix).
const BOARD_HELP: &str = "\
Usage: fspec board [options]

Display Kanban board of work units

Options:
  --format <format>  Output format: text or json (default: \"text\")
  --limit <limit>    Max items per column (default: \"25\")
  -h, --help         Display help for command
";

/// Byte-exact TS reference output of
/// `node dist/index.js list-foundation-sections --help` piped to non-TTY.
///
/// The TS reference uses bare Commander.js without a custom `-help.ts`
/// file (see `src/commands/list-foundation-sections.ts:191-202`), so we
/// reproduce Commander's default Usage/Description/Options block verbatim.
/// Captured fixture: `rust/fspec/tests/fixtures/help/list-foundation-sections.txt`.
const LIST_FOUNDATION_SECTIONS_HELP: &str = "\
Usage: fspec list-foundation-sections [options]

List every valid foundation section with its JSON path and constraint info

Options:
  --format <format>  Output format: text (default) or json (default: \"text\")
  -h, --help         Display help for command
";
