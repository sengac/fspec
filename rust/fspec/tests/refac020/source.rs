use std::path::PathBuf;
use std::process::Command;

use quote::ToTokens;
use sha2::{Digest, Sha256};
use syn::visit_mut::VisitMut;

use crate::TestResult;

pub const MAIN_BLOB: &str = "2962c98e7d66d917026aa6feb660914f8be73ff7";
pub const MAIN_SHA: &str = "e20bd03b59b2801df601a18d491344765b6da550e9da141cde6af4bd5679559f";

pub fn original() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["cat-file", "blob", MAIN_BLOB])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    if !output.status.success() {
        return Err(format!("frozen original CLI blob unavailable: {}", MAIN_BLOB).into());
    }
    assert_eq!(hex::encode(Sha256::digest(&output.stdout)), MAIN_SHA);
    Ok(crate::later_changes::cli020_renamed(String::from_utf8(
        output.stdout,
    )?)?)
}

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs")
}

pub fn actual() -> Result<String, Box<dyn std::error::Error>> {
    Ok(codelet_test_helpers::source_composition::read_source(
        &root(),
    )?)
}

pub fn declarations(source: &str) -> Result<Vec<syn::Item>, syn::Error> {
    Ok(syn::parse_file(source)?
        .items
        .into_iter()
        .filter(|item| match item {
            syn::Item::Struct(item) => item.ident == "Cli",
            syn::Item::Enum(item) => item.ident == "Mode",
            _ => false,
        })
        .collect())
}

pub fn additions(command: &str) -> &'static [&'static str] {
    match command {
        // BUG-502: link-coverage --replace.
        "LinkCoverage" | "link-coverage" => {
            &["test_mapping_index", "expected_scenario_hash", "replace"]
        }
        "UnlinkCoverage" | "unlink-coverage" => &["test_mapping_index", "expected_scenario_hash"],
        // RPC-456: the daemon serves TLS with --tls-cert/--tls-key.
        "Daemon" | "daemon" => ADDED_TLS_FIELDS,
        "GenerateCoverage" | "generate-coverage" => &["feature_name"],
        "ShowCoverage" | "show-coverage" => &["mapping_json"],
        // BUG-537 (recorded by BUG-583): show-foundation --section.
        "ShowFoundation" | "show-foundation" => ADDED_SECTION_FIELDS,
        // BUG-285: the root `Cli` struct / `fspec` command gained the hidden
        // top-level `--sync-version` flag.
        "Cli" | "fspec" => ADDED_CLI_FIELDS,
        // BUG-429: `client` and `status` accept a bearer `--token`.
        "Client" | "Status" | "client" | "status" => ADDED_TOKEN_FIELDS,
        _ => &[],
    }
}

/// BUG-537: the `--section` field `show-foundation` gained beside its
/// positional (routed as `section.or(section_flag)`, see
/// `later_changes::StripAddedSectionRouting`).
pub const ADDED_SECTION_FIELDS: &[&str] = &["section_flag"];

/// RPC-456: the TLS fields `daemon` gained; BUG-595: `--tls-name`.
pub const ADDED_TLS_FIELDS: &[&str] = &["tls_cert", "tls_key", "tls_name"];

/// RPC-456: arguments whose help text changed after the freeze (the daemon's
/// `--bind` now documents TLS for non-loopback addresses); their doc
/// attributes and compiled help are left out of the comparison.
pub const APPROVED_HELP_CHANGES: &[(&str, &str)] = &[("Daemon", "bind")];

/// BUG-496 / BUG-521: free-text arguments accept values that start with
/// dashes. `allow_hyphen_values` is an approved change on every argument, so
/// the comparison ignores it.
pub fn strip_hyphen_values(items: &mut [syn::Item]) {
    StripHyphenValues.visit_items(items);
}

struct StripHyphenValues;

impl StripHyphenValues {
    fn visit_items(&mut self, items: &mut [syn::Item]) {
        for item in items {
            self.visit_item_mut(item);
        }
    }
}

impl VisitMut for StripHyphenValues {
    fn visit_attribute_mut(&mut self, attribute: &mut syn::Attribute) {
        if attribute.path().is_ident("arg") {
            if let syn::Meta::List(list) = &mut attribute.meta {
                let kept: Vec<String> = split_top_level(&list.tokens.to_string())
                    .into_iter()
                    .filter(|part| part.trim() != "allow_hyphen_values = true")
                    .collect();
                if let Ok(tokens) = kept.join(" , ").parse() {
                    list.tokens = tokens;
                }
            }
        }
    }
}

