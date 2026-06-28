//! Dart language support for ast-grep tools.
//!
//! Since ast-grep upstream removed Dart in v0.30.0, we integrate `tree-sitter-dart`
//! directly and implement the ast-grep `Language` + `LanguageExt` traits ourselves.
//!
//! Dart identifiers accept `$` as a valid character (`[a-zA-Z_$][\w$]*`), so ast-grep's
//! `$NAME` meta-variables parse directly as valid identifiers. No expando_char needed —
//! this is the same approach used for Java, JavaScript, and Bash in ast-grep (impl_lang!).
//!
//! Note: Dart's tree-sitter grammar splits top-level function declarations into two
//! sibling nodes (`function_signature` + `function_body`). Patterns like
//! `void $NAME() { $$$BODY }` will fail with "Multiple AST nodes". Use
//! `void $NAME($$$PARAMS)` to match just the signature, or search inside class bodies
//! where method signatures and bodies are wrapped in `class_member` nodes.

use ast_grep_core::language::Language;
use ast_grep_core::matcher::{Pattern, PatternBuilder, PatternError};
use ast_grep_core::tree_sitter::{StrDoc, TSLanguage};
pub use ast_grep_language::LanguageExt;

/// Dart language implementation for ast-grep pattern matching.
///
/// Implements `Language` and `LanguageExt` traits using `tree-sitter-dart` v0.1.0.
/// No expando_char needed since `$` is valid in Dart identifiers.
#[derive(Clone, Copy, Debug)]
pub struct DartLang;

impl Language for DartLang {
    fn kind_to_id(&self, kind: &str) -> u16 {
        self.get_ts_language()
            .id_for_node_kind(kind, /*named*/ true)
    }

    fn field_to_id(&self, field: &str) -> Option<u16> {
        self.get_ts_language()
            .field_id_for_name(field)
            .map(std::num::NonZero::get)
    }

    fn build_pattern(&self, builder: &PatternBuilder) -> Result<Pattern, PatternError> {
        builder.build(|src| StrDoc::try_new(src, *self))
    }
}

impl LanguageExt for DartLang {
    fn get_ts_language(&self) -> TSLanguage {
        tree_sitter_dart::LANGUAGE.into()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
mod tests {
    use super::*;

    #[test]
    fn test_dart_lang_parses_simple_code() {
        let dart = DartLang;
        let root = dart.ast_grep("class Foo { int x; }");
        assert!(
            root.root().text().contains("Foo"),
            "Should parse Dart source code"
        );
    }

    #[test]
    fn test_dart_class_pattern_matching() {
        let dart = DartLang;
        let root = dart.ast_grep("class Dog { String name; }");
        let pattern = Pattern::new("class $NAME { $$$BODY }", dart);
        let matches: Vec<_> = root.root().find_all(pattern).collect();
        assert!(
            !matches.is_empty(),
            "Should find class declaration with pattern"
        );
    }

    #[test]
    fn test_dart_function_signature_matching() {
        // Dart splits function_signature and function_body as siblings,
        // so we can only match the signature part at top level
        let dart = DartLang;
        let root = dart.ast_grep("int add(int a, int b) { return a + b; }");
        let pattern = Pattern::new("int $NAME($$$PARAMS)", dart);
        let matches: Vec<_> = root.root().find_all(pattern).collect();
        assert!(
            !matches.is_empty(),
            "Should find function signature with pattern"
        );
    }
}
