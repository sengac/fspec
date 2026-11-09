//! Follow file-backed `mod name;` declarations so source-shape tests read a
//! module split across files as one text: the module's own composed source
//! (includes and `compose_` macros expanded) followed by each child module,
//! depth-first in declaration order.

use std::io;
use std::path::{Path, PathBuf};

use super::invalid;

pub(super) fn read_tree(path: &Path) -> io::Result<String> {
    let mut text = super::read_source(path)?;
    let file = syn::parse_file(&text).map_err(invalid)?;
    let mut children = Vec::new();
    for item in &file.items {
        if let syn::Item::Mod(module) = item {
            if module.content.is_none() {
                children.push(match path_attribute(module) {
                    Some(explicit) => explicit_path(path, &explicit)?,
                    None => child_path(path, &module.ident.to_string())?,
                });
            }
        }
    }
    for child in children {
        text.push_str("\n\n");
        text.push_str(&read_tree(&child)?);
    }
    Ok(text)
}

/// `foo.rs` declares children in `foo/`; `mod.rs`, `lib.rs` and `main.rs`
/// declare them beside themselves.
fn child_path(parent: &Path, name: &str) -> io::Result<PathBuf> {
    let file_name = parent
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let base = parent
        .parent()
        .ok_or_else(|| invalid("source has no parent"))?;
    let dir = match file_name {
        "mod.rs" | "lib.rs" | "main.rs" => base.to_path_buf(),
        _ => base.join(parent.file_stem().unwrap_or_default()),
    };
    let flat = dir.join(format!("{name}.rs"));
    if flat.is_file() {
        return Ok(flat);
    }
    let nested = dir.join(name).join("mod.rs");
    if nested.is_file() {
        return Ok(nested);
    }
    // A crate root such as an integration test (`tests/foo.rs`) declares its
    // children beside itself, like `mod.rs` does.
    for sibling in [
        base.join(format!("{name}.rs")),
        base.join(name).join("mod.rs"),
    ] {
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
    Err(invalid(format!(
        "module {name} not found beside {}",
        parent.display()
    )))
}

/// The value of a `#[path = "..."]` attribute on a file-backed module.
fn path_attribute(module: &syn::ItemMod) -> Option<String> {
    module.attrs.iter().find_map(|attr| {
        let syn::Meta::NameValue(meta) = &attr.meta else {
            return None;
        };
        if !meta.path.is_ident("path") {
            return None;
        }
        match &meta.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(value),
                ..
            }) => Some(value.value()),
            _ => None,
        }
    })
}

/// `#[path]` is relative to the declaring file's directory.
fn explicit_path(parent: &Path, explicit: &str) -> io::Result<PathBuf> {
    let base = parent
        .parent()
        .ok_or_else(|| invalid("source has no parent"))?;
    let resolved = base.join(explicit);
    if resolved.is_file() {
        Ok(resolved)
    } else {
        Err(invalid(format!(
            "#[path] module {} not found",
            resolved.display()
        )))
    }
}