/// Split attribute tokens on commas that are not inside brackets or quotes.
fn split_top_level(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut quoted = false;
    let mut current = String::new();
    for character in text.chars() {
        match character {
            '"' => quoted = !quoted,
            '(' | '[' | '{' if !quoted => depth += 1,
            ')' | ']' | '}' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                parts.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    if !current.trim().is_empty() {
        parts.push(current);
    }
    parts
}

/// RPC-456: drops the doc attributes of [`APPROVED_HELP_CHANGES`] fields.
pub fn strip_approved_help(items: &mut [syn::Item]) {
    for item in items {
        if let syn::Item::Enum(item) = item {
            for variant in &mut item.variants {
                if let syn::Fields::Named(fields) = &mut variant.fields {
                    for field in &mut fields.named {
                        let name = field
                            .ident
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default();
                        let approved = APPROVED_HELP_CHANGES
                            .iter()
                            .any(|(command, arg)| variant.ident == command && name == *arg);
                        if approved {
                            field
                                .attrs
                                .retain(|attribute| !attribute.path().is_ident("doc"));
                        }
                    }
                }
            }
        }
    }
}

/// RPC-456: drops the TLS bindings and the `tls_cert . zip (tls_key)`
/// argument from the daemon dispatch arm.
pub struct StripAddedTlsRouting;

impl VisitMut for StripAddedTlsRouting {
    fn visit_pat_struct_mut(&mut self, pattern: &mut syn::PatStruct) {
        let variant = pattern
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        if variant.as_deref() == Some("Daemon") {
            pattern.fields = pattern
                .fields
                .iter()
                .filter(|field| !ADDED_TLS_FIELDS.contains(&tokens(&field.member).as_str()))
                .cloned()
                .collect();
        }
        syn::visit_mut::visit_pat_struct_mut(self, pattern);
    }

    fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
        call.args = call
            .args
            .iter()
            .filter(|argument| {
                // BUG-595: the daemon dispatch also passes `tls_name`.
                !["tls_cert . zip (tls_key)", "tls_name"].contains(&tokens(*argument).as_str())
            })
            .cloned()
            .collect();
        syn::visit_mut::visit_expr_call_mut(self, call);
    }
}

/// BUG-429 / BUG-457: the `--token` field `client` and `status` gained.
pub const ADDED_TOKEN_FIELDS: &[&str] = &["token"];

/// BUG-457: drops the BUG-429 `token` bindings and call arguments from the
/// `client` / `status` dispatch arms (`Mode::Client { connect, token, mcp }`
/// → `client::run(connect, token)`), so the frozen routing still compares.
pub struct StripAddedTokenRouting;

impl VisitMut for StripAddedTokenRouting {
    fn visit_pat_struct_mut(&mut self, pattern: &mut syn::PatStruct) {
        let variant = pattern
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        let has_token = pattern
            .fields
            .iter()
            .any(|field| ADDED_TOKEN_FIELDS.contains(&tokens(&field.member).as_str()));
        if has_token && matches!(variant.as_deref(), Some("Client" | "Status")) {
            pattern.fields = pattern
                .fields
                .iter()
                .filter(|field| !ADDED_TOKEN_FIELDS.contains(&tokens(&field.member).as_str()))
                .cloned()
                .collect();
        }
        syn::visit_mut::visit_pat_struct_mut(self, pattern);
    }

    fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
        let callee = tokens(&call.func);
        if ["client :: run", "client :: run_mcp", "status :: run"].contains(&callee.as_str()) {
            call.args = call
                .args
                .iter()
                .filter(|argument| !ADDED_TOKEN_FIELDS.contains(&tokens(*argument).as_str()))
                .cloned()
                .collect();
        }
        syn::visit_mut::visit_expr_call_mut(self, call);
    }
}

