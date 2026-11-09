# AST research for CLI-016 (2026-11-07)

## Help configs (CommandHelpConfig statics)
```
159
```

## Help printed by the binary (support_03.rs)
```
rust/fspec/src/cli_registration/support_03.rs:47:1:fn ts_help_config(sub: &str) -> Option<&'static codelet_fspec_core::help::CommandHelpConfig> {

```

## clap root
```
rust/fspec/src/cli_registration/cli.rs:15:1:struct Cli {

```

## Reminder text with fspec mentions
```
rust/fspec-core/src/commands/update_work_unit_status/reminders/specifying.rs:11:1:const SPEC_HEAD: &str = r#"Work unit __ID__ is now in SPECIFYING status.
rust/fspec-core/src/commands/update_work_unit_status/reminders/specifying.rs:35:1:const SPEC_MID: &str = r#"Configuration:
rust/fspec-core/src/commands/update_work_unit_status/reminders/specifying.rs:41:1:const SPEC_TAIL: &str = r#"Research results can be attached to work units for Example Mapping context.
rust/fspec-core/src/commands/update_work_unit_status/reminders/specifying.rs:57:1:const PERPLEXITY_EXAMPLE: &str = r#"{
rust/fspec-core/src/commands/update_work_unit_status/reminders/specifying.rs:65:1:const JIRA_EXAMPLE: &str = r#"{
rust/fspec-core/src/commands/update_work_unit_status/reminders/specifying.rs:75:1:const CONFLUENCE_EXAMPLE: &str = r#"{
rust/fspec-core/src/commands/update_work_unit_status/reminders/specifying.rs:85:1:const STAKEHOLDER_EXAMPLE: &str = r#"{
rust/fspec-core/src/commands/update_work_unit_status/reminders/status.rs:11:1:const TESTING: &str = r#"Work unit __ID__ is now in TESTING status.
rust/fspec-core/src/commands/update_work_unit_status/reminders/status.rs:68:1:const VALIDATING: &str = r#"Work unit __ID__ is now in VALIDATING status.
rust/fspec-core/src/commands/update_work_unit_status/reminders/status.rs:95:1:const BLOCKED: &str = r#"Work unit __ID__ is now in BLOCKED status.
rust/fspec-core/src/commands/update_work_unit_status/reminders/status.rs:111:1:const EVENT_STORM: &str = r#"EVENT STORM ASSESSMENT - Domain Complexity Check
rust/fspec-core/src/commands/update_work_unit_status/reminders/status.rs:148:1:const VIRTUAL_HOOKS: &str = r#"Work unit __ID__ is moving to TESTING phase.
rust/fspec-core/src/commands/update_work_unit_status/reminders/status.rs:186:1:const DONE_REVIEW: &str = r#"QUALITY CHECK OPPORTUNITY
rust/fspec-core/src/commands/update_work_unit_status/reminders/enhanced.rs:88:1:const IMPLEMENTING_BODY: &str = r#"⚠️ COMMON FAILURE MODE: Code that exists but isn't connected.

```
