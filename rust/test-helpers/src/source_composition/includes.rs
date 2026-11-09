use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;

use super::{invalid, spans};

pub(super) fn expand(
    path: &Path,
    active: &mut BTreeSet<PathBuf>,
    inputs: &mut BTreeSet<PathBuf>,
) -> io::Result<String> {
    let path = path.canonicalize()?;
    if !active.insert(path.clone()) {
        return Err(invalid("cyclic source include"));
    }
    inputs.insert(path.clone());
    let source = std::fs::read_to_string(&path)?;
    let file = syn::parse_file(&source).map_err(invalid)?;
    let mut edits = Vec::new();
    for item in file.items {
        if let syn::Item::Macro(item) = item {
            if item.mac.path.is_ident("include") {
                if !item.attrs.is_empty() {
                    return Err(invalid("attributes on source includes are unsupported"));
                }
                let literal =
                    syn::parse2::<syn::LitStr>(item.mac.tokens.clone()).map_err(invalid)?;
                let child = path
                    .parent()
                    .ok_or_else(|| invalid("source has no parent"))?
                    .join(literal.value());
                let expanded = expand(&child, active, inputs)?;
                edits.push((spans::range(&source, item.span())?, expanded));
            }
        }
    }
    active.remove(&path);
    spans::replace(source, edits)
}
