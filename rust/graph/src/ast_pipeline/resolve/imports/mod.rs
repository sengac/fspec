//! Import and package rules of the languages resolved by
//! `GenericResolver` (KGRAPH-075). Each language module turns the import
//! facts of a file into name bindings, from exact facts only: the import as
//! written, the files of the tree, and package or namespace declarations.

use std::collections::BTreeMap;

use super::{DefRef, Reason, TreeIndex};

mod c_family;
mod jvm;
mod scripting;

/// What an import binds.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ImportTarget {
    /// A file (a module): `local` names it, and `local.f` names its `f`.
    File(usize),
    /// A definition.
    Definition(DefRef),
}

/// One name an import binds in a file.
#[derive(Clone, Debug)]
pub(crate) struct ImportBinding {
    /// The name bound in the importing file; `None` binds every top-level
    /// name of the target file (a wildcard or an include).
    pub local: Option<String>,
    pub target: Result<ImportTarget, Reason>,
    /// The import as written, for the Imports edge.
    pub import_path: String,
}

/// The import bindings of `file`.
pub(crate) fn bindings(
    index: &TreeIndex<'_>,
    file: usize,
    manifests: &BTreeMap<String, String>,
) -> Vec<ImportBinding> {
    match index.file(file).language {
        "python" | "typescript" | "javascript" | "go" => {
            scripting::bindings(index, file, manifests)
        }
        "java" | "kotlin" | "scala" | "csharp" => jvm::bindings(index, file, manifests),
        "c" | "cpp" | "ruby" | "php" | "swift" | "dart" => {
            c_family::bindings(index, file, manifests)
        }
        _ => Vec::new(),
    }
}

/// The files whose top-level definitions are in scope in `file` without an
/// import (Go: the other files of the package directory; Java: the files
/// declaring the same package).
pub(crate) fn package_peers(index: &TreeIndex<'_>, file: usize) -> Vec<usize> {
    match index.file(file).language {
        "python" | "typescript" | "javascript" | "go" => scripting::package_peers(index, file),
        "java" | "kotlin" | "scala" | "csharp" => jvm::package_peers(index, file),
        "c" | "cpp" | "ruby" | "php" | "swift" | "dart" => c_family::package_peers(index, file),
        _ => Vec::new(),
    }
}
