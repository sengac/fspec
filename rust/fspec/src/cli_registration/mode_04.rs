// REFAC-020: `Mode` variants, stage 4 of 9 (whole-item composition).
macro_rules! compose_mode_04 {
    ($($collected:tt)*) => {
        compose_mode_05! {
            $($collected)*
            /// RPC-281: remove one or more tags from a feature file.
            #[command(
                name = "remove-tag-from-feature",
                about = "Remove tags from a feature file"
            )]
            RemoveTagFromFeature {
                /// Feature file path (positional).
                #[arg(value_name = "FILE")]
                file: String,
                /// One or more tags to remove (variadic positional).
                #[arg(value_name = "TAGS", num_args = 1..)]
                tags: Vec<String>,
            },
            /// RPC-194: add one or more tags to a specific scenario in a feature file.
            #[command(
                name = "add-tag-to-scenario",
                about = "Add tags to a scenario in a feature file"
            )]
            AddTagToScenario {
                /// Feature file path (positional).
                #[arg(value_name = "FILE")]
                file: String,
                /// Scenario name (positional).
                #[arg(value_name = "SCENARIO", allow_hyphen_values = true)]
                scenario_name: String,
                /// One or more tags to add (variadic positional).
                #[arg(value_name = "TAGS", num_args = 1..)]
                tags: Vec<String>,
                /// Validate added tags against `spec/tags.json` registry.
                #[arg(long = "validate-registry")]
                validate_registry: bool,
            },
            /// RPC-282: remove one or more tags from a specific scenario in a feature file.
            #[command(
                name = "remove-tag-from-scenario",
                about = "Remove tags from a scenario in a feature file"
            )]
            RemoveTagFromScenario {
                /// Feature file path (positional).
                #[arg(value_name = "FILE")]
                file: String,
                /// Scenario name (positional).
                #[arg(value_name = "SCENARIO", allow_hyphen_values = true)]
                scenario_name: String,
                /// One or more tags to remove (variadic positional).
                #[arg(value_name = "TAGS", num_args = 1..)]
                tags: Vec<String>,
            },
            /// RPC-170: add an attachment to a work unit.
            #[command(name = "add-attachment", about = "Add an attachment to a work unit")]
            AddAttachment {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Source file path (positional).
                #[arg(value_name = "FILE_PATH")]
                file_path: String,
                /// Optional description of the attachment.
                #[arg(short = 'd', long = "description", value_name = "TEXT", allow_hyphen_values = true)]
                description: Option<String>,
            },
            /// RPC-268: remove an attachment from a work unit.
            #[command(
                name = "remove-attachment",
                about = "Remove an attachment from a work unit"
            )]
            RemoveAttachment {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Attachment file name (positional).
                #[arg(value_name = "FILE_NAME")]
                file_name: String,
                /// Keep the file on disk; only remove the JSON reference.
                #[arg(long = "keep-file")]
                keep_file: bool,
            },
            /// RPC-195: add a virtual hook to a work unit.
            #[command(name = "add-virtual-hook", about = "Add a virtual hook to a work unit")]
            AddVirtualHook {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Event name (positional).
                #[arg(value_name = "EVENT", allow_hyphen_values = true)]
                event: String,
                /// Hook command (positional).
                #[arg(value_name = "COMMAND")]
                command: String,
                /// Blocking hook (default false).
                #[arg(long = "blocking")]
                blocking: bool,
                /// Pass git context to the hook.
                #[arg(long = "git-context")]
                git_context: bool,
            },
            /// RPC-283: remove a virtual hook from a work unit by name.
            #[command(
                name = "remove-virtual-hook",
                about = "Remove a virtual hook from a work unit by name"
            )]
            RemoveVirtualHook {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                /// Hook name (positional).
                #[arg(value_name = "HOOK_NAME", allow_hyphen_values = true)]
                hook_name: String,
            },
            /// RPC-205: remove all virtual hooks from a work unit.
            #[command(
                name = "clear-virtual-hooks",
                about = "Clear all virtual hooks from a work unit"
            )]
            ClearVirtualHooks {
                /// Required work-unit ID (positional).
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
            },
            /// RPC-209: copy virtual hooks from one work unit to another.
            #[command(
                name = "copy-virtual-hooks",
                about = "Copy virtual hooks from one work unit to another"
            )]
            CopyVirtualHooks {
                /// Source work-unit ID.
                #[arg(long = "from", value_name = "workUnitId")]
                from: Option<String>,
                /// Target work-unit ID.
                #[arg(long = "to", value_name = "workUnitId")]
                to: Option<String>,
                /// Copy only the named hook.
                #[arg(long = "hook-name", value_name = "HOOK_NAME", allow_hyphen_values = true)]
                hook_name: Option<String>,
            },
            /// RPC-184: add a project-level hook.
            #[command(name = "add-hook", about = "Add a project-level lifecycle hook")]
            AddHook {
                /// Event name (positional).
                #[arg(value_name = "EVENT", allow_hyphen_values = true)]
                event: String,
                /// Hook name (positional).
                #[arg(value_name = "NAME", allow_hyphen_values = true)]
                name: String,
                /// Hook command.
                #[arg(long = "command", value_name = "COMMAND")]
                command: String,
                /// Blocking hook (default false).
                #[arg(long = "blocking")]
                blocking: bool,
                /// Timeout in seconds.
                #[arg(long = "timeout", value_name = "SECONDS")]
                timeout: Option<u64>,
            },
            /// RPC-275: remove a project-level hook by event + name.
            #[command(name = "remove-hook", about = "Remove a project-level lifecycle hook")]
            RemoveHook {
                /// Event name (positional).
                #[arg(value_name = "EVENT", allow_hyphen_values = true)]
                event: String,
                /// Hook name (positional).
                #[arg(value_name = "NAME", allow_hyphen_values = true)]
                name: String,
            },
            /// RPC-178: add a mermaid diagram to a foundation section.
            #[command(
                name = "add-diagram",
                about = "Add a mermaid diagram to a foundation section"
            )]
            AddDiagram {
                /// Foundation section (positional).
                #[arg(value_name = "SECTION", allow_hyphen_values = true)]
                section: String,
                /// Diagram title (positional).
                #[arg(value_name = "TITLE", allow_hyphen_values = true)]
                title: String,
                /// Mermaid code (positional).
                #[arg(value_name = "CODE")]
                code: String,
            },
            /// RPC-216: delete a mermaid diagram from a foundation section.
            #[command(
                name = "delete-diagram",
                about = "Delete a mermaid diagram from a foundation section"
            )]
            DeleteDiagram {
                /// Foundation section (positional).
                #[arg(value_name = "SECTION", allow_hyphen_values = true)]
                section: String,
                /// Diagram title (positional).
                #[arg(value_name = "TITLE", allow_hyphen_values = true)]
                title: String,
            },
            // Batch 11 (2026-06-12) — Event Storm item-add + create-* commands
            /// RPC-165: add an aggregate to a work unit's Event Storm section.
            #[command(
                name = "add-aggregate",
                about = "Add aggregate to Event Storm section of work unit"
            )]
            AddAggregate {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "TEXT", allow_hyphen_values = true)]
                text: String,
                #[arg(long, value_name = "LIST", allow_hyphen_values = true)]
                responsibilities: Option<String>,
                #[arg(long, value_name = "MS")]
                timestamp: Option<String>,
                #[arg(long = "bounded-context", value_name = "CONTEXT", allow_hyphen_values = true)]
                bounded_context: Option<String>,
            },
            /// RPC-174: add a command to a work unit's Event Storm section.
            #[command(
                name = "add-command",
                about = "Add command to Event Storm section of work unit"
            )]
            AddCommand {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "TEXT", allow_hyphen_values = true)]
                text: String,
                #[arg(long, value_name = "ACTOR", allow_hyphen_values = true)]
                actor: Option<String>,
                #[arg(long, value_name = "MS")]
                timestamp: Option<String>,
                #[arg(long = "bounded-context", value_name = "CONTEXT", allow_hyphen_values = true)]
                bounded_context: Option<String>,
            },
            /// RPC-179: add a domain event to a work unit's Event Storm section.
            #[command(
                name = "add-domain-event",
                about = "Add domain event to Event Storm section of work unit"
            )]
            AddDomainEvent {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "TEXT", allow_hyphen_values = true)]
                text: String,
                #[arg(long, value_name = "MS")]
                timestamp: Option<String>,
                #[arg(long = "bounded-context", value_name = "CONTEXT", allow_hyphen_values = true)]
                bounded_context: Option<String>,
            },
            /// RPC-185: add a hotspot to a work unit's Event Storm section.
            #[command(
                name = "add-hotspot",
                about = "Add hotspot to Event Storm section of work unit"
            )]
            AddHotspot {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "TEXT", allow_hyphen_values = true)]
                text: String,
                #[arg(long, value_name = "CONCERN", allow_hyphen_values = true)]
                concern: Option<String>,
                #[arg(long, value_name = "MS")]
                timestamp: Option<String>,
                #[arg(long = "bounded-context", value_name = "CONTEXT", allow_hyphen_values = true)]
                bounded_context: Option<String>,
            },
        }
    };
}
