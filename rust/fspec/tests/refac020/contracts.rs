use quote::ToTokens;
use syn::visit_mut::VisitMut;

use crate::{dispatch, source, TestResult};

#[test]
fn ordered_command_and_argument_declarations_retain_their_frozen_contract() -> TestResult {
    dispatch::verify_resolver_controls()?;
    // @step Given the frozen original parser and all 169 ordered command variants
    let mut expected = source::declarations(&source::original()?)?;
    // @step When the actual maintained command declarations are resolved
    let mut actual = source::declarations(&source::actual()?)?;
    source::strip_declaration_additions(&mut expected);
    source::strip_declaration_additions(&mut actual);
    let variants = expected.iter().find_map(|item| match item {
        syn::Item::Enum(item) => Some(item.variants.len()),
        _ => None,
    });
    assert_eq!(variants, Some(169));
    // @step Then all original tokens and order remain with only the approved coverage options added
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(&expected) {
        assert_eq!(source::tokens(actual), source::tokens(expected));
    }
    let original = syn::parse_file(&source::original()?)?;
    let maintained = syn::parse_file(&source::actual()?)?;
    let header = |file: &syn::File| {
        file.items.iter().filter_map(|item| match item {
            syn::Item::Mod(item) => Some(source::tokens(item)),
            syn::Item::Use(item) => Some(source::tokens(item)),
            _ => None,
        }).collect::<Vec<_>>()
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
    let mut resolver = ResolveDispatch { source: definition, error: None };
    resolver.visit_expr_mut(&mut bare);
    assert_eq!(resolver.error, None);
    assert!(matches!(bare, syn::Expr::Match(_)), "unattributed entry must resolve");
    for attribute in ["#[cfg(any())]", "#[cfg_attr(all(), cfg(any()))]", "#[allow(dead_code)]"] {
        let mut attributed: syn::Expr = syn::parse_str(&format!("{attribute} {invocation}"))?;
        let original = source::tokens(&attributed);
        let mut resolver = ResolveDispatch { source: definition, error: None };
        resolver.visit_expr_mut(&mut attributed);
        assert_eq!(resolver.error.as_deref(), Some("attributes on dispatch entry cannot be discarded"));
        assert_eq!(source::tokens(&attributed), original, "attributed entry must not expand or lose attributes");
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
            if invocation.mac.path.segments.last().is_some_and(|segment| {
                segment.ident.to_string().starts_with("route_")
            }) {
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
                let command = ["LinkCoverage", "UnlinkCoverage", "GenerateCoverage", "ShowCoverage"]
                    .into_iter().find(|command| pattern.contains(&format!("Mode :: {command}")));
                if let Some(command) = command {
                    let mut strip = source::StripDispatchFields { fields: source::additions(command) };
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
    let expected = source::functions(&original)?;
    let mut actual = source::functions(&maintained)?;
    // @step When the actual maintained registration composition is resolved
    let mut resolver = ResolveDispatch { source: &maintained, error: None };
    for function in &mut actual {
        resolver.visit_item_fn_mut(function);
    }
    assert_eq!(resolver.error, None);
    // @step Then every original destination argument expression and output contract remains unchanged
    assert_eq!(actual.len(), expected.len());
    for mut expected in expected {
        let function = actual.iter().find(|function| function.sig.ident == expected.sig.ident)
            .ok_or("original function missing")?;
        let mut normalize = ResolveDispatch { source: &original, error: None };
        normalize.visit_item_fn_mut(&mut expected);
        assert_eq!(source::tokens(function), source::tokens(&expected), "function {}", expected.sig.ident);
    }
    let constants = |text: &str| -> Result<Vec<String>, syn::Error> {
        Ok(syn::parse_file(text)?.items.iter().filter_map(|item| match item {
            syn::Item::Const(item) => Some(source::tokens(item)),
            _ => None,
        }).collect())
    };
    assert_eq!(constants(&maintained)?, constants(&original)?);
    source::assert_original_help()?;
    Ok(())
}

#[test]
fn maintained_cli_integration_sources_remain_below_the_physical_ceiling() -> TestResult {
    // @step Given the CLI root four coverage bridges and reachable registration sources
    let mut inputs = codelet_test_helpers::source_composition::resolve(&source::root())?.inputs;
    for name in ["link_coverage", "unlink_coverage", "generate_coverage", "show_coverage"] {
        inputs.push(source::root().with_file_name(format!("{name}.rs")));
    }
    // @step When their raw maintained physical source lines are counted
    let oversized = inputs.iter().map(|path| {
        std::fs::read_to_string(path).map(|text| (path, text.lines().count()))
    }).collect::<Result<Vec<_>, _>>()?.into_iter().filter(|(_, lines)| *lines >= 300)
        .map(|(path, lines)| format!("{}: {lines}", path.display())).collect::<Vec<_>>();
    // @step Then every targeted maintained source has fewer than 300 lines without hiding oversized source
    assert!(oversized.is_empty(), "maintained physical sources exceed ceiling: {oversized:?}");
    Ok(())
}
