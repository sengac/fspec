// REFAC-020: `Mode` variants, stage 9 of 9 (whole-item composition).
macro_rules! compose_mode_09 {
    ($($collected:tt)*) => {
        compose_mode_finish! {
            $($collected)*
            /// CLI-015: AST code search via the native AstGrep tool implementation.
            #[command(
                name = "astgrep",
                about = "AST code search (pattern-based, language-aware)"
            )]
            AstGrep {
                #[arg(long = "pattern", value_name = "pattern", required = true, allow_hyphen_values = true)]
                pattern: Option<String>,
                #[arg(long = "lang", value_name = "language", required = true)]
                lang: Option<String>,
                #[arg(long = "path", value_name = "path")]
                path: Option<String>,
            },
            /// RPC-200: print the fspec bootstrap workflow document.
            #[command(
                name = "bootstrap",
                about = "Print the fspec workflow bootstrap document"
            )]
            Bootstrap {},
            /// RPC-285: build a pre-filled GitHub bug-report issue URL.
            #[command(
                name = "report-bug-to-github",
                about = "Report a bug to the fspec GitHub repository"
            )]
            ReportBugToGitHub {
                #[arg(long = "project-root", value_name = "path")]
                project_root: Option<String>,
                #[arg(long = "bug-description", value_name = "text", allow_hyphen_values = true)]
                bug_description: Option<String>,
                #[arg(long = "expected-behavior", value_name = "text", allow_hyphen_values = true)]
                expected_behavior: Option<String>,
                #[arg(long = "actual-behavior", value_name = "text", allow_hyphen_values = true)]
                actual_behavior: Option<String>,
                #[arg(long = "interactive", action = clap::ArgAction::SetTrue)]
                interactive: bool,
            },
            /// RPC-295: review a work unit's ACDD compliance and produce a report.
            #[command(name = "review", about = "Review a work unit for ACDD compliance")]
            Review {
                // value_name mirrors the TS Commander positional `<workUnitId>`
                // (`src/commands/review.ts:571`) so clap's bare `--help` usage line AND
                // the missing-argument error (`error: missing required argument
                // 'work-unit-id'`) match the TS reference byte-for-byte.
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
            },
            /// BUG-507: replace one exact piece of non-step text in a feature file.
            #[command(
                name = "update-feature-text",
                about = "Replace one exact piece of non-step text in a feature file"
            )]
            UpdateFeatureText {
                #[arg(value_name = "feature")]
                feature: String,
                #[arg(long = "find", value_name = "text", required = true, allow_hyphen_values = true)]
                find: String,
                #[arg(long = "replace", value_name = "text", required = true, allow_hyphen_values = true)]
                replace: String,
            },
            /// RPC-447: manage saved daemons (name, URL, bearer token).
            #[command(about = "Manage saved fspec daemons (add, list, remove)")]
            Daemons {
                #[command(subcommand)]
                action: crate::daemons_cmd::DaemonsAction,
            },
            /// RPC-457: manage the workspace registry the daemon reads.
            #[command(about = "Manage registered workspaces (add, list, remove)")]
            Workspaces {
                #[command(subcommand)]
                action: crate::workspaces_cmd::WorkspacesAction,
            },
        }
    };
}
