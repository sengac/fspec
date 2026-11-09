// REFAC-020: `Mode` variants, stage 8 of 9 (whole-item composition).
macro_rules! compose_mode_08 {
    ($($collected:tt)*) => {
        compose_mode_09! {
            $($collected)*
            /// RPC-240: link scenarios to test/impl files for traceability.
            #[command(
                name = "link-coverage",
                about = "Link Gherkin scenarios to test files and implementation code for full traceability"
            )]
            LinkCoverage {
                #[arg(value_name = "feature-name")]
                feature_name: String,
                // BUG-496: any value, including one starting with `--`.
                #[arg(long = "scenario", value_name = "name", allow_hyphen_values = true)]
                scenario: String,
                #[arg(long = "test-file", value_name = "path")]
                test_file: Option<String>,
                #[arg(long = "test-lines", value_name = "range")]
                test_lines: Option<String>,
                #[arg(long = "impl-file", value_name = "path")]
                impl_file: Option<String>,
                #[arg(long = "impl-lines", value_name = "lines")]
                impl_lines: Option<String>,
                #[arg(long = "skip-validation")]
                skip_validation: bool,
                #[arg(long = "skip-step-validation")]
                skip_step_validation: bool,
                /// REFAC-020: zero-based row selector, paired with the hash.
                #[arg(long = "test-mapping-index", value_name = "index")]
                test_mapping_index: Option<u64>,
                /// REFAC-020: canonical scenario hash from `show-coverage --mapping-json`.
                #[arg(long = "expected-scenario-hash", value_name = "hash")]
                expected_scenario_hash: Option<String>,
                /// BUG-502: replace the scenario's mappings of the test file.
                #[arg(long = "replace")]
                replace: bool,
            },
            /// RPC-235: generate a comprehensive project summary report.
            #[command(
                name = "generate-summary-report",
                about = "Generate a comprehensive project summary report"
            )]
            GenerateSummaryReport {
                #[arg(long = "format", value_name = "format")]
                format: Option<String>,
                #[arg(long = "output", value_name = "file")]
                output: Option<String>,
            },
            /// RPC-238: import Example Mapping data from JSON into a work unit.
            #[command(
                name = "import-example-map",
                about = "Import Example Mapping data from JSON file to work unit"
            )]
            ImportExampleMap {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "file")]
                file: String,
            },
            // Batch 18 (2026-06-16) — event-storm/analysis/work-unit-status
            /// RPC-225: scaffold an Event Storm session on a work unit.
            #[command(
                name = "discover-event-storm",
                about = "Start Event Storming discovery for a work unit"
            )]
            DiscoverEventStorm {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
            },
            /// RPC-232: transform an Event Storm into Example Mapping entries.
            #[command(
                name = "generate-example-mapping-from-event-storm",
                about = "Transform Event Storm artifacts into Example Mapping rules, examples, and questions"
            )]
            GenerateExampleMappingFromEventStorm {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
            },
            /// RPC-309: suggest dependencies between work units.
            #[command(
                name = "suggest-dependencies",
                about = "Suggest dependencies between work units based on heuristics"
            )]
            SuggestDependencies {
                #[arg(long = "output", value_name = "format")]
                output: Option<String>,
            },
            /// RPC-323: validate spec/test/impl alignment for a work unit.
            #[command(
                name = "validate-spec-alignment",
                about = "Validate that specifications align with tests and implementation"
            )]
            ValidateSpecAlignment {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(long = "fix")]
                fix: bool,
            },
            /// RPC-276: remove fspec init files for the detected agent.
            #[command(
                name = "remove-init-files",
                about = "Remove fspec initialization files created by init"
            )]
            RemoveInitFiles {
                #[arg(long = "keep-config")]
                keep_config: bool,
                #[arg(long = "no-keep-config")]
                no_keep_config: bool,
            },
            /// RPC-198: auto-advance a work unit through its lifecycle.
            #[command(
                name = "auto-advance",
                about = "Automatically advance work units through their lifecycle"
            )]
            AutoAdvance {
                #[arg(long = "dry-run")]
                dry_run: bool,
            },
            /// RPC-326: drive workflow automation actions for a work unit.
            #[command(
                name = "workflow-automation",
                about = "Run workflow automation actions for a work unit"
            )]
            WorkflowAutomation {
                #[arg(value_name = "action", allow_hyphen_values = true)]
                action: String,
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(long = "event", value_name = "event", allow_hyphen_values = true)]
                event: Option<String>,
                #[arg(long = "from-state", value_name = "state")]
                from_state: Option<String>,
            },
            /// RPC-202: create a manual git checkpoint for a work unit.
            #[command(
                name = "checkpoint",
                about = "Create a manual checkpoint for safe experimentation"
            )]
            Checkpoint {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "checkpoint-name", allow_hyphen_values = true)]
                checkpoint_name: String,
            },
            /// RPC-203: prune old checkpoints for a work unit.
            #[command(
                name = "cleanup-checkpoints",
                about = "Clean up old checkpoints for a work unit, keeping the most recent N"
            )]
            CleanupCheckpoints {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                // Parity with TS Commander.js `requiredOption('--keep-last <number>')`
                // (`src/commands/cleanup-checkpoints.ts:113`): the option is REQUIRED,
                // and Commander reports its absence (`required option '--keep-last
                // <number>' not specified`) even when the positional is also missing.
                // `allow_hyphen_values` lets clap accept a negative value (e.g. `-3`)
                // as the option's argument so it reaches our domain validation (which
                // emits the `--keep-last must be a positive number` message) instead
                // of clap treating `-3` as an unknown flag.
                #[arg(
                    long = "keep-last",
                    value_name = "number",
                    required = true,
                    allow_hyphen_values = true
                )]
                keep_last: String,
            },
            /// RPC-288: restore a checkpoint for a work unit.
            #[command(
                name = "restore-checkpoint",
                about = "Restore a checkpoint for a work unit"
            )]
            RestoreCheckpoint {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "checkpoint-name", allow_hyphen_values = true)]
                checkpoint_name: String,
            },
            /// RPC-294: reverse-engineer specifications from an existing codebase.
            #[command(
                name = "reverse",
                about = "Reverse-engineer specifications from existing code"
            )]
            Reverse {
                #[arg(long, value_name = "A|B|C|D")]
                strategy: Option<String>,
                #[arg(long = "continue")]
                r#continue: bool,
                #[arg(long)]
                status: bool,
                #[arg(long)]
                reset: bool,
                #[arg(long)]
                complete: bool,
                #[arg(long = "dry-run")]
                dry_run: bool,
            },
            /// RPC-319: update a work unit's workflow status.
            #[command(
                name = "update-work-unit-status",
                about = "Update a work unit's workflow status"
            )]
            UpdateWorkUnitStatus {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "status")]
                status: String,
                #[arg(long = "blocked-reason", value_name = "reason", allow_hyphen_values = true)]
                blocked_reason: Option<String>,
                #[arg(long = "reason", value_name = "reason", allow_hyphen_values = true)]
                reason: Option<String>,
                #[arg(long = "skip-temporal-validation")]
                skip_temporal_validation: bool,
            },
            /// RPC-226: interactive AI-guided foundation discovery.
            #[command(
                name = "discover-foundation",
                about = "Discover and scaffold the project foundation"
            )]
            DiscoverFoundation {
                #[arg(long = "output", value_name = "path")]
                output: Option<String>,
                #[arg(long = "finalize", action = clap::ArgAction::SetTrue)]
                finalize: bool,
                #[arg(long = "draft-path", value_name = "path")]
                draft_path: Option<String>,
                #[arg(
                    long = "auto-generate-md",
                    default_value_t = true,
                    action = clap::ArgAction::Set,
                    num_args = 0..=1,
                    default_missing_value = "true"
                )]
                auto_generate_md: bool,
                #[arg(long = "force", action = clap::ArgAction::SetTrue)]
                force: bool,
            },
            /// DISC-003: read-only progress report for foundation discovery.
            #[command(
                name = "foundation-status",
                about = "Show foundation phase, 8-field progress, and remaining fields with fix commands"
            )]
            FoundationStatus {
                #[arg(long)]
                json: bool,
            },
            /// CONFIG-009: validate the user-scope config for leaked project-scope
            /// keys (tools, agent). Warnings are non-fatal (exit 0).
            #[command(
                name = "validate-config",
                about = "Validate the user-scope config for leaked project-scope keys (tools, agent)"
            )]
            ValidateConfig {},
            /// RPC-234: generate a context-only Gherkin scaffold from an Example Map.
            #[command(
                name = "generate-scenarios",
                about = "Generate scenarios from a work unit's example map"
            )]
            GenerateScenarios {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(long = "feature", value_name = "name")]
                feature: Option<String>,
                #[arg(long = "ignore-possible-duplicates", action = clap::ArgAction::SetTrue)]
                ignore_possible_duplicates: bool,
            },
            /// RPC-239: initialize fspec for one or more AI agents in the project.
            #[command(name = "init", about = "Initialize fspec configuration and agent docs")]
            Init {
                #[arg(long = "agent", value_name = "agent")]
                agents: Vec<String>,
            },
            /// RPC-286: list available research tools (EXECUTE mode deferred).
            #[command(name = "research", about = "List available research tools")]
            Research {
                #[arg(long = "tool", value_name = "name")]
                tool: Option<String>,
                #[arg(long = "work-unit", value_name = "workUnitId")]
                work_unit: Option<String>,
            },
        }
    };
}
