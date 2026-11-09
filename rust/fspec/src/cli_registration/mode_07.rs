// REFAC-020: `Mode` variants, stage 7 of 9 (whole-item composition).
macro_rules! compose_mode_07 {
    ($($collected:tt)*) => {
        compose_mode_08! {
            $($collected)*
            /// RPC-167: add architecture notes to a feature file via doc strings.
            #[command(
                name = "add-architecture",
                about = "Add architecture notes to a feature file using doc strings"
            )]
            AddArchitecture {
                #[arg(value_name = "feature")]
                file: String,
                #[arg(value_name = "text", allow_hyphen_values = true)]
                notes: String,
            },
            /// RPC-219: delete a scenario from a feature file.
            #[command(
                name = "delete-scenario",
                about = "Delete a scenario from a feature file"
            )]
            DeleteScenario {
                #[arg(value_name = "feature")]
                file: String,
                #[arg(value_name = "scenario", allow_hyphen_values = true)]
                scenario: String,
            },
            /// RPC-221: delete a step from a scenario.
            #[command(name = "delete-step", about = "Delete a step from a scenario")]
            DeleteStep {
                #[arg(value_name = "feature")]
                file: String,
                #[arg(value_name = "scenario", allow_hyphen_values = true)]
                scenario: String,
                #[arg(value_name = "step", allow_hyphen_values = true)]
                step: String,
            },
            /// RPC-218: bulk delete feature files by tag.
            #[command(name = "delete-features", about = "Bulk delete feature files by tag")]
            DeleteFeatures {
                #[arg(long = "tag", value_name = "tag")]
                tag: Vec<String>,
                #[arg(long = "dry-run")]
                dry_run: bool,
            },
            /// RPC-314: update a scenario name in a feature file.
            #[command(
                name = "update-scenario",
                about = "Update a scenario name in a feature file"
            )]
            UpdateScenario {
                #[arg(value_name = "feature")]
                file: String,
                #[arg(value_name = "old-name", allow_hyphen_values = true)]
                old_name: String,
                #[arg(value_name = "new-name", allow_hyphen_values = true)]
                new_name: String,
            },
            /// RPC-315: update step text or keyword in a scenario.
            #[command(
                name = "update-step",
                about = "Update step text or keyword in a scenario by finding and replacing the current step"
            )]
            UpdateStep {
                #[arg(value_name = "feature")]
                feature: String,
                #[arg(value_name = "scenario", allow_hyphen_values = true)]
                scenario: String,
                #[arg(value_name = "current-step", allow_hyphen_values = true)]
                current_step: String,
                #[arg(long = "text", value_name = "text", overrides_with = "text", allow_hyphen_values = true)]
                text: Option<String>,
                #[arg(long = "keyword", value_name = "keyword", overrides_with = "keyword")]
                keyword: Option<String>,
            },
            // Batch 16 (2026-06-14) — validation + search + coverage + generator/retag.
            /// RPC-324: validate feature-file tags against spec/tags.json.
            #[command(
                name = "validate-tags",
                about = "Validate that all tags used in feature files are registered"
            )]
            ValidateTags {
                #[arg(value_name = "file")]
                file: Option<String>,
                #[arg(long = "verbose")]
                verbose: bool,
                #[arg(long = "summary")]
                summary: bool,
            },
            /// RPC-325: validate work-units.json data integrity.
            ///
            /// NOTE: the `--fix` option appears in `--help` (rich help, byte-parity
            /// with the TS reference) but is DOCUMENTED-ONLY. The TS Commander
            /// registration declares no functional flags, so `--fix` is rejected at
            /// runtime as an unknown option. The clap variant therefore declares NO
            /// fields — passing `--fix` raises `UnknownArgument` →
            /// `error: unknown option '--fix'`, exit 1 (parity, RPC-325 rule [9]).
            #[command(
                name = "validate-work-units",
                about = "Validate work units data integrity"
            )]
            ValidateWorkUnits {},
            /// RPC-322: validate hook configuration and verify scripts exist.
            #[command(
                name = "validate-hooks",
                about = "Validate hook configuration and verify that all hook scripts exist"
            )]
            ValidateHooks {},
            /// RPC-321: validate foundation.json against its JSON schema.
            #[command(
                name = "validate-foundation-schema",
                about = "Validate foundation.json against its JSON schema using Ajv"
            )]
            ValidateFoundationSchema {
                /// DISC-003: validate `spec/foundation.json.draft` instead of the
                /// finalized `spec/foundation.json`.
                #[arg(long)]
                draft: bool,
            },
            /// RPC-320: validate Gherkin syntax in feature files.
            #[command(
                name = "validate",
                about = "Validate Gherkin syntax in feature files using @cucumber/gherkin parser"
            )]
            Validate {
                #[arg(value_name = "file")]
                file: Option<String>,
                #[arg(short = 'v', long = "verbose")]
                verbose: bool,
            },
            /// RPC-296: search implementation files linked via coverage data.
            #[command(
                name = "search-implementation",
                about = "Search implementation files for a specific function"
            )]
            SearchImplementation {
                #[arg(long = "function", value_name = "name", allow_hyphen_values = true)]
                function: String,
                #[arg(long = "show-work-units")]
                show_work_units: bool,
                #[arg(long = "json")]
                json: bool,
            },
            /// RPC-311: remove test/impl mappings from a scenario's coverage sidecar.
            #[command(
                name = "unlink-coverage",
                about = "Remove test or implementation mappings from a scenario"
            )]
            UnlinkCoverage {
                #[arg(value_name = "feature-name")]
                feature_name: String,
                // BUG-496: any value, including one starting with `--`.
                #[arg(long = "scenario", value_name = "name", allow_hyphen_values = true)]
                scenario: String,
                #[arg(long = "test-file", value_name = "path")]
                test_file: Option<String>,
                #[arg(long = "impl-file", value_name = "path")]
                impl_file: Option<String>,
                #[arg(long = "all")]
                all: bool,
                /// REFAC-020: zero-based row selector, paired with the hash.
                #[arg(long = "test-mapping-index", value_name = "index")]
                test_mapping_index: Option<u64>,
                /// REFAC-020: canonical scenario hash from `show-coverage --mapping-json`.
                #[arg(long = "expected-scenario-hash", value_name = "hash")]
                expected_scenario_hash: Option<String>,
            },
            /// RPC-236: render spec/TAGS.md from spec/tags.json.
            #[command(
                name = "generate-tags-md",
                about = "Generate TAGS.md from spec/tags.json"
            )]
            GenerateTagsMd {
                #[arg(long = "output", value_name = "output")]
                output: Option<String>,
            },
            /// RPC-293: bulk-rename a tag across all feature files.
            #[command(name = "retag", about = "Rename a tag across all feature files")]
            Retag {
                #[arg(long = "from", value_name = "tag")]
                from: Option<String>,
                #[arg(long = "to", value_name = "tag")]
                to: Option<String>,
                #[arg(long = "dry-run")]
                dry_run: bool,
            },
            // Batch 17 (2026-06-15) — coverage/board/check/format/compare/import/report
            /// AUDIT-003: the one deterministic drift check.
            #[command(
                name = "audit",
                about = "Report every exact coverage, spec, Gherkin, tag and format problem with the command that fixes it"
            )]
            Audit {
                #[arg(long = "area", value_name = "name")]
                area: Vec<String>,
                #[arg(long = "feature", value_name = "name")]
                feature: Vec<String>,
                #[arg(long = "work-unit", value_name = "workUnitId")]
                work_unit: Option<String>,
                #[arg(long = "json")]
                json: bool,
                #[arg(long = "fix")]
                fix: bool,
                /// COV-077: audit the staged changes (the commit gate).
                #[arg(long = "changed")]
                changed: bool,
                /// AUDIT-004: start an audit session over the scope.
                #[arg(long = "start")]
                start: bool,
                #[arg(long = "status")]
                status: bool,
                #[arg(long = "continue")]
                r#continue: bool,
                #[arg(long = "reset")]
                reset: bool,
                #[arg(long = "complete")]
                complete: bool,
            },
            /// RPC-199: display Kanban board of work units grouped by status.
            #[command(
                name = "board",
                about = "Display Kanban board of all work units grouped by status"
            )]
            Board {
                #[arg(long = "format", value_name = "format")]
                format: Option<String>,
                #[arg(long = "limit", value_name = "limit")]
                limit: Option<usize>,
            },
            /// RPC-207: compare implementation approaches across work units.
            #[command(
                name = "compare-implementations",
                about = "Compare implementation approaches across work units to identify patterns and inconsistencies"
            )]
            CompareImplementations {
                #[arg(long = "tag", value_name = "tag")]
                tag: String,
                #[arg(long = "show-coverage")]
                show_coverage: bool,
                #[arg(long = "json")]
                json: bool,
            },
            /// RPC-220: bulk delete scenarios by tag across multiple files.
            #[command(
                name = "delete-scenarios",
                about = "Bulk delete scenarios by tag across multiple files"
            )]
            DeleteScenarios {
                #[arg(long = "tag", value_name = "tag")]
                tags: Vec<String>,
                #[arg(long = "dry-run")]
                dry_run: bool,
            },
            /// RPC-230: format feature files with the AST-based Gherkin formatter.
            #[command(
                name = "format",
                about = "Format feature files with custom AST-based Gherkin formatter"
            )]
            Format {
                #[arg(value_name = "file")]
                file: Option<String>,
            },
            /// RPC-231: generate/update .feature.coverage files.
            #[command(
                name = "generate-coverage",
                about = "Generate or update .feature.coverage files for existing .feature files"
            )]
            GenerateCoverage {
                #[arg(long = "dry-run")]
                dry_run: bool,
                /// REFAC-020: generate only this exact feature (JSON plan).
                #[arg(long = "feature-name", value_name = "name")]
                feature_name: Option<String>,
            },
        }
    };
}
