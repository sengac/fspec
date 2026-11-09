// REFAC-020: `Mode` variants, stage 1 of 9 (whole-item composition).
macro_rules! compose_mode_01 {
    ($($collected:tt)*) => {
        compose_mode_02! {
            $($collected)*
            /// Run as a headless WS daemon (no TUI).
            Daemon {
                /// TCP bind address. RPC-456: a non-loopback address also needs
                /// --tls-cert/--tls-key and FSPEC_DAEMON_TOKEN (refused otherwise).
                #[arg(long, value_name = "ADDR", default_value = "127.0.0.1:0")]
                bind: String,
                /// RPC-456: PEM certificate chain; the daemon then serves wss://.
                #[arg(long, value_name = "PEM", requires = "tls_key")]
                tls_cert: Option<PathBuf>,
                /// RPC-456: PEM private key for --tls-cert.
                #[arg(long, value_name = "PEM", requires = "tls_cert")]
                tls_key: Option<PathBuf>,
                /// BUG-595: the host the TLS certificate is for; local clients
                /// dial it. Required with TLS on an unspecified --bind (0.0.0.0, ::).
                #[arg(long, value_name = "HOST", requires = "tls_cert")]
                tls_name: Option<String>,
                /// Write `pid=<u32>\nport=<u16>` to this path on bootstrap; remove on shutdown.
                #[arg(long, value_name = "PATH")]
                pidfile: Option<PathBuf>,
            },
            /// Run as a frontend client attached to a running daemon.
            Client {
                /// Explicit WS URL (e.g. `ws://127.0.0.1:12345`). When omitted,
                /// client resolves daemon via `$XDG_RUNTIME_DIR/fspec/daemon.json`.
                #[arg(long, value_name = "URL")]
                connect: Option<String>,
                /// BUG-429: bearer token for a token-protected daemon (else
                /// FSPEC_DAEMON_TOKEN; a saved daemon defaults to its token).
                #[arg(long, value_name = "TOKEN")]
                token: Option<String>,
                /// Expose MCP tools over stdio; full RPC requires a matching daemon.
                #[arg(long)]
                mcp: bool,
            },
            /// RPC-011: print live daemon health and exit.
            #[command(about = "One-shot health probe against the running daemon")]
            Status {
                /// Explicit WS URL — bypasses daemon.json autodiscovery.
                #[arg(long, value_name = "URL")]
                connect: Option<String>,
                /// BUG-429: bearer token for a token-protected daemon (else
                /// FSPEC_DAEMON_TOKEN; a saved daemon defaults to its token).
                #[arg(long, value_name = "TOKEN")]
                token: Option<String>,
            },
            /// UPD-002: in-place self-update. Downloads the latest platform binary
            /// from the GitHub release, verifies its checksum, and replaces the
            /// running binary (activated on next launch). `--check` is scriptable:
            /// exit 0 when current, exit 1 when a newer release is available.
            #[command(about = "Update fspec in place (or --check for availability)")]
            Update {
                /// Check only: print the latest version and exit 0 (current) or 1
                /// (update available) without downloading or replacing anything.
                #[arg(long)]
                check: bool,
            },
            /// RPC-253 / CLI-017: query the board in `spec/work-units.json`.
            /// Delegates to `fspec_core::commands::list_work_units::run` for
            /// two-front-doors parity.
            #[command(
                name = "list-work-units",
                about = "Query work units (filter by status, type, prefix, epic, time, history, text and links)"
            )]
            ListWorkUnits {
                #[command(flatten)]
                flags: Box<crate::list_work_units::Flags>,
            },
            /// RPC-248: list registered prefixes from `spec/prefixes.json`.
            #[command(name = "list-prefixes", about = "List all prefixes")]
            ListPrefixes,
            /// RPC-243: list registered epics from `spec/epics.json`.
            #[command(name = "list-epics", about = "List all epics")]
            ListEpics,
            /// RPC-251: list registered tags from `spec/tags.json`.
            #[command(name = "list-tags", about = "List all registered tags")]
            ListTags {
                /// Filter tags by category (exact-match against category name).
                #[arg(long, value_name = "CATEGORY", allow_hyphen_values = true)]
                category: Option<String>,
            },
            /// RPC-245: list Gherkin feature files under `spec/features/`.
            #[command(name = "list-features", about = "List all feature files")]
            ListFeatures {
                /// Filter features whose top-level tag set includes this tag (e.g. `@critical`).
                #[arg(long, value_name = "TAG")]
                tag: Option<String>,
            },
            /// RPC-241: list attachments for a work unit from `spec/work-units.json`.
            #[command(name = "list-attachments", about = "List attachments for a work unit")]
            ListAttachments {
                /// Required work-unit identifier (e.g. `AUTH-001`).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
            },
            /// RPC-247: list configured lifecycle hooks from `spec/fspec-hooks.json`.
            #[command(name = "list-hooks", about = "List all configured lifecycle hooks")]
            ListHooks,
            /// RPC-250: list scheduled jobs from `spec/schedules.json`.
            #[command(name = "list-schedules", about = "List all configured scheduled jobs")]
            ListSchedules {
                /// Emit the JSON payload instead of the tab-separated text
                /// table. Maps to the dispatcher `format: "json"` key.
                #[arg(long)]
                json: bool,
            },
            /// RPC-246: list canonical foundation sections.
            #[command(
                name = "list-foundation-sections",
                about = "List canonical foundation sections with their JSON paths and constraints"
            )]
            ListFoundationSections {
                /// Output format: `text` (default) or `json`.
                #[arg(long, value_name = "FORMAT")]
                format: Option<String>,
            },
            /// RPC-244: list all tags applied to scenarios in a feature file.
            #[command(
                name = "list-feature-tags",
                about = "List feature/scenario tags from a single feature file"
            )]
            ListFeatureTags {
                /// Required path to the .feature file (positional).
                #[arg(value_name = "FILE")]
                file: String,
                /// Group tags by category instead of flat alphabetical list.
                #[arg(long)]
                show_categories: bool,
            },
            /// RPC-249: list tags applied to a specific Scenario in a feature file.
            #[command(
                name = "list-scenario-tags",
                about = "List tags applied to a specific Scenario in a feature file"
            )]
            ListScenarioTags {
                /// Required path to the .feature file (positional).
                #[arg(value_name = "FILE")]
                file: String,
                /// Required Scenario name (positional).
                #[arg(value_name = "SCENARIO", allow_hyphen_values = true)]
                scenario: String,
                /// Group tags by category instead of flat alphabetical list.
                #[arg(long)]
                show_categories: bool,
            },
            /// RPC-252: list virtual (work-unit-scoped) hooks for a work unit.
            #[command(
                name = "list-virtual-hooks",
                about = "List virtual hooks for a work unit"
            )]
            ListVirtualHooks {
                /// Required work-unit identifier (e.g. `AUTH-001`).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
            },
            /// RPC-242: list checkpoints (git stashes) saved for a work unit.
            #[command(
                name = "list-checkpoints",
                about = "List all checkpoints for a work unit"
            )]
            ListCheckpoints {
                /// Required work-unit identifier (e.g. `AUTH-001`).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
            },
            /// RPC-301: show soft-deleted items for a work unit.
            #[command(
                name = "show-deleted",
                about = "Show all soft-deleted items for a work unit"
            )]
            ShowDeleted {
                /// Required work-unit identifier (e.g. `AUTH-001`).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
            },
            /// RPC-302: show details and progress for an epic.
            #[command(name = "show-epic", about = "Show epic details and progress")]
            ShowEpic {
                /// Required epic identifier.
                #[arg(value_name = "EPIC_ID")]
                epic_id: String,
                /// Output format: `text` (default) or `json`.
                #[arg(short = 'f', long, value_name = "FORMAT")]
                format: Option<String>,
            },
            /// RPC-304: show the contents of a feature file with work-unit annotations.
            #[command(name = "show-feature", about = "Show feature file contents")]
            ShowFeature {
                /// Required feature file path or basename.
                #[arg(value_name = "FEATURE")]
                feature: String,
                /// Output format: `text` (default) or `json`.
                #[arg(short = 'f', long, value_name = "FORMAT")]
                format: Option<String>,
                /// Optional output file path.
                #[arg(short = 'o', long, value_name = "PATH")]
                output: Option<String>,
            },
            /// RPC-310: print tag usage statistics across the project.
            #[command(
                name = "tag-stats",
                about = "Show tag usage statistics across feature files"
            )]
            TagStats,
            /// RPC-308: show full details of a single work unit.
            #[command(
                name = "show-work-unit",
                about = "Show full details of a single work unit"
            )]
            ShowWorkUnit {
                /// Required work-unit identifier (e.g. `AUTH-001`).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Output format: `text` (default) or `json`.
                #[arg(short = 'f', long, value_name = "FORMAT")]
                format: Option<String>,
            },
            /// RPC-257: print dependency relationship statistics.
            #[command(
                name = "query-dependency-stats",
                about = "Query dependency relationship statistics"
            )]
            QueryDependencyStats {
                /// Output format: `text` (default — silent) or `json`.
                #[arg(short = 'f', long, value_name = "FORMAT")]
                format: Option<String>,
            },
            /// RPC-258: print estimate accuracy aggregated across completed work units.
            #[command(
                name = "query-estimate-accuracy",
                about = "Query estimate accuracy across completed work units"
            )]
            QueryEstimateAccuracy {
                /// Output format: `text` (default) or `json`.
                #[arg(short = 'f', long, value_name = "FORMAT")]
                format: Option<String>,
            },
            /// RPC-261: print velocity / iteration / cycle-time metrics.
            #[command(
                name = "query-metrics",
                about = "Query velocity, iteration, and cycle-time metrics"
            )]
            QueryMetrics {
                /// `--work-unit <workUnitId>` — query metrics for a single unit.
                #[arg(long = "work-unit", value_name = "workUnitId")]
                work_unit_id: Option<String>,
                /// `--type <type>` — filter aggregate metrics to `story`, `task`, `bug`.
                #[arg(long = "type", value_name = "TYPE")]
                r#type: Option<String>,
                /// Output format: `text` (default) or `json`.
                #[arg(short = 'f', long, value_name = "FORMAT")]
                format: Option<String>,
            },
            /// RPC-263: query work units with rich filtering.
            #[command(
                name = "query-work-units",
                about = "Query work units with filters (status, prefix, epic, type, tag)"
            )]
            QueryWorkUnits {
                /// Filter by status.
                #[arg(short = 's', long, value_name = "STATUS")]
                status: Option<String>,
                /// Filter by prefix (no short flag: -p means --parent, CLI-020).
                #[arg(long, value_name = "PREFIX")]
                prefix: Option<String>,
                /// Filter by epic.
                #[arg(short = 'e', long, value_name = "EPIC")]
                epic: Option<String>,
                /// Filter by type.
                #[arg(short = 't', long = "type", value_name = "TYPE")]
                r#type: Option<String>,
                /// Filter by tag.
                #[arg(long, value_name = "TAG")]
                tag: Option<String>,
                /// Output format: `text` (default), `json`, or `csv`.
                #[arg(short = 'f', long, value_name = "FORMAT")]
                format: Option<String>,
            },
        }
    };
}
