use crate::{dispatch, later_changes, source, TestResult};
use quote::ToTokens;
use syn::visit_mut::VisitMut;

#[test]
fn ordered_command_and_argument_declarations_retain_their_frozen_contract() -> TestResult {
    dispatch::verify_resolver_controls()?;
    // @step Given the frozen original parser and all 169 ordered command variants
    let mut expected = source::declarations(&source::original()?)?;
    // @step When the actual maintained command declarations are resolved
    let mut actual = source::declarations(&source::actual()?)?;
    source::strip_added_commands(&mut actual);
    source::strip_declaration_additions(&mut expected);
    source::strip_declaration_additions(&mut actual);
    for items in [&mut expected, &mut actual] {
        source::strip_hyphen_values(items);
        source::strip_approved_help(items);
    }
    let variants = expected.iter().find_map(|item| match item {
        syn::Item::Enum(item) => Some(item.variants.len()),
        _ => None,
    });
    assert_eq!(variants, Some(169));
    source::strip_added_commands(&mut expected);
    // @step Then all original tokens and order remain with only the approved coverage options added
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(&expected) {
        assert_eq!(source::tokens(actual), source::tokens(expected));
    }
    let original = syn::parse_file(&source::original()?)?;
    let maintained = syn::parse_file(&source::actual()?)?;
    let header = |file: &syn::File| {
        file.items
            .iter()
            .filter(|item| !source::is_added_module(item))
            .filter_map(|item| match item {
                syn::Item::Mod(item) => Some(source::tokens(item)),
                syn::Item::Use(item) => Some(source::tokens(item)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(header(&maintained), header(&original));
    let attributes = |file: &syn::File| file.attrs.iter().map(source::tokens).collect::<Vec<_>>();
    assert_eq!(attributes(&maintained), attributes(&original));
    Ok(())
}

fn verify_invocation_attribute_guard() -> TestResult {
    let definition = "macro_rules! route_finish {
        ($command:expr, $workspace:expr, $forward:ident; $($collected:tt)*) => {
            match $command { $($collected)* }
        };
    }";
    let invocation = "route_finish!(cli.cmd, cli.workspace, forward;)";
    let mut bare: syn::Expr = syn::parse_str(invocation)?;
    let mut resolver = ResolveDispatch {
        source: definition,
        error: None,
    };
    resolver.visit_expr_mut(&mut bare);
    assert_eq!(resolver.error, None);
    assert!(
        matches!(bare, syn::Expr::Match(_)),
        "unattributed entry must resolve"
    );
    for attribute in [
        "#[cfg(any())]",
        "#[cfg_attr(all(), cfg(any()))]",
        "#[allow(dead_code)]",
    ] {
        let mut attributed: syn::Expr = syn::parse_str(&format!("{attribute} {invocation}"))?;
        let original = source::tokens(&attributed);
        let mut resolver = ResolveDispatch {
            source: definition,
            error: None,
        };
        resolver.visit_expr_mut(&mut attributed);
        assert_eq!(
            resolver.error.as_deref(),
            Some("attributes on dispatch entry cannot be discarded")
        );
        assert_eq!(
            source::tokens(&attributed),
            original,
            "attributed entry must not expand or lose attributes"
        );
    }
    Ok(())
}

struct ResolveDispatch<'source> {
    source: &'source str,
    error: Option<String>,
}

impl VisitMut for ResolveDispatch<'_> {
    fn visit_expr_mut(&mut self, expression: &mut syn::Expr) {
        if let syn::Expr::Macro(invocation) = expression {
            if invocation
                .mac
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident.to_string().starts_with("route_"))
            {
                if !invocation.attrs.is_empty() {
                    self.error = Some("attributes on dispatch entry cannot be discarded".into());
                    return;
                }
                match dispatch::expand(self.source, &invocation.mac) {
                    Ok(expanded) => *expression = expanded,
                    Err(error) => self.error = Some(error.to_string()),
                }
            }
        }
        syn::visit_mut::visit_expr_mut(self, expression);
    }

    fn visit_expr_match_mut(&mut self, expression: &mut syn::ExprMatch) {
        if source::tokens(&expression.expr) == "cli . cmd" {
            for arm in &mut expression.arms {
                let pattern = arm.pat.to_token_stream().to_string();
                let command = [
                    "LinkCoverage",
                    "UnlinkCoverage",
                    "GenerateCoverage",
                    "ShowCoverage",
                ]
                .into_iter()
                .find(|command| pattern.contains(&format!("Mode :: {command}")));
                if let Some(command) = command {
                    let mut strip = source::StripDispatchFields {
                        fields: source::additions(command),
                    };
                    strip.visit_pat_mut(&mut arm.pat);
                    strip.visit_expr_mut(&mut arm.body);
                }
            }
        }
        syn::visit_mut::visit_expr_match_mut(self, expression);
    }
}

#[test]
fn dispatch_startup_errors_and_help_retain_their_frozen_contract() -> TestResult {
    verify_invocation_attribute_guard()?;
    // @step Given the frozen ordered dispatch arms startup shell error helpers and help interception
    let original = source::original()?;
    let maintained = source::actual()?;
    // @step When the actual maintained registration composition is resolved
    let compared = dispatch_contract(&original, &maintained);
    // @step Then every original destination argument expression and output contract remains unchanged
    compared?;
    source::assert_original_help()?;
    Ok(())
}

