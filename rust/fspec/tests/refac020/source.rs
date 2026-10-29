use std::path::PathBuf;
use std::process::Command;

use quote::ToTokens;
use sha2::{Digest, Sha256};
use syn::visit_mut::VisitMut;

use crate::TestResult;

pub const MAIN_BLOB: &str = "2962c98e7d66d917026aa6feb660914f8be73ff7";
pub const MAIN_SHA: &str =
    "e20bd03b59b2801df601a18d491344765b6da550e9da141cde6af4bd5679559f";

pub fn original() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["cat-file", "blob", MAIN_BLOB])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    if !output.status.success() {
        return Err(format!("frozen original CLI blob unavailable: {}", MAIN_BLOB).into());
    }
    assert_eq!(hex::encode(Sha256::digest(&output.stdout)), MAIN_SHA);
    Ok(String::from_utf8(output.stdout)?)
}

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs")
}

pub fn actual() -> Result<String, Box<dyn std::error::Error>> {
    Ok(codelet_test_helpers::source_composition::read_source(&root())?)
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
        "LinkCoverage" | "UnlinkCoverage" | "link-coverage" | "unlink-coverage" => {
            &["test_mapping_index", "expected_scenario_hash"]
        }
        "GenerateCoverage" | "generate-coverage" => &["feature_name"],
        "ShowCoverage" | "show-coverage" => &["mapping_json"],
        _ => &[],
    }
}

pub fn strip_declaration_additions(items: &mut [syn::Item]) {
    for item in items {
        if let syn::Item::Enum(item) = item {
            for variant in &mut item.variants {
                if let syn::Fields::Named(fields) = &mut variant.fields {
                    fields.named = fields
                        .named
                        .iter()
                        .filter(|field| {
                            !additions(&variant.ident.to_string()).contains(
                                &field.ident.as_ref().map(ToString::to_string).unwrap_or_default().as_str(),
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
        pattern.fields = pattern.fields.iter().filter(|field| {
            !self.fields.contains(&tokens(&field.member).as_str())
        }).cloned().collect();
        syn::visit_mut::visit_pat_struct_mut(self, pattern);
    }

    fn visit_expr_struct_mut(&mut self, expression: &mut syn::ExprStruct) {
        expression.fields = expression.fields.iter().filter(|field| {
            !self.fields.contains(&tokens(&field.member).as_str())
        }).cloned().collect();
        syn::visit_mut::visit_expr_struct_mut(self, expression);
    }

    fn visit_expr_macro_mut(&mut self, expression: &mut syn::ExprMacro) {
        if expression.mac.path.is_ident("forward") {
            if let Ok(mut invocation) = syn::parse2::<Forward>(expression.mac.tokens.clone()) {
                self.visit_expr_mut(&mut invocation.arguments);
                let bridge = invocation.bridge;
                let arguments = invocation.arguments;
                expression.mac.tokens = quote::quote!(#bridge, #arguments);
            }
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