/// Commands added after the REFAC-020 freeze, each specified by its own card
/// (RPC-447: `fspec daemons`; RPC-457: `fspec workspaces`; BUG-507:
/// `fspec update-feature-text`; CLI-019: `fspec context`). They are compared
/// by their own features, so the frozen-contract comparisons drop them from
/// the maintained CLI.
pub const ADDED_COMMANDS: &[&str] = &[
    "Daemons",
    "Workspaces",
    "UpdateFeatureText",
    "Audit",
    "Context",
    // BUG-633: remove-assumption / restore-assumption.
    "RemoveAssumption",
    "RestoreAssumption",
];
/// The command names of [`ADDED_COMMANDS`], for string-matched arms such as
/// the help interception.
pub const ADDED_COMMAND_NAMES: &[&str] = &[
    "update-feature-text",
    "audit",
    "context",
    "remove-assumption",
    "restore-assumption",
];
/// Modules main.rs declares for those added commands.
pub const ADDED_MODULES: &[&str] = &[
    "audit",
    // CLI-019: the fspec context bridge.
    "context",
    "daemon_directory",
    "daemons_cmd",
    // BUG-633: the remove-assumption / restore-assumption bridges.
    "remove_assumption",
    "restore_assumption",
    // CLI-016 (recorded by BUG-583): the guidance parse contract tests.
    "guidance_contract_tests",
    // BUG-566 (recorded by BUG-583): the TypeScript-leftover tests.
    "help_and_step_leftovers_tests",
    // BUG-590: every add-hook mention names an event add-hook accepts.
    "bug590_add_hook_event_guidance_tests",
    "sync_version",
    "update_feature_text",
    "workspaces_cmd",
];
/// `use` items main.rs gained after the freeze (HOOK-018, recorded by
/// BUG-583: the command hooks run around every fspec command).
pub const ADDED_USES: &[&str] = &["use codelet_fspec_core :: command_hooks :: { format_hook_output , is_hooked_command , run_command_hooks , HookPhase , PhaseOutcome , } ;"];
/// Commands removed after the freeze (AUDIT-003: `fspec audit` replaced
/// `audit-coverage` and `check`; CLI-019: `fspec context` replaced
/// `show-acceptance-criteria`, `get-scenarios` and `search-scenarios`, product
/// owner 2026-11-08): their variants, string-matched arms and modules are
/// dropped from the frozen original before comparing.
pub const REMOVED_COMMANDS: &[&str] = &[
    "AuditCoverage",
    "Check",
    "ShowAcceptanceCriteria",
    "GetScenarios",
    "SearchScenarios",
];
pub const REMOVED_COMMAND_NAMES: &[&str] = &[
    "audit-coverage",
    "check",
    "show-acceptance-criteria",
    "get-scenarios",
    "search-scenarios",
];
pub const REMOVED_MODULES: &[&str] = &[
    "audit_coverage",
    "check",
    "show_acceptance_criteria",
    "get_scenarios",
    "search_scenarios",
];
/// Commands whose arguments a later card replaced; like added and removed
/// commands, their variants and dispatch arms are compared by their own
/// features, so both sides drop them. CLI-017: `list-work-units` takes the
/// board-query flags through a flattened `list_work_units::Flags` (product
/// owner, 2026-11-07: extend list-work-units, no second command).
pub const CHANGED_COMMANDS: &[&str] = &["ListWorkUnits"];
/// Root `Cli` fields added after the freeze (BUG-285: `--sync-version`), each
/// routed by one early-return statement in `main` (see [`StripAddedStatements`]).
pub const ADDED_CLI_FIELDS: &[&str] = &["sync_version"];

/// Drops the statements that route [`ADDED_CLI_FIELDS`] (`cli.<field>`), so
/// the frozen-contract comparison of `main` sees only the original body.
pub struct StripAddedStatements;

impl syn::visit_mut::VisitMut for StripAddedStatements {
    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        block.stmts.retain(|statement| {
            let statement = tokens(statement);
            // BUG-619: main installs the ring crypto provider first.
            statement != "codelet_rpc_server :: install_crypto_provider () ;"
                && !ADDED_CLI_FIELDS
                    .iter()
                    .any(|field| statement.contains(&format!("cli . {field}")))
        });
        syn::visit_mut::visit_block_mut(self, block);
    }
}

pub fn is_added_module(item: &syn::Item) -> bool {
    matches!(item, syn::Item::Mod(module) if ADDED_MODULES.contains(&module.ident.to_string().as_str())
        || REMOVED_MODULES.contains(&module.ident.to_string().as_str()))
        || matches!(item, syn::Item::Use(_) if ADDED_USES.contains(&tokens(item).as_str()))
}

/// Remove [`ADDED_COMMANDS`] (and [`REMOVED_COMMANDS`] and
/// [`CHANGED_COMMANDS`]) variants from the `Mode` enum.
pub fn strip_added_commands(items: &mut [syn::Item]) {
    for item in items {
        if let syn::Item::Enum(item) = item {
            let trailing = item.variants.trailing_punct();
            let mut kept: syn::punctuated::Punctuated<syn::Variant, syn::Token![,]> = item
                .variants
                .iter()
                .filter(|variant| {
                    let name = variant.ident.to_string();
                    !ADDED_COMMANDS.contains(&name.as_str())
                        && !REMOVED_COMMANDS.contains(&name.as_str())
                        && !CHANGED_COMMANDS.contains(&name.as_str())
                })
                .cloned()
                .collect();
            if trailing && !kept.empty_or_trailing() {
                kept.push_punct(Default::default());
            }
            item.variants = kept;
        }
    }
}

