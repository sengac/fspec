use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;

mod includes;
mod macros;
mod spans;

pub struct ResolvedSource {
    pub text: String,
    pub inputs: Vec<PathBuf>,
}

pub fn read_source(path: &Path) -> io::Result<String> {
    Ok(resolve(path)?.text)
}

pub fn resolve(path: &Path) -> io::Result<ResolvedSource> {
    let mut inputs = BTreeSet::new();
    let source = includes::expand(path, &mut BTreeSet::new(), &mut inputs)?;
    let file = syn::parse_file(&source).map_err(invalid)?;
    let mut definitions = BTreeMap::new();
    let mut edits = Vec::new();
    for item in &file.items {
        if let syn::Item::Macro(item) = item {
            if item.mac.path.is_ident("macro_rules") {
                if let Some(name) = &item.ident {
                    if name.to_string().starts_with("compose_") {
                        if !item.attrs.is_empty() {
                            return Err(invalid(
                                "attributes on composition definitions are unsupported",
                            ));
                        }
                        if definitions.insert(name.to_string(), item).is_some() {
                            return Err(invalid("duplicate composition definition"));
                        }
                        edits.push((spans::range(&source, item.span())?, String::new()));
                    }
                }
            }
        }
    }
    let mut used = BTreeSet::new();
    for item in &file.items {
        if let syn::Item::Macro(item) = item {
            if item.ident.is_none() {
                if let Some(name) = item.mac.path.get_ident() {
                    if name.to_string().starts_with("compose_") {
                        if !item.attrs.is_empty() {
                            return Err(invalid(
                                "attributes on composition entries are unsupported",
                            ));
                        }
                        if !item.mac.tokens.is_empty() {
                            return Err(invalid("composition entry must have no initial members"));
                        }
                        let expanded =
                            macros::expand(&source, &definitions, &name.to_string(), &mut used)?;
                        edits.push((spans::range(&source, item.span())?, expanded));
                    }
                }
            }
        }
    }
    if used.len() != definitions.len() {
        return Err(invalid("disconnected composition definition"));
    }
    let text = spans::replace(source, edits)?;
    syn::parse_file(&text).map_err(invalid)?;
    Ok(ResolvedSource {
        text,
        inputs: inputs.into_iter().collect(),
    })
}

fn invalid(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}
