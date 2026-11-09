// REFAC-020: `Mode` variants, stage 5 of 9 (whole-item composition).
macro_rules! compose_mode_05 {
    ($($collected:tt)*) => {
        compose_mode_06! {
            $($collected)*
            /// RPC-172: add a bounded context to a work unit's Event Storm section.
            #[command(
                name = "add-bounded-context",
                about = "Add bounded context to Event Storm section of work unit"
            )]
            AddBoundedContext {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "TEXT", allow_hyphen_values = true)]
                text: String,
                #[arg(long, value_name = "DESCRIPTION", allow_hyphen_values = true)]
                description: Option<String>,
                #[arg(long, value_name = "MS")]
                timestamp: Option<String>,
                #[arg(long = "bounded-context", value_name = "CONTEXT", allow_hyphen_values = true)]
                context: Option<String>,
            },
            /// RPC-182: add an external system to a work unit's Event Storm section.
            #[command(
                name = "add-external-system",
                about = "Add external system to Event Storm section of work unit"
            )]
            AddExternalSystem {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "TEXT", allow_hyphen_values = true)]
                text: String,
                #[arg(long = "type", value_name = "TYPE")]
                system_type: Option<String>,
                #[arg(long, value_name = "MS")]
                timestamp: Option<String>,
                #[arg(long = "bounded-context", value_name = "CONTEXT", allow_hyphen_values = true)]
                context: Option<String>,
            },
            /// RPC-187: add a policy item to a work unit's Event Storm section.
            #[command(
                name = "add-policy",
                about = "Add policy to Event Storm section for reactive business logic (WHEN event THEN command)"
            )]
            AddPolicy {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "TEXT", allow_hyphen_values = true)]
                text: String,
                #[arg(long, value_name = "EVENT", allow_hyphen_values = true)]
                when: Option<String>,
                #[arg(long, value_name = "COMMAND", allow_hyphen_values = true)]
                then: Option<String>,
                #[arg(long, value_name = "MS")]
                timestamp: Option<String>,
                #[arg(long = "bounded-context", value_name = "NAME", allow_hyphen_values = true)]
                bounded_context: Option<String>,
            },
            /// RPC-214: create a new story work unit.
            #[command(
                name = "create-story",
                about = "Create a new story with Example Mapping guidance for defining acceptance criteria"
            )]
            CreateStory {
                #[arg(value_name = "PREFIX")]
                prefix: String,
                #[arg(value_name = "TITLE", allow_hyphen_values = true)]
                title: String,
                #[arg(short = 'd', long, value_name = "DESCRIPTION", allow_hyphen_values = true)]
                description: Option<String>,
                #[arg(short = 'e', long, value_name = "EPIC")]
                epic: Option<String>,
                #[arg(short = 'p', long, value_name = "workUnitId")]
                parent: Option<String>,
            },
            /// RPC-210: create a new bug work unit.
            #[command(name = "create-bug", about = "Create a new bug with research guidance")]
            CreateBug {
                #[arg(value_name = "PREFIX")]
                prefix: String,
                #[arg(value_name = "TITLE", allow_hyphen_values = true)]
                title: String,
                #[arg(short = 'd', long, value_name = "DESCRIPTION", allow_hyphen_values = true)]
                description: Option<String>,
                #[arg(short = 'e', long, value_name = "EPIC")]
                epic: Option<String>,
                #[arg(short = 'p', long, value_name = "workUnitId")]
                parent: Option<String>,
            },
            /// RPC-215: create a new task work unit.
            #[command(
                name = "create-task",
                about = "Create a new task with minimal requirements"
            )]
            CreateTask {
                #[arg(value_name = "PREFIX")]
                prefix: String,
                #[arg(value_name = "TITLE", allow_hyphen_values = true)]
                title: String,
                #[arg(short = 'd', long, value_name = "DESCRIPTION", allow_hyphen_values = true)]
                description: Option<String>,
                #[arg(short = 'e', long, value_name = "EPIC")]
                epic: Option<String>,
                #[arg(short = 'p', long, value_name = "workUnitId")]
                parent: Option<String>,
            },
            /// RPC-317: update work unit metadata.
            #[command(
                name = "update-work-unit",
                about = "Update work unit fields (title, description, epic, parent)"
            )]
            UpdateWorkUnit {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(short = 't', long, value_name = "TITLE", allow_hyphen_values = true)]
                title: Option<String>,
                #[arg(short = 'd', long, value_name = "DESCRIPTION", allow_hyphen_values = true)]
                description: Option<String>,
                #[arg(short = 'e', long, value_name = "EPIC")]
                epic: Option<String>,
                #[arg(short = 'p', long, value_name = "workUnitId")]
                parent: Option<String>,
            },
            /// RPC-318: set a Fibonacci story-point estimate.
            #[command(
                name = "update-work-unit-estimate",
                about = "Set a Fibonacci story-point estimate on a work unit"
            )]
            UpdateWorkUnitEstimate {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "estimate")]
                estimate: String,
            },
            /// RPC-223: delete a work unit.
            #[command(name = "delete-work-unit", about = "Delete a work unit")]
            DeleteWorkUnit {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(long)]
                force: bool,
                #[arg(long = "skip-confirmation")]
                skip_confirmation: bool,
                #[arg(long = "cascade-dependencies")]
                cascade_dependencies: bool,
            },
            /// RPC-206: permanently remove soft-deleted items from a work unit.
            #[command(
                name = "compact-work-unit",
                about = "Permanently remove all soft-deleted items from a work unit"
            )]
            CompactWorkUnit {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
            },
            /// RPC-255: reorder a work unit within its status column.
            #[command(
                name = "prioritize-work-unit",
                about = "Reorder a work unit within its status column"
            )]
            PrioritizeWorkUnit {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(long, value_name = "POSITION", allow_hyphen_values = true)]
                position: Option<String>,
                #[arg(long, value_name = "workUnitId")]
                before: Option<String>,
                #[arg(long, value_name = "workUnitId")]
                after: Option<String>,
            },
            /// RPC-284: rebuild work-unit state arrays and bidirectional dependency links.
            #[command(
                name = "repair-work-units",
                about = "Repair work unit state arrays and bidirectional dependency links"
            )]
            RepairWorkUnits {
                #[arg(long = "dry-run")]
                dry_run: bool,
            },
            /// RPC-264: record an iteration increment on a work unit.
            #[command(
                name = "record-iteration",
                about = "Record an iteration increment on a work unit"
            )]
            RecordIteration {
                #[arg(value_name = "name", allow_hyphen_values = true)]
                name: String,
                #[arg(long, value_name = "DATE")]
                start: Option<String>,
                #[arg(long, value_name = "DATE")]
                end: Option<String>,
            },
            /// RPC-229: export all work units to a file.
            #[command(
                name = "export-work-units",
                about = "Export all work units to a JSON file"
            )]
            ExportWorkUnits {
                #[arg(value_name = "format")]
                format: String,
                #[arg(value_name = "output")]
                output: String,
                #[arg(long, value_name = "STATUS")]
                status: Option<String>,
            },
            /// RPC-228: export a work unit's Example Map to a JSON file.
            #[command(
                name = "export-example-map",
                about = "Export a work unit's Example Map to a JSON file"
            )]
            ExportExampleMap {
                #[arg(value_name = "workUnitId")]
                work_unit_id: String,
                #[arg(value_name = "file")]
                file: String,
            },
            /// RPC-227: export the dependency graph to mermaid or JSON.
            #[command(
                name = "export-dependencies",
                about = "Export the work-unit dependency graph to mermaid or JSON"
            )]
            ExportDependencies {
                #[arg(value_name = "format")]
                format: String,
                #[arg(value_name = "output")]
                output: String,
            },
            // Batch 13 (2026-06-12) — foundation mutation commands
            /// RPC-173: add a capability to foundation.json / .draft.
            #[command(
                name = "add-capability",
                about = "Add a capability to foundation.json or foundation.json.draft"
            )]
            AddCapability {
                #[arg(value_name = "name", allow_hyphen_values = true)]
                name: String,
                #[arg(value_name = "description", allow_hyphen_values = true)]
                description: String,
            },
            /// RPC-269: remove a capability from foundation.json / .draft.
            #[command(
                name = "remove-capability",
                about = "Remove a capability from foundation.json or foundation.json.draft"
            )]
            RemoveCapability {
                #[arg(value_name = "name", allow_hyphen_values = true)]
                name: String,
            },
            /// RPC-186: add a user persona to foundation.json / .draft.
            #[command(
                name = "add-persona",
                about = "Add a user persona to foundation.json or foundation.json.draft"
            )]
            AddPersona {
                #[arg(value_name = "name", allow_hyphen_values = true)]
                name: String,
                #[arg(value_name = "description", allow_hyphen_values = true)]
                description: String,
                #[arg(long = "goal", value_name = "GOAL")]
                goal: Vec<String>,
            },
            /// RPC-277: remove a persona from foundation.json / .draft.
            #[command(
                name = "remove-persona",
                about = "Remove a persona from foundation.json or foundation.json.draft"
            )]
            RemovePersona {
                #[arg(value_name = "name", allow_hyphen_values = true)]
                name: String,
            },
        }
    };
}