/// Whether a dispatch match arm routes one of [`ADDED_COMMANDS`].
pub fn is_added_command_arm(arm: &syn::Arm) -> bool {
    let pattern = tokens(&arm.pat);
    ADDED_COMMANDS
        .iter()
        .chain(REMOVED_COMMANDS)
        .chain(CHANGED_COMMANDS)
        .any(|command| names_variant(&pattern, command))
        || ADDED_COMMAND_NAMES
            .iter()
            .chain(REMOVED_COMMAND_NAMES)
            .any(|name| pattern == format!("\"{name}\""))
}

/// Whether `pattern` names `Mode :: <command>` as a whole identifier (so
/// `Check` does not match `Checkpoint`).
fn names_variant(pattern: &str, command: &str) -> bool {
    let needle = format!("Mode :: {command}");
    pattern.match_indices(&needle).any(|(at, _)| {
        !pattern[at + needle.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

/// Drops dispatch arms for commands added after the freeze.
pub struct StripAddedArms;

impl syn::visit_mut::VisitMut for StripAddedArms {
    fn visit_expr_match_mut(&mut self, node: &mut syn::ExprMatch) {
        node.arms.retain(|arm| !is_added_command_arm(arm));
        syn::visit_mut::visit_expr_match_mut(self, node);
    }
}

pub fn strip_declaration_additions(items: &mut [syn::Item]) {
    for item in items.iter_mut() {
        if let syn::Item::Struct(item) = item {
            let added = additions(&item.ident.to_string());
            if let syn::Fields::Named(fields) = &mut item.fields {
                fields.named = fields
                    .named
                    .iter()
                    .filter(|field| {
                        !field
                            .ident
                            .as_ref()
                            .is_some_and(|ident| added.contains(&ident.to_string().as_str()))
                    })
                    .cloned()
                    .collect();
            }
        }
    }
    for item in items {
        if let syn::Item::Enum(item) = item {
            for variant in &mut item.variants {
                if let syn::Fields::Named(fields) = &mut variant.fields {
                    fields.named = fields
                        .named
                        .iter()
                        .filter(|field| {
                            !additions(&variant.ident.to_string()).contains(
                                &field
                                    .ident
                                    .as_ref()
                                    .map(ToString::to_string)
                                    .unwrap_or_default()
                                    .as_str(),
                            )
                        })
                        .cloned()
                        .collect();
                }
            }
        }
    }
}

pub fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

pub fn functions(source: &str) -> Result<Vec<syn::ItemFn>, syn::Error> {
    Ok(syn::parse_file(source)?
        .items
        .into_iter()
        .filter_map(|item| match item {
            syn::Item::Fn(item) => Some(item),
            _ => None,
        })
        .collect())
}

pub struct StripDispatchFields {
    pub fields: &'static [&'static str],
}

impl VisitMut for StripDispatchFields {
    fn visit_pat_struct_mut(&mut self, pattern: &mut syn::PatStruct) {
        pattern.fields = pattern
            .fields
            .iter()
            .filter(|field| !self.fields.contains(&tokens(&field.member).as_str()))
            .cloned()
            .collect();
        syn::visit_mut::visit_pat_struct_mut(self, pattern);
    }

    fn visit_expr_struct_mut(&mut self, expression: &mut syn::ExprStruct) {
        expression.fields = expression
            .fields
            .iter()
            .filter(|field| !self.fields.contains(&tokens(&field.member).as_str()))
            .cloned()
            .collect();
        syn::visit_mut::visit_expr_struct_mut(self, expression);
    }

    fn visit_expr_macro_mut(&mut self, expression: &mut syn::ExprMacro) {
        visit_forward_arguments(self, expression);
    }
}

/// Visit the argument expression of a `forward!(bridge, arguments)`
/// invocation, which syn leaves as unparsed macro tokens.
pub fn visit_forward_arguments(visitor: &mut impl VisitMut, expression: &mut syn::ExprMacro) {
    if expression.mac.path.is_ident("forward") {
        if let Ok(mut invocation) = syn::parse2::<Forward>(expression.mac.tokens.clone()) {
            visitor.visit_expr_mut(&mut invocation.arguments);
            let bridge = invocation.bridge;
            let arguments = invocation.arguments;
            expression.mac.tokens = quote::quote!(#bridge, #arguments);
        }
    }
}

struct Forward {
    bridge: syn::Path,
    arguments: syn::Expr,
}

impl syn::parse::Parse for Forward {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let bridge = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let arguments = input.parse()?;
        if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
        }
        Ok(Self { bridge, arguments })
    }
}

pub fn assert_original_help() -> TestResult {
    crate::help::assert_originals()
}
