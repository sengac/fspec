// REFAC-020: `Mode` variants, stage 2 of 9 (whole-item composition).
macro_rules! compose_mode_02 {
    ($($collected:tt)*) => {
        compose_mode_03! {
            $($collected)*
            /// RPC-256: print bottleneck analysis for work-unit dependency graph.
            #[command(name = "query-bottlenecks", about = "Identify dependency bottlenecks")]
            QueryBottlenecks {
                /// Output format: `text` (default) or `json`.
                #[arg(short = 'o', long, value_name = "FORMAT")]
                output: Option<String>,
            },
            /// RPC-262: print orphan work units (no dependencies in either direction).
            #[command(name = "query-orphans", about = "Show orphan work units")]
            QueryOrphans {
                /// Output format: `text` (default) or `json`.
                #[arg(short = 'o', long, value_name = "FORMAT")]
                output: Option<String>,
                /// Filter out work units in done status.
                #[arg(long = "exclude-done")]
                exclude_done: bool,
            },
            /// RPC-259: print estimation guide for a single work unit.
            #[command(
                name = "query-estimation-guide",
                about = "Show estimation guide for a work unit"
            )]
            QueryEstimationGuide {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Output format: `text` (default — silent) or `json`.
                #[arg(short = 'f', long, value_name = "FORMAT")]
                format: Option<String>,
            },
            /// RPC-260: print Example Mapping coverage statistics.
            #[command(
                name = "query-example-mapping-stats",
                about = "Show Example Mapping coverage statistics"
            )]
            QueryExampleMappingStats {
                /// Output format: `text` (default — silent) or `json`.
                #[arg(short = 'f', long, value_name = "FORMAT")]
                format: Option<String>,
            },
            /// RPC-303: show Event Storm artifacts for a work unit.
            #[command(
                name = "show-event-storm",
                about = "Show Event Storm artifacts for a work unit"
            )]
            ShowEventStorm {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
            },
            /// RPC-305: show contents of foundation.json (or a single section).
            #[command(name = "show-foundation", about = "Show foundation.json contents")]
            ShowFoundation {
                /// Optional positional section name (e.g. `whatWeAreBuilding`).
                section: Option<String>,
                /// BUG-537: the same section as the positional; giving both is
                /// a parse error.
                #[arg(
                    long = "section",
                    value_name = "SECTION",
                    conflicts_with = "section"
                )]
                section_flag: Option<String>,
                /// Output format: `text` (default), `json`, or `markdown`.
                #[arg(short = 'f', long, value_name = "FORMAT")]
                format: Option<String>,
                /// Write output to file instead of stdout.
                #[arg(short = 'o', long, value_name = "FILE")]
                output: Option<String>,
                /// Read from `foundation.json.draft` instead of `foundation.json`.
                #[arg(long)]
                draft: bool,
                /// DISC-003: force the finalized `foundation.json` even when a draft
                /// exists (defeats the draft auto-preference).
                #[arg(long)]
                r#final: bool,
                /// Accepted for TS parity; lists all valid sections.
                #[arg(long = "list-sections")]
                list_sections: bool,
                /// Accepted for TS parity; no behaviour change.
                #[arg(long = "line-numbers")]
                line_numbers: bool,
            },
            /// RPC-306: show foundation-level Event Storm.
            #[command(
                name = "show-foundation-event-storm",
                about = "Show foundation-level Event Storm"
            )]
            ShowFoundationEventStorm {
                /// Filter by type: `event`, `command`, `aggregate`, etc.
                #[arg(long = "type", value_name = "TYPE")]
                r#type: Option<String>,
                /// Filter by bounded context name.
                #[arg(long, value_name = "CONTEXT", allow_hyphen_values = true)]
                context: Option<String>,
            },
            /// RPC-307: show test patterns across coverage files for a tag.
            #[command(name = "show-test-patterns", about = "Show test patterns for a tag")]
            ShowTestPatterns {
                /// Tag to filter on (required).
                #[arg(long, value_name = "TAG")]
                tag: String,
                /// Include coverage data in output.
                #[arg(long = "include-coverage")]
                include_coverage: bool,
                /// Emit JSON instead of human-readable output.
                #[arg(long)]
                json: bool,
            },
            /// CLI-019: load acceptance criteria by tag expression, in pages.
            #[command(
                name = "context",
                about = "Load acceptance criteria selected by a cucumber tag expression, in pages"
            )]
            Context {
                #[command(flatten)]
                flags: Box<crate::context::Flags>,
            },
            /// RPC-300: show coverage report for a feature or project-wide.
            #[command(
                name = "show-coverage",
                about = "Show coverage report for a feature (or project-wide)"
            )]
            ShowCoverage {
                /// Optional feature name (with or without `.feature` extension).
                feature_name: Option<String>,
                /// Output format: `text` (default) or `json`.
                #[arg(short = 'f', long, value_name = "FORMAT")]
                format: Option<String>,
                /// Write output to file instead of stdout.
                #[arg(short = 'o', long, value_name = "FILE")]
                output: Option<String>,
                /// REFAC-020: explicit ordered mapping readback with scenario hashes.
                #[arg(long = "mapping-json")]
                mapping_json: bool,
            },
            /// RPC-211: create a new epic in spec/epics.json.
            #[command(name = "create-epic", about = "Create a new epic")]
            CreateEpic {
                /// Required epic identifier (positional).
                #[arg(value_name = "EPIC_ID")]
                epic_id: String,
                /// Required epic title (positional).
                #[arg(value_name = "TITLE", allow_hyphen_values = true)]
                title: String,
                /// Optional epic description.
                #[arg(short = 'd', long, value_name = "DESCRIPTION", allow_hyphen_values = true)]
                description: Option<String>,
            },
            /// RPC-217: delete an epic from spec/epics.json.
            #[command(name = "delete-epic", about = "Delete an epic")]
            DeleteEpic {
                /// Required epic identifier (positional).
                #[arg(value_name = "EPIC_ID")]
                epic_id: String,
                /// Force delete even when work-units reference this epic.
                /// NOTE: long-form `--force` only — TS Commander.js does NOT
                /// expose a `-f` short alias (`src/commands/delete-epic.ts:97`)
                /// and accepting one here would diverge from the byte-parity
                /// help fixture and from any shell script that relies on `-f`
                /// being an "unknown option" error.
                #[arg(long)]
                force: bool,
            },
            /// RPC-213: register a new work-unit prefix in spec/prefixes.json.
            #[command(name = "create-prefix", about = "Register a new work unit prefix")]
            CreatePrefix {
                /// Required prefix code, 2-6 uppercase letters (positional).
                #[arg(value_name = "PREFIX")]
                prefix: String,
                /// Required prefix description (positional).
                #[arg(value_name = "DESCRIPTION", allow_hyphen_values = true)]
                description: String,
            },
            /// RPC-265: register a new tag in spec/tags.json.
            #[command(name = "register-tag", about = "Register a new tag")]
            RegisterTag {
                /// Tag name (positional; e.g. `@critical`).
                #[arg(value_name = "TAG")]
                tag: String,
                /// Tag category (positional).
                #[arg(value_name = "CATEGORY", allow_hyphen_values = true)]
                category: String,
                /// Tag description (positional).
                #[arg(value_name = "DESCRIPTION", allow_hyphen_values = true)]
                description: String,
            },
            /// RPC-313: update an existing prefix in spec/prefixes.json.
            #[command(name = "update-prefix", about = "Update an existing prefix")]
            UpdatePrefix {
                /// Required prefix code (positional).
                #[arg(value_name = "PREFIX")]
                prefix: String,
                /// New description for the prefix.
                #[arg(short = 'd', long, value_name = "DESCRIPTION", allow_hyphen_values = true)]
                description: Option<String>,
            },
            /// RPC-316: update an existing tag in spec/tags.json.
            #[command(name = "update-tag", about = "Update an existing tag")]
            UpdateTag {
                /// Tag name (positional).
                #[arg(value_name = "TAG")]
                tag: String,
                /// New category for the tag.
                #[arg(short = 'c', long, value_name = "CATEGORY", allow_hyphen_values = true)]
                category: Option<String>,
                /// New description for the tag.
                #[arg(short = 'd', long, value_name = "DESCRIPTION", allow_hyphen_values = true)]
                description: Option<String>,
            },
            /// RPC-176: add multiple dependencies to a work unit at once.
            #[command(
                name = "add-dependencies",
                about = "Add multiple dependencies to a work unit"
            )]
            AddDependencies {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Work-unit IDs this unit blocks (variadic per flag).
                #[arg(long, num_args = 1.., value_name = "workUnitId")]
                blocks: Option<Vec<String>>,
                /// Work-unit IDs blocking this unit (variadic per flag).
                #[arg(long = "blocked-by", num_args = 1.., value_name = "workUnitId")]
                blocked_by: Option<Vec<String>>,
                /// Work-unit IDs this unit depends on (variadic per flag).
                #[arg(long = "depends-on", num_args = 1.., value_name = "workUnitId")]
                depends_on: Option<Vec<String>>,
                /// Related work-unit IDs (variadic per flag).
                #[arg(long = "relates-to", num_args = 1.., value_name = "workUnitId")]
                relates_to: Option<Vec<String>>,
            },
            /// RPC-222: delete a tag from spec/tags.json.
            #[command(name = "delete-tag", about = "Delete a tag")]
            DeleteTag {
                /// Tag name (positional).
                #[arg(value_name = "TAG")]
                tag: String,
                /// Force delete even when the tag is in use.
                #[arg(short = 'f', long)]
                force: bool,
                /// Dry-run: report intended deletion without writing.
                #[arg(long = "dry-run")]
                dry_run: bool,
            },
            /// RPC-271: remove a dependency relationship between two work units.
            #[command(name = "remove-dependency", about = "Remove a dependency relationship")]
            RemoveDependency {
                /// Source work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Optional positional shorthand: removes a depends-on link.
                #[arg(value_name = "workUnitId")]
                depends_on_positional: Option<String>,
                /// Remove a blocks edge.
                #[arg(long, value_name = "workUnitId")]
                blocks: Option<String>,
                /// Remove a blocked-by edge.
                #[arg(long = "blocked-by", value_name = "workUnitId")]
                blocked_by: Option<String>,
                /// Remove a depends-on edge.
                #[arg(long = "depends-on", value_name = "workUnitId")]
                depends_on: Option<String>,
                /// Remove a relates-to edge.
                #[arg(long = "relates-to", value_name = "workUnitId")]
                relates_to: Option<String>,
            },
        }
    };
}