/// The frozen dispatch, startup and help contract: every frozen function of
/// `original` against `maintained` once the recorded later changes
/// (source.rs, later_changes.rs) are applied. BUG-583: a difference is an
/// error naming the function or help entry, not a panic.
pub fn dispatch_contract(original: &str, maintained: &str) -> Result<(), String> {
    let parse = |text: &str| source::functions(text).map_err(|error| error.to_string());
    let mut expected = parse(original)?;
    let mut actual = parse(maintained)?;
    later_changes::frozen_view(&mut expected, &mut actual)?;
    let mut resolver = ResolveDispatch {
        source: maintained,
        error: None,
    };
    for function in &mut actual {
        resolver.visit_item_fn_mut(function);
        source::StripAddedArms.visit_item_fn_mut(function);
        source::StripAddedStatements.visit_item_fn_mut(function);
        source::StripAddedTokenRouting.visit_item_fn_mut(function);
        source::StripAddedTlsRouting.visit_item_fn_mut(function);
        later_changes::StripAddedSectionRouting.visit_item_fn_mut(function);
    }
    if let Some(error) = resolver.error {
        return Err(error);
    }
    if actual.len() != expected.len() {
        return Err(format!(
            "{} maintained functions against {} frozen ones",
            actual.len(),
            expected.len()
        ));
    }
    for mut expected in expected {
        let function = actual
            .iter()
            .find(|function| function.sig.ident == expected.sig.ident)
            .ok_or_else(|| format!("original function {} missing", expected.sig.ident))?;
        let mut normalize = ResolveDispatch {
            source: original,
            error: None,
        };
        normalize.visit_item_fn_mut(&mut expected);
        // AUDIT-003: the arms of removed commands leave the frozen original.
        source::StripAddedArms.visit_item_fn_mut(&mut expected);
        let (maintained, frozen) = (source::tokens(function), source::tokens(&expected));
        if maintained != frozen {
            return Err(format!(
                "function {} changed: {}",
                expected.sig.ident,
                first_difference(&maintained, &frozen)
            ));
        }
    }
    let constants = |text: &str| -> Result<Vec<String>, String> {
        Ok(syn::parse_file(text)
            .map_err(|error| error.to_string())?
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Const(item) => Some(source::tokens(item)),
                _ => None,
            })
            .collect())
    };
    if constants(maintained)? != constants(original)? {
        return Err("top-level constants changed".into());
    }
    Ok(())
}

/// The maintained and frozen text around their first difference.
fn first_difference(maintained: &str, frozen: &str) -> String {
    let at = maintained
        .char_indices()
        .zip(frozen.chars())
        .find(|((_, left), right)| left != right)
        .map_or(maintained.len().min(frozen.len()), |((at, _), _)| at);
    let window = |text: &str| -> String {
        text.get(at..)
            .unwrap_or_default()
            .chars()
            .take(160)
            .collect()
    };
    format!(
        "maintained {:?} where frozen has {:?}",
        window(maintained),
        window(frozen)
    )
}

/// One unrecorded change applied to the maintained CLI source text.
fn mutated(maintained: &str, from: &str, to: &str) -> Result<String, String> {
    if !maintained.contains(from) {
        return Err(format!("maintained CLI source no longer contains {from:?}"));
    }
    Ok(maintained.replacen(from, to, 1))
}

#[test]
fn a_later_change_the_contract_does_not_record_still_fails_it() -> TestResult {
    // @step Given the maintained CLI source with one unrecorded change: an added function, a statement added to main, or a help interception entry pointing at another config
    let original = source::original()?;
    let maintained = source::actual()?;
    let changes = [
        (
            format!("{maintained}\nfn unrecorded_helper() {{}}\n"),
            "unrecorded_helper",
        ),
        (
            mutated(
                &maintained,
                "    let Some(target) = command_hook_target(&matches) else {",
                "    eprintln!(\"unrecorded\");\n    let Some(target) = command_hook_target(&matches) else {",
            )?,
            "function main",
        ),
        (
            mutated(
                &maintained,
                "\"list-epics\" => Some(&configs::list_epics::CONFIG),",
                "\"list-epics\" => Some(&configs::list_tags::CONFIG),",
            )?,
            "list-epics: config list_epics",
        ),
    ];
    for (changed, named) in changes {
        // @step When the frozen dispatch, startup and help contract compares it with the frozen original
        let compared = dispatch_contract(&original, &changed);
        // @step Then the comparison fails and names the function or the help entry that changed
        match compared {
            Err(error) => assert!(error.contains(named), "{error:?} must name {named:?}"),
            Ok(()) => return Err(format!("an unrecorded change naming {named:?} passed").into()),
        }
    }
    Ok(())
}

#[test]
fn maintained_cli_integration_sources_remain_below_the_physical_ceiling() -> TestResult {
    // @step Given the CLI root four coverage bridges and reachable registration sources
    let mut inputs = codelet_test_helpers::source_composition::resolve(&source::root())?.inputs;
    for name in [
        "link_coverage",
        "unlink_coverage",
        "generate_coverage",
        "show_coverage",
    ] {
        inputs.push(source::root().with_file_name(format!("{name}.rs")));
    }
    // @step When their raw maintained physical source lines are counted
    let oversized = inputs
        .iter()
        .map(|path| std::fs::read_to_string(path).map(|text| (path, text.lines().count())))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|(_, lines)| *lines >= 300)
        .map(|(path, lines)| format!("{}: {lines}", path.display()))
        .collect::<Vec<_>>();
    // @step Then every targeted maintained source has fewer than 300 lines without hiding oversized source
    assert!(
        oversized.is_empty(),
        "maintained physical sources exceed ceiling: {oversized:?}"
    );
    Ok(())
}
