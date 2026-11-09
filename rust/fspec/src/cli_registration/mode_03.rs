// REFAC-020: `Mode` variants, stage 3 of 9 (whole-item composition).
macro_rules! compose_mode_03 {
    ($($collected:tt)*) => {
        compose_mode_04! {
            $($collected)*
            /// RPC-204: clear all dependencies from a work unit.
            #[command(
                name = "clear-dependencies",
                about = "Clear all dependencies from a work unit"
            )]
            ClearDependencies {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Confirm the destructive operation.
                #[arg(long)]
                confirm: bool,
            },
            /// RPC-189: add a business rule to a work unit (Blue card in Example Mapping).
            #[command(
                name = "add-rule",
                about = "Add a business rule to a work unit (Blue card in Example Mapping)"
            )]
            AddRule {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Business rule description (positional).
                #[arg(value_name = "RULE", allow_hyphen_values = true)]
                rule: String,
            },
            /// RPC-279: remove a business rule from a work unit by index.
            #[command(
                name = "remove-rule",
                about = "Remove a business rule from a work unit by index"
            )]
            RemoveRule {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Rule index (0-based, positional). Accepted as a raw string so TS
                /// `parseInt('abc', 10) → NaN` semantics are preserved when the
                /// caller passes a non-numeric value — the core then surfaces
                /// `"Rule with ID NaN not found"` instead of clap exiting with code 2.
                #[arg(value_name = "INDEX")]
                index: String,
            },
            /// RPC-169: add an assumption to a work unit during specification.
            #[command(
                name = "add-assumption",
                about = "Add assumption to work unit during specification"
            )]
            AddAssumption {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Assumption text (positional).
                #[arg(value_name = "ASSUMPTION", allow_hyphen_values = true)]
                assumption: String,
            },
            /// RPC-181: add an example to a work unit (Green card in Example Mapping).
            #[command(
                name = "add-example",
                about = "Add an example to a work unit during specification phase"
            )]
            AddExample {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Example description (positional).
                #[arg(value_name = "EXAMPLE", allow_hyphen_values = true)]
                example: String,
            },
            /// RPC-273: remove an example from a work unit by index.
            #[command(
                name = "remove-example",
                about = "Remove an example from a work unit by index"
            )]
            RemoveExample {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Example index (0-based, positional). Accepted as a raw string so
                /// TS `parseInt('abc', 10) → NaN` semantics are preserved when the
                /// caller passes a non-numeric value — the core then surfaces
                /// `"Example with ID NaN not found"` instead of clap exiting with
                /// code 2. `allow_hyphen_values` lets clap accept negative integers
                /// such as `-1` so TS-style `parseInt('-1', 10) → -1` parity holds.
                #[arg(value_name = "INDEX", allow_hyphen_values = true)]
                index: String,
            },
            /// RPC-188: add a question to a work unit (Red card in Example Mapping).
            #[command(
                name = "add-question",
                about = "Add a question to a work unit during specification phase"
            )]
            AddQuestion {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Question text (positional).
                #[arg(value_name = "QUESTION", allow_hyphen_values = true)]
                question: String,
            },
            /// RPC-278: remove a question from a work unit by index.
            #[command(
                name = "remove-question",
                about = "Remove a question from a work unit by index"
            )]
            RemoveQuestion {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Question index (0-based, positional). Accepted as a raw String
                /// so the bridge can preserve TS `parseInt(_, 10)` semantics
                /// (non-numeric input → "NaN" → canonical not-found error).
                #[arg(value_name = "INDEX")]
                index: String,
            },
            /// RPC-168: add an architecture note to a work unit during Example Mapping.
            #[command(
                name = "add-architecture-note",
                about = "Add architecture note to work unit during Example Mapping"
            )]
            AddArchitectureNote {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Architecture note text (positional).
                #[arg(value_name = "NOTE", allow_hyphen_values = true)]
                note: String,
            },
            /// RPC-267: remove an architecture note from a work unit by index.
            #[command(
                name = "remove-architecture-note",
                about = "Remove architecture note from work unit by index"
            )]
            RemoveArchitectureNote {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Index of note to remove (0-based, positional). Accepted as a
                /// raw String so the bridge can preserve TS `parseInt(_, 10)`
                /// semantics (non-numeric input → "NaN" → canonical not-found
                /// error).
                #[arg(value_name = "INDEX")]
                index: String,
            },
            /// RPC-298: set user-story fields (role/action/benefit) for a work unit.
            #[command(name = "set-user-story", about = "Set user story fields for work unit")]
            SetUserStory {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// User role (As a...).
                #[arg(long, value_name = "ROLE", allow_hyphen_values = true)]
                role: String,
                /// User action (I want to...).
                #[arg(long, value_name = "ACTION", allow_hyphen_values = true)]
                action: String,
                /// User benefit (So that...).
                #[arg(long, value_name = "BENEFIT", allow_hyphen_values = true)]
                benefit: String,
            },
            /// RPC-177: add a dependency relationship between two work units.
            #[command(
                name = "add-dependency",
                about = "Add dependency relationship between two work units"
            )]
            AddDependency {
                /// Required source work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Optional positional shorthand: adds a depends-on link.
                #[arg(value_name = "workUnitId")]
                depends_on_positional: Option<String>,
                /// Add a blocks edge.
                #[arg(long, value_name = "workUnitId")]
                blocks: Option<String>,
                /// Add a blocked-by edge.
                #[arg(long = "blocked-by", value_name = "workUnitId")]
                blocked_by: Option<String>,
                /// Add a depends-on edge.
                #[arg(long = "depends-on", value_name = "workUnitId")]
                depends_on: Option<String>,
                /// Add a relates-to edge.
                #[arg(long = "relates-to", value_name = "workUnitId")]
                relates_to: Option<String>,
            },
            /// RPC-196: answer an Example Mapping question and optionally promote it to a rule/assumption.
            #[command(
                name = "answer-question",
                about = "Answer a question and optionally promote it"
            )]
            AnswerQuestion {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Question index (0-based, positional).
                #[arg(value_name = "INDEX", allow_hyphen_values = true)]
                index: i64,
                /// Optional answer text.
                #[arg(long, value_name = "ANSWER", allow_hyphen_values = true)]
                answer: Option<String>,
                /// Where to promote the answer: rule|rules|assumption|assumptions|none.
                #[arg(long = "add-to", value_name = "TARGET")]
                add_to: Option<String>,
            },
            /// RPC-289: restore a soft-deleted example on a work unit.
            #[command(name = "restore-example", about = "Restore a soft-deleted example")]
            RestoreExample {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Example index/id (raw — TS `parseInt('abc', 10)` semantics).
                #[arg(value_name = "INDEX")]
                index: String,
            },
            /// RPC-291: restore a soft-deleted rule on a work unit.
            #[command(name = "restore-rule", about = "Restore a soft-deleted rule")]
            RestoreRule {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Rule index/id (raw — TS `parseInt('abc', 10)` semantics).
                #[arg(value_name = "INDEX")]
                index: String,
            },
            /// BUG-633: soft-delete an assumption on a work unit.
            #[command(name = "remove-assumption", about = "Remove an assumption (soft delete)")]
            RemoveAssumption {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Assumption ID (stable index, positional).
                #[arg(value_name = "INDEX")]
                index: u64,
            },
            /// BUG-633: restore a removed assumption on a work unit.
            #[command(name = "restore-assumption", about = "Restore a removed assumption")]
            RestoreAssumption {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Assumption ID (stable index, positional).
                #[arg(value_name = "INDEX")]
                index: u64,
            },
            /// RPC-290: restore a soft-deleted question on a work unit.
            #[command(name = "restore-question", about = "Restore a soft-deleted question")]
            RestoreQuestion {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Question index (0-based, positional).
                #[arg(value_name = "INDEX")]
                index: u64,
            },
            /// RPC-287: restore a soft-deleted architecture note on a work unit.
            #[command(
                name = "restore-architecture-note",
                about = "Restore a soft-deleted architecture note"
            )]
            RestoreArchitectureNote {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Architecture note index (0-based, positional).
                #[arg(value_name = "INDEX")]
                index: u64,
            },
            /// RPC-193: add one or more tags to a feature file.
            #[command(name = "add-tag-to-feature", about = "Add tags to a feature file")]
            AddTagToFeature {
                /// Feature file path (positional).
                #[arg(value_name = "FILE")]
                file: String,
                /// One or more tags to add (variadic positional).
                #[arg(value_name = "TAGS", num_args = 1..)]
                tags: Vec<String>,
                /// Validate added tags against `spec/tags.json` registry.
                #[arg(long = "validate-registry")]
                validate_registry: bool,
            },
        }
    };
}
