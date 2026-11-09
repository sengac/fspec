//! TypeScript with JSX (tree-sitter-typescript 0.23, `tsx` grammar): the
//! TypeScript table with the `tsx` grammar.

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

pub static TSX: LanguageSpec =
    super::typescript::spec(&["tsx"], Grammar::Builtin(SupportLang::Tsx));
