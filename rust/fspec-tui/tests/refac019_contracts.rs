//! Feature: spec/features/preserve-contracts-while-decomposing-native-board-integration-surfaces.feature

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_test_helpers::source_composition::{read_source, resolve};
use quote::ToTokens;
use sha2::{Digest, Sha256};

mod common;
#[path = "refac019/composition.rs"]
mod composition;
#[path = "refac019/schema.rs"]
mod schema;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn contract(path: &str, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let source = read_source(&workspace().join(path))?;
    let file = syn::parse_file(&source)?;
    for item in file.items {
        let (header, members) = match item {
            syn::Item::Enum(mut item) if item.ident == name => {
                let members = item
                    .variants
                    .iter()
                    .map(|member| {
                        (
                            member.ident.to_string(),
                            member.to_token_stream().to_string(),
                        )
                    })
                    .collect::<Vec<_>>();
                item.variants.clear();
                (item.to_token_stream().to_string(), members)
            }
            syn::Item::Trait(mut item) if item.ident == name => {
                let members = item
                    .items
                    .iter()
                    .filter_map(|member| {
                        if let syn::TraitItem::Fn(method) = member {
                            Some((
                                method.sig.ident.to_string(),
                                member.to_token_stream().to_string(),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                item.items.clear();
                (item.to_token_stream().to_string(), members)
            }
            syn::Item::Impl(mut item)
                if item.trait_.as_ref().is_some_and(|(_, path, _)| {
                    path.segments
                        .last()
                        .is_some_and(|segment| segment.ident == name)
                }) =>
            {
                let members = item
                    .items
                    .iter()
                    .filter_map(|member| {
                        if let syn::ImplItem::Fn(method) = member {
                            Some((
                                method.sig.ident.to_string(),
                                member.to_token_stream().to_string(),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                item.items.clear();
                (item.to_token_stream().to_string(), members)
            }
            _ => continue,
        };
        let mut lines = vec![format!("header {:x}", Sha256::digest(header.as_bytes()))];
        lines.extend(
            members
                .into_iter()
                .map(|(name, tokens)| format!("{name} {:x}", Sha256::digest(tokens.as_bytes()))),
        );
        return Ok(lines.join("\n"));
    }
    Err(format!("contract {name} not found in {path}").into())
}

#[test]
fn ordered_public_and_implementation_contracts_retain_their_baseline() -> TestResult {
    // @step Given frozen contracts for Action backend and RPC declarations and implementations
    let surfaces = [
        ("actions", "fspec-tui/src/components/mod.rs", "Action"),
        ("backend", "fspec-tui/src/transport/mod.rs", "FspecBackend"),
        (
            "embedded",
            "fspec-tui/src/transport/embedded.rs",
            "FspecBackend",
        ),
        (
            "websocket",
            "fspec-tui/src/transport/websocket.rs",
            "FspecBackend",
        ),
        ("rpc_declaration", "rpc/src/lib.rs", "FspecService"),
        (
            "rpc_implementation",
            "rpc/src/lib.rs",
            "FspecServiceImplContract",
        ),
    ];
    // @step When their actual source composition is resolved
    for (label, path, name) in surfaces {
        let name = if name == "FspecServiceImplContract" {
            "FspecService"
        } else {
            name
        };
        let tokens = if label == "rpc_implementation" {
            let source = read_source(&workspace().join(path))?;
            let source = source.replacen("pub trait FspecService", "pub trait IgnoredService", 1);
            contract_text_impl(&source, name)?
        } else {
            contract(path, name)?
        };
        // @step Then ordered members signatures attributes and bodies match the baseline
        insta::assert_snapshot!(format!("refac019_{label}"), tokens);
    }
    Ok(())
}

fn contract_text_impl(source: &str, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    for item in syn::parse_file(source)?.items {
        if let syn::Item::Impl(mut item) = item {
            if item
                .trait_
                .as_ref()
                .is_some_and(|(_, path, _)| path.is_ident(name))
            {
                let methods = item
                    .items
                    .iter()
                    .filter_map(|member| {
                        if let syn::ImplItem::Fn(method) = member {
                            Some(format!(
                                "{} {:x}",
                                method.sig.ident,
                                Sha256::digest(member.to_token_stream().to_string().as_bytes())
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                item.items.clear();
                let header = format!(
                    "header {:x}",
                    Sha256::digest(item.to_token_stream().to_string().as_bytes())
                );
                return Ok(std::iter::once(header)
                    .chain(methods)
                    .collect::<Vec<_>>()
                    .join("\n"));
            }
        }
    }
    Err("RPC implementation missing".into())
}

#[tokio::test]
async fn backend_consumers_retain_object_safety_and_existing_defaults() -> TestResult {
    // @step Given an existing external backend implementation and both native backend types
    fn accepts_backend<Backend: FspecBackend>() {}
    accepts_backend::<EmbeddedFspecBackend>();
    accepts_backend::<WebSocketFspecBackend>();
    let backend: Arc<dyn FspecBackend> = Arc::new(common::MockBackend::new());
    // @step When they are consumed through the original public trait object
    assert!(backend.list_live_sessions().await?.is_empty());
    let source = syn::parse_file(&read_source(
        &workspace().join("fspec-tui/src/transport/mod.rs"),
    )?)?;
    // @step Then existing consumer compilation and default behavior remain intact
    let defaults = source
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Trait(item) if item.ident == "FspecBackend" => Some(
                item.items
                    .iter()
                    .filter_map(|member| match member {
                        syn::TraitItem::Fn(method) if method.default.is_some() => {
                            Some(method.sig.ident.to_string())
                        }
                        _ => None,
                    })
                    .collect::<BTreeSet<_>>(),
            ),
            _ => None,
        })
        .ok_or("backend trait missing")?;
    for name in [
        "list_live_sessions",
        "list_provider_settings",
        "session_created_rx",
    ] {
        assert!(
            defaults.contains(name),
            "existing default {name} must remain optional"
        );
    }
    Ok(())
}

#[test]
fn decomposed_integration_source_remains_below_the_module_ceiling() -> TestResult {
    // @step Given the six approved integration entrypoints and their actual included sources
    let entrypoints = [
        "fspec-tui/src/components/mod.rs",
        "fspec-tui/src/views/navigator_mux_events.rs",
        "fspec-tui/src/transport/mod.rs",
        "fspec-tui/src/transport/embedded.rs",
        "fspec-tui/src/transport/websocket.rs",
        "rpc/src/lib.rs",
    ];
    // @step When their maintained Rust source sizes are inspected
    let mut oversized = Vec::new();
    for path in entrypoints {
        for input in resolve(&workspace().join(path))?.inputs {
            let lines = std::fs::read_to_string(&input)?.lines().count();
            if lines >= 300 {
                oversized.push(format!("{}: {lines}", input.display()));
            }
        }
    }
    // @step Then every targeted entrypoint and extracted source is below 300 lines
    assert!(
        oversized.is_empty(),
        "oversized maintained sources:\n{}",
        oversized.join("\n")
    );
    Ok(())
}

#[test]
fn mcp_mappings_equal_the_resolved_service_methods_exactly() -> TestResult {
    // @step Given the actual composed RPC service declaration and typed MCP registry
    let file = syn::parse_file(&read_source(&workspace().join("rpc/src/lib.rs"))?)?;
    let methods = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Trait(item) if item.ident == "FspecService" => Some(
                item.items
                    .iter()
                    .filter_map(|member| match member {
                        syn::TraitItem::Fn(method) => Some(method.sig.ident.to_string()),
                        _ => None,
                    })
                    .collect::<BTreeSet<_>>(),
            ),
            _ => None,
        })
        .ok_or("service trait missing")?;
    // @step When their method names are compared
    let registrations = codelet_rpc::mcp_dispatch::methods();
    let actual = registrations
        .iter()
        .map(|method| method.name.to_string())
        .collect::<BTreeSet<_>>();
    // @step Then names match exactly without duplicate registrations and schemas remain strict
    assert_eq!(methods, actual);
    assert_eq!(actual.len(), registrations.len());
    for method in registrations {
        assert_eq!(method.schema["additionalProperties"], false);
        assert!(codelet_rpc::mcp_dispatch::validate(
            method.name,
            serde_json::json!({"unexpected":true})
        )
        .is_err());
    }
    Ok(())
}
