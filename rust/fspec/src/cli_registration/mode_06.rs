// REFAC-020: `Mode` variants, stage 6 of 9 (whole-item composition).
macro_rules! compose_mode_06 {
    ($($collected:tt)*) => {
        compose_mode_07! {
            $($collected)*
            /// RPC-183: add a bounded context to the foundation Big Picture Event Storm.
            #[command(
                name = "add-foundation-bounded-context",
                about = "Add a bounded context to foundation Big Picture Event Storm"
            )]
            AddFoundationBoundedContext {
                #[arg(value_name = "text", allow_hyphen_values = true)]
                text: String,
            },
            /// RPC-274: remove a bounded context from the foundation Big Picture Event Storm (soft-delete).
            #[command(
                name = "remove-foundation-bounded-context",
                about = "Remove a bounded context from foundation Big Picture Event Storm (soft-delete)"
            )]
            RemoveFoundationBoundedContext {
                #[arg(value_name = "context-name", allow_hyphen_values = true)]
                context_name: String,
                #[arg(long)]
                cascade: bool,
            },
            /// RPC-166: add an aggregate to a foundation bounded context.
            #[command(
                name = "add-aggregate-to-foundation",
                about = "Add an aggregate to a foundation bounded context in Big Picture Event Storm"
            )]
            AddAggregateToFoundation {
                #[arg(value_name = "context-name", allow_hyphen_values = true)]
                context_name: String,
                #[arg(value_name = "aggregate-name", allow_hyphen_values = true)]
                aggregate_name: String,
                #[arg(short = 'd', long = "description", value_name = "text", allow_hyphen_values = true)]
                description: Option<String>,
            },
            /// RPC-266: remove an aggregate from a foundation bounded context (soft-delete).
            #[command(
                name = "remove-aggregate-from-foundation",
                about = "Remove an aggregate from a foundation bounded context (soft-delete)"
            )]
            RemoveAggregateFromFoundation {
                #[arg(value_name = "context-name", allow_hyphen_values = true)]
                context_name: String,
                #[arg(value_name = "aggregate-name", allow_hyphen_values = true)]
                aggregate_name: String,
            },
            /// RPC-175: add a command to a foundation bounded context.
            #[command(
                name = "add-command-to-foundation",
                about = "Add a command to a foundation bounded context in Big Picture Event Storm"
            )]
            AddCommandToFoundation {
                #[arg(value_name = "context-name", allow_hyphen_values = true)]
                context_name: String,
                #[arg(value_name = "command-name", allow_hyphen_values = true)]
                command_name: String,
                #[arg(short = 'd', long = "description", value_name = "text", allow_hyphen_values = true)]
                description: Option<String>,
            },
            /// RPC-270: remove a command from a foundation bounded context (soft-delete).
            #[command(
                name = "remove-command-from-foundation",
                about = "Remove a command from a foundation bounded context (soft-delete)"
            )]
            RemoveCommandFromFoundation {
                #[arg(value_name = "context-name", allow_hyphen_values = true)]
                context_name: String,
                #[arg(value_name = "command-name", allow_hyphen_values = true)]
                command_name: String,
            },
            /// RPC-233: generate FOUNDATION.md from foundation.json.
            #[command(
                name = "generate-foundation-md",
                about = "Generate FOUNDATION.md from foundation.json"
            )]
            GenerateFoundationMd {
                #[arg(long = "output", value_name = "path")]
                output: Option<String>,
            },
            /// RPC-191: add a scheduled job (agent or shell) to schedules.json.
            #[command(
                name = "add-schedule",
                about = "Add a scheduled workflow automation job"
            )]
            AddSchedule {
                #[arg(
                    short = 'n',
                    long = "name",
                    value_name = "name",
                    allow_hyphen_values = true
                )]
                name: String,
                #[arg(
                    short = 'c',
                    long = "cron",
                    value_name = "expression",
                    allow_hyphen_values = true
                )]
                cron: String,
                #[arg(
                    short = 'z',
                    long = "timezone",
                    value_name = "tz",
                    allow_hyphen_values = true
                )]
                timezone: String,
                #[arg(
                    short = 't',
                    long = "type",
                    value_name = "type",
                    allow_hyphen_values = true
                )]
                r#type: String,
                #[arg(
                    short = 'r',
                    long = "role",
                    value_name = "role",
                    allow_hyphen_values = true
                )]
                role: Option<String>,
                #[arg(
                    short = 'p',
                    long = "prompt",
                    value_name = "prompt",
                    allow_hyphen_values = true
                )]
                prompt: Option<String>,
                #[arg(long = "command", value_name = "command", allow_hyphen_values = true)]
                command: Option<String>,
                #[arg(
                    short = 'o',
                    long = "overlap",
                    value_name = "policy",
                    default_value = "skip",
                    allow_hyphen_values = true
                )]
                overlap: String,
            },
            /// RPC-280: remove a scheduled job from schedules.json.
            #[command(name = "remove-schedule", about = "Remove a scheduled job")]
            RemoveSchedule {
                #[arg(value_name = "name", allow_hyphen_values = true)]
                name: String,
            },
            /// RPC-254: pause an active scheduled job.
            #[command(name = "pause-schedule", about = "Pause an active scheduled job")]
            PauseSchedule {
                #[arg(value_name = "name", allow_hyphen_values = true)]
                name: String,
            },
            /// RPC-292: resume a paused scheduled job.
            #[command(name = "resume-schedule", about = "Resume a paused scheduled job")]
            ResumeSchedule {
                #[arg(value_name = "name", allow_hyphen_values = true)]
                name: String,
            },
            /// RPC-180: add a domain event to a foundation bounded context.
            #[command(
                name = "add-domain-event-to-foundation",
                about = "Add a domain event to a foundation bounded context in Big Picture Event Storm"
            )]
            AddDomainEventToFoundation {
                #[arg(value_name = "context-name", allow_hyphen_values = true)]
                context_name: String,
                #[arg(value_name = "event-name", allow_hyphen_values = true)]
                event_name: String,
                #[arg(short = 'd', long = "description", value_name = "text", allow_hyphen_values = true)]
                description: Option<String>,
            },
            /// RPC-272: remove a domain event from a foundation bounded context (soft-delete).
            #[command(
                name = "remove-domain-event-from-foundation",
                about = "Remove a domain event from a foundation bounded context (soft-delete)"
            )]
            RemoveDomainEventFromFoundation {
                #[arg(value_name = "context-name", allow_hyphen_values = true)]
                context_name: String,
                #[arg(value_name = "event-name", allow_hyphen_values = true)]
                event_name: String,
            },
            /// RPC-224: show dependency relationships for a work unit.
            #[command(
                name = "dependencies",
                about = "Show all dependency relationships for a work unit"
            )]
            Dependencies {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(long = "graph")]
                graph: bool,
            },
            /// RPC-312: update a foundation field.
            #[command(name = "update-foundation", about = "Update a foundation field")]
            UpdateFoundation {
                #[arg(value_name = "section", allow_hyphen_values = true)]
                section: String,
                #[arg(value_name = "content", allow_hyphen_values = true)]
                content: String,
            },
            /// RPC-208: configure test and quality-check tool commands.
            #[command(
                name = "configure-tools",
                about = "Configure test and quality check commands"
            )]
            ConfigureTools {
                #[arg(long = "test-command", value_name = "command", allow_hyphen_values = true)]
                test_command: Option<String>,
                #[arg(long = "quality-commands", value_name = "commands", num_args = 1..)]
                quality_commands: Option<Vec<String>>,
                #[arg(long = "reconfigure")]
                reconfigure: bool,
            },
            /// RPC-212: create a new feature file from a template.
            #[command(
                name = "create-feature",
                about = "Create a new feature file with proper Gherkin structure template"
            )]
            CreateFeature {
                #[arg(value_name = "name", allow_hyphen_values = true)]
                name: String,
            },
            /// RPC-190: add a scenario to an existing feature file.
            #[command(
                name = "add-scenario",
                about = "Add a new scenario to an existing feature file"
            )]
            AddScenario {
                #[arg(value_name = "file")]
                file: String,
                #[arg(value_name = "scenario-name", allow_hyphen_values = true)]
                scenario_name: String,
            },
            /// RPC-192: add a step to a scenario in a feature file.
            #[command(
                name = "add-step",
                about = "Add a step to a scenario in a feature file"
            )]
            AddStep {
                #[arg(value_name = "file")]
                file: String,
                #[arg(value_name = "scenario", allow_hyphen_values = true)]
                scenario: String,
                #[arg(value_name = "type")]
                r#type: String,
                #[arg(value_name = "text", allow_hyphen_values = true)]
                text: String,
            },
            /// RPC-171: add or update the Background (user story) section.
            #[command(
                name = "add-background",
                about = "Add or update Background (user story) section in a feature file"
            )]
            AddBackground {
                #[arg(value_name = "feature")]
                feature: String,
                #[arg(value_name = "text", allow_hyphen_values = true)]
                text: String,
            },
        }
    };
}
