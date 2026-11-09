//! BUG-642: the exact source check for range slices with a fixed bound.
//!
//! Feature: spec/features/text-cuts-land-on-char-boundaries.feature
//!
//! Cutting text at a fixed byte index (`&text[..N]`) panics when the index
//! falls inside a multi-byte character. The check parses a Rust file with
//! `syn` and reports every index expression whose index is a range of one of
//! these exact shapes, inside macro arguments too (`format!("{}", &s[..N])`):
//!
//! - `x[..N]` / `x[..=N]` where `N` is an integer literal or a constant
//!   (an all-caps path such as `MAX_LEN` or `limits::MAX_LEN`);
//! - `x[x.len() - N..]` where `N` is an integer literal or a constant.
//!
//! A slice cut with `floor_char_boundary(N)` has a computed bound and is not
//! reported. The check can't see types, so the caller keeps an exact list of
//! the reported slices that are safe (ASCII-only text such as UUID and hex
//! prefixes, or non-text data such as byte arrays and vectors).

use proc_macro2::TokenStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, Lit, Token};

/// One reported slice: its 1-based line and its exact source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedLengthCut {
    pub line: usize,
    pub text: String,
}

/// Every fixed-bound range slice in `source`, in source order.
pub fn fixed_length_cuts(source: &str) -> Result<Vec<FixedLengthCut>, String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let lines: Vec<&str> = source.lines().collect();
    let mut visitor = CutVisitor {
        lines: &lines,
        found: Vec::new(),
    };
    visitor.visit_file(&file);
    visitor.found.sort_by_key(|cut| cut.line);
    visitor.found.dedup();
    Ok(visitor.found)
}

struct CutVisitor<'a> {
    lines: &'a [&'a str],
    found: Vec<FixedLengthCut>,
}

fn is_fixed(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(lit) => matches!(lit.lit, Lit::Int(_)),
        Expr::Path(path) => path.path.segments.last().is_some_and(|segment| {
            let name = segment.ident.to_string();
            name.chars().any(|c| c.is_ascii_uppercase())
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        }),
        Expr::Paren(paren) => is_fixed(&paren.expr),
        _ => false,
    }
}

fn is_len_minus_fixed(expr: &Expr) -> bool {
    match expr {
        Expr::Binary(binary) if matches!(binary.op, syn::BinOp::Sub(_)) => {
            matches!(&*binary.left, Expr::MethodCall(call) if call.method == "len" && call.args.is_empty())
                && is_fixed(&binary.right)
        }
        Expr::Paren(paren) => is_len_minus_fixed(&paren.expr),
        _ => false,
    }
}

fn is_fixed_range(index: &Expr) -> bool {
    match index {
        Expr::Range(range) => match (&range.start, &range.end) {
            (None, Some(end)) => is_fixed(end),
            (Some(start), None) => is_len_minus_fixed(start),
            _ => false,
        },
        Expr::Paren(paren) => is_fixed_range(&paren.expr),
        _ => false,
    }
}

impl CutVisitor<'_> {
    fn text_of(&self, expr: &syn::ExprIndex) -> FixedLengthCut {
        let (start, end) = (expr.span().start(), expr.span().end());
        let line = self
            .lines
            .get(start.line.saturating_sub(1))
            .copied()
            .unwrap_or("");
        let text = if start.line == end.line {
            line.chars()
                .skip(start.column)
                .take(end.column.saturating_sub(start.column))
                .collect()
        } else {
            line.chars().skip(start.column).collect::<String>()
        };
        FixedLengthCut {
            line: start.line,
            text,
        }
    }

    fn visit_tokens(&mut self, tokens: TokenStream) {
        use syn::parse::Parser;
        let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
        if let Ok(exprs) = parser.parse2(tokens) {
            for expr in exprs {
                self.visit_expr(&expr);
            }
        }
    }
}

impl<'ast> Visit<'ast> for CutVisitor<'_> {
    fn visit_expr_index(&mut self, expr: &'ast syn::ExprIndex) {
        if is_fixed_range(&expr.index) {
            let cut = self.text_of(expr);
            self.found.push(cut);
        }
        syn::visit::visit_expr_index(self, expr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.visit_tokens(mac.tokens.clone());
        syn::visit::visit_macro(self, mac);
    }
}
