//! TEST-042: the exact source check that fspec's test code never waits on a
//! wall-clock sleep or a fixed timeout.
//!
//! Feature: spec/features/fspec-tests-never-wait-on-wall-clock-sleeps-or-fixed-timeouts.feature
//!
//! The check is exact, not heuristic: each file is read as Rust tokens, every
//! `a::b::c` path (inside macros too) is resolved through the `use`
//! declarations in scope, and a path that resolves to one of
//! [`FORBIDDEN_PATHS`] is reported. A bare imported name counts only where it
//! is called (`sleep(..)`), so a local variable named `timeout` is never
//! mistaken for the function. Method calls are reported by their exact name
//! ([`FORBIDDEN_METHODS`]). In a `src` file only the test code counts: items
//! under `#[cfg(test)]` and `#[test]` functions. A test annotated
//! `#[tokio::test(start_paused = true)]` runs on tokio's virtual clock, so
//! its tokio time calls are not wall-clock waits and are not reported (a
//! std sleep or std clock reading there still is).
//!
//! A comparison (`<`, `<=`, `>`, `>=`) with an operand that reads the clock
//! is a fixed deadline too ([`CLOCK_READINGS`]): an `.elapsed()` call, an
//! `Instant::now()` call, or a local bound by `let` to an expression that
//! reads the clock. A test module's `super` paths resolve through its parent
//! module file's `use` declarations.
//!
//! The only timed wait allowed is [`crate::deadlock_guard`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

/// Functions a test may not call: wall-clock sleeps and fixed timeouts.
pub const FORBIDDEN_PATHS: &[&str] = &[
    "std::thread::sleep",
    "std::thread::sleep_ms",
    "tokio::time::sleep",
    "tokio::time::sleep_until",
    "tokio::time::timeout",
    "tokio::time::timeout_at",
    "tokio::time::interval",
    "tokio::time::interval_at",
];

/// Timed-wait methods a test may not call.
pub const FORBIDDEN_METHODS: &[&str] = &[
    "recv_timeout",
    "wait_timeout",
    "wait_timeout_while",
    "wait_timeout_ms",
];

/// Clock readings a test may not compare with a bound (reported as
/// `<reading> comparison`); `.elapsed()` calls count as well.
pub const CLOCK_READINGS: &[&str] = &["std::time::Instant::now", "tokio::time::Instant::now"];

/// One forbidden wait: where it is and what it calls.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WallClockWait {
    pub file: PathBuf,
    pub line: usize,
    pub call: String,
}

impl std::fmt::Display for WallClockWait {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.file.display(), self.line, self.call)
    }
}

/// The names a scope's `use` declarations bring in: a name maps to the full
/// path it stands for, and glob imports add whole prefixes.
#[derive(Debug, Clone, Default)]
pub struct Imports {
    names: BTreeMap<String, Vec<String>>,
    globs: Vec<Vec<String>>,
    /// What `super` stands for: the parent module file's imports.
    parent: Option<Box<Imports>>,
}

impl Imports {
    fn add_tree(&mut self, prefix: &[String], tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(path) => {
                let mut prefix = prefix.to_vec();
                prefix.push(path.ident.to_string());
                self.add_tree(&prefix, &path.tree);
            }
            syn::UseTree::Name(name) => {
                let name = name.ident.to_string();
                if name == "self" {
                    if let Some(last) = prefix.last() {
                        self.names.insert(last.clone(), prefix.to_vec());
                    }
                } else {
                    let mut full = prefix.to_vec();
                    full.push(name.clone());
                    self.names.insert(name, full);
                }
            }
            syn::UseTree::Rename(rename) => {
                let mut full = prefix.to_vec();
                let original = rename.ident.to_string();
                if original != "self" {
                    full.push(original);
                }
                self.names.insert(rename.rename.to_string(), full);
            }
            syn::UseTree::Glob(_) => self.globs.push(prefix.to_vec()),
            syn::UseTree::Group(group) => {
                for tree in &group.items {
                    self.add_tree(prefix, tree);
                }
            }
        }
    }

    /// The full paths `segments` may stand for: itself, through a named or
    /// glob import, and through `super` (the parent module's imports).
    fn resolve(&self, segments: &[String]) -> Vec<String> {
        let mut candidates: Vec<Vec<String>> = vec![segments.to_vec()];
        if let Some((first, rest)) = segments.split_first() {
            if let Some(full) = self.names.get(first) {
                let mut resolved = full.clone();
                resolved.extend(rest.iter().cloned());
                candidates.push(resolved);
            }
            for glob in &self.globs {
                let mut resolved = glob.clone();
                resolved.extend(segments.iter().cloned());
                candidates.push(resolved);
            }
        }
        let mut paths = Vec::new();
        for candidate in candidates {
            match (candidate.split_first(), &self.parent) {
                (Some((first, rest)), Some(parent)) if first == "super" && !rest.is_empty() => {
                    paths.extend(parent.resolve(rest));
                }
                _ => paths.push(candidate.join("::")),
            }
        }
        paths
    }

    /// The forbidden path `segments` resolves to, if any. A single segment
    /// counts only when `called`.
    fn forbidden(&self, segments: &[String], called: bool) -> Option<String> {
        if segments.len() == 1 && !called {
            return None;
        }
        self.resolve(segments)
            .into_iter()
            .find(|path| FORBIDDEN_PATHS.contains(&path.as_str()))
    }

    /// The clock reading `segments` resolves to, if any (always called).
    fn clock(&self, segments: &[String]) -> Option<String> {
        self.resolve(segments)
            .into_iter()
            .find(|path| CLOCK_READINGS.contains(&path.as_str()))
    }
}

fn is_punct(token: Option<&TokenTree>, ch: char) -> bool {
    matches!(token, Some(TokenTree::Punct(punct)) if punct.as_char() == ch)
}

fn is_path_separator(tokens: &[TokenTree], at: usize) -> bool {
    matches!(tokens.get(at), Some(TokenTree::Punct(p)) if p.as_char() == ':' && p.spacing() == Spacing::Joint)
        && is_punct(tokens.get(at + 1), ':')
}

fn ident(token: Option<&TokenTree>) -> Option<String> {
    match token {
        Some(TokenTree::Ident(ident)) => Some(ident.to_string()),
        _ => None,
    }
}

fn is_call(token: Option<&TokenTree>) -> bool {
    matches!(token, Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
}

/// The `use` declarations directly in `tokens` (one brace level), and the
/// token ranges they occupy.
fn scope_imports(tokens: &[TokenTree], inherited: &Imports) -> (Imports, Vec<(usize, usize)>) {
    let mut imports = inherited.clone();
    let mut ranges = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        let starts_use = ident(tokens.get(at)).as_deref() == Some("use")
            && !is_punct(at.checked_sub(1).and_then(|before| tokens.get(before)), '.');
        if !starts_use {
            at += 1;
            continue;
        }
        let end = (at..tokens.len())
            .find(|&index| is_punct(tokens.get(index), ';'))
            .unwrap_or(tokens.len() - 1);
        let declaration: TokenStream = tokens[at..=end].iter().cloned().collect();
        if let Ok(item) = syn::parse2::<syn::ItemUse>(declaration) {
            let prefix: Vec<String> = item.leading_colon.map(|_| Vec::new()).unwrap_or_default();
            imports.add_tree(&prefix, &item.tree);
        }
        ranges.push((at, end));
        at = end + 1;
    }
    (imports, ranges)
}

/// Which lines count: test code (`counts`) and paused-clock tests
/// (`paused`, where only wall-clock calls and readings are reported).
struct Lines<'a> {
    counts: &'a dyn Fn(usize) -> bool,
    paused: &'a dyn Fn(usize) -> bool,
}

/// A local bound to a clock reading: what it reads, and whether that is
/// surely the wall clock (a std `Instant`).
#[derive(Debug, Clone)]
struct Reading {
    name: String,
    wall: bool,
}

type Locals = BTreeMap<String, Reading>;

/// Report every forbidden wait in `stream` (and its nested groups) whose
/// line counts.
fn walk(
    stream: TokenStream,
    inherited: &Imports,
    inherited_locals: &Locals,
    lines: &Lines<'_>,
    found: &mut Vec<(usize, String)>,
) {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let (imports, use_ranges) = scope_imports(&tokens, inherited);
    let locals = scope_locals(&tokens, &imports, inherited_locals);
    clock_comparisons(&tokens, &imports, &locals, lines, found);
    let in_use = |index: usize| {
        use_ranges
            .iter()
            .any(|&(start, end)| index >= start && index <= end)
    };
    let mut at = 0;
    while at < tokens.len() {
        if in_use(at) {
            at += 1;
            continue;
        }
        match &tokens[at] {
            TokenTree::Group(group) => {
                walk(group.stream(), &imports, &locals, lines, found);
                at += 1;
            }
            TokenTree::Ident(first) => {
                let line = first.span().start().line;
                let before = at.checked_sub(1).and_then(|index| tokens.get(index));
                let is_method = is_punct(before, '.');
                let is_definition = matches!(ident(before).as_deref(), Some("fn"));
                let mut segments = vec![first.to_string()];
                let mut next = at + 1;
                while is_path_separator(&tokens, next) {
                    match ident(tokens.get(next + 2)) {
                        Some(segment) => {
                            segments.push(segment);
                            next += 3;
                        }
                        None => break,
                    }
                }
                let called = is_call(tokens.get(next))
                    || (is_path_separator(&tokens, next) && is_punct(tokens.get(next + 2), '<'));
                if (lines.counts)(line) && !is_definition {
                    if is_method {
                        if called && FORBIDDEN_METHODS.contains(&segments[0].as_str()) {
                            found.push((line, segments[0].clone()));
                        }
                    } else if let Some(path) = imports.forbidden(&segments, called) {
                        // Tokio's clock is virtual in a paused-clock test.
                        if !((lines.paused)(line) && path.starts_with("tokio::")) {
                            found.push((line, path));
                        }
                    }
                }
                at = next;
            }
            TokenTree::Punct(_) | TokenTree::Literal(_) => at += 1,
        }
    }
}

fn punct(token: Option<&TokenTree>) -> Option<&proc_macro2::Punct> {
    match token {
        Some(TokenTree::Punct(punct)) => Some(punct),
        _ => None,
    }
}

/// The end (exclusive) of the operand that starts with the identifier at
/// `start`: its path, then calls, indexing, `?`, field and method access
/// and `as` casts.
fn operand_end(tokens: &[TokenTree], start: usize) -> usize {
    let mut at = start + 1;
    while is_path_separator(tokens, at) && ident(tokens.get(at + 2)).is_some() {
        at += 3;
    }
    loop {
        match tokens.get(at) {
            Some(TokenTree::Group(group))
                if matches!(
                    group.delimiter(),
                    Delimiter::Parenthesis | Delimiter::Bracket
                ) =>
            {
                at += 1;
            }
            Some(TokenTree::Punct(p)) if p.as_char() == '?' => at += 1,
            Some(TokenTree::Punct(p))
                if p.as_char() == '.'
                    && p.spacing() == Spacing::Alone
                    && matches!(
                        tokens.get(at + 1),
                        Some(TokenTree::Ident(_) | TokenTree::Literal(_))
                    ) =>
            {
                at += 2;
            }
            Some(TokenTree::Ident(word)) if word == "as" && ident(tokens.get(at + 1)).is_some() => {
                at += 2;
                while is_path_separator(tokens, at) && ident(tokens.get(at + 2)).is_some() {
                    at += 3;
                }
            }
            _ => return at,
        }
    }
}

/// The clock reading of the operand `tokens[start..end]`, if it reads the
/// clock: an `.elapsed()` call, an `Instant::now()` call, or a local bound
/// to a reading.
fn operand_reading(
    tokens: &[TokenTree],
    start: usize,
    end: usize,
    imports: &Imports,
    locals: &Locals,
) -> Option<Reading> {
    let mut segments = Vec::new();
    let mut at = start;
    if let Some(first) = ident(tokens.get(at)) {
        segments.push(first);
        at += 1;
        while is_path_separator(tokens, at) {
            match ident(tokens.get(at + 2)) {
                Some(segment) => {
                    segments.push(segment);
                    at += 3;
                }
                None => break,
            }
        }
    }
    let head_called = is_call(tokens.get(at));
    let now = head_called.then(|| imports.clock(&segments)).flatten();
    let local = (segments.len() == 1 && !head_called && !is_punct(tokens.get(at), '!'))
        .then(|| locals.get(&segments[0]))
        .flatten();
    let elapsed = (start + 1..end).any(|index| {
        is_punct(tokens.get(index - 1), '.')
            && ident(tokens.get(index)).as_deref() == Some("elapsed")
            && matches!(tokens.get(index + 1), Some(TokenTree::Group(group))
                if group.delimiter() == Delimiter::Parenthesis && group.stream().is_empty())
    });
    let wall =
        now.as_deref() == Some("std::time::Instant::now") || local.is_some_and(|local| local.wall);
    if elapsed {
        return Some(Reading {
            name: "elapsed()".to_string(),
            wall,
        });
    }
    if let Some(now) = now {
        return Some(Reading {
            name: format!("{now}()"),
            wall,
        });
    }
    local.cloned()
}

/// Whether the operand `tokens[start..]` starts an operand: an identifier
/// that does not continue a path, a field or a method chain.
fn starts_operand(tokens: &[TokenTree], start: usize) -> bool {
    if ident(tokens.get(start)).is_none() {
        return false;
    }
    let Some(before) = start.checked_sub(1) else {
        return true;
    };
    !(is_punct(tokens.get(before), '.')
        || is_punct(tokens.get(before), '?')
        || (is_punct(tokens.get(before), ':')
            && before >= 1
            && is_path_separator(tokens, before - 1)))
}

/// The comparison operator right after an operand ending at `end`, as the
/// index of its first token.
fn comparison_after(tokens: &[TokenTree], end: usize) -> Option<usize> {
    let first = punct(tokens.get(end))?;
    if !matches!(first.as_char(), '<' | '>') {
        return None;
    }
    if first.spacing() == Spacing::Alone {
        return Some(end);
    }
    is_punct(tokens.get(end + 1), '=').then_some(end)
}

/// The comparison operator right before an operand starting at `start`, as
/// the index of its first token.
fn comparison_before(tokens: &[TokenTree], start: usize) -> Option<usize> {
    let last_index = start.checked_sub(1)?;
    let last = punct(tokens.get(last_index))?;
    let previous = last_index
        .checked_sub(1)
        .and_then(|index| punct(tokens.get(index)))
        .filter(|previous| previous.spacing() == Spacing::Joint);
    match last.as_char() {
        '<' | '>' => match previous {
            // `<<`, `>>`, `->`, `=>`, `<=`-like pairs are not this operator.
            Some(previous) if matches!(previous.as_char(), '<' | '>' | '-' | '=') => None,
            _ => Some(last_index),
        },
        '=' => match previous {
            Some(previous) if matches!(previous.as_char(), '<' | '>') => Some(last_index - 1),
            _ => None,
        },
        _ => None,
    }
}

/// Whether the brace group at `index` is an item's body (a `fn`, `impl`,
/// `mod`, `trait`, `struct` or `enum`), not a block of the code around it.
fn is_item_body(tokens: &[TokenTree], index: usize) -> bool {
    tokens[..index]
        .iter()
        .rev()
        .take_while(|token| {
            !matches!(token, TokenTree::Punct(p) if p.as_char() == ';')
                && !matches!(token, TokenTree::Group(g) if g.delimiter() == Delimiter::Brace)
        })
        .any(|token| {
            matches!(token, TokenTree::Ident(word)
                if ["fn", "impl", "mod", "trait", "struct", "enum"].contains(&word.to_string().as_str()))
        })
}

/// The clock reading of the expression `tokens[from..to]`, if any operand
/// in it reads the clock.
fn expression_reading(
    tokens: &[TokenTree],
    from: usize,
    to: usize,
    imports: &Imports,
    locals: &Locals,
) -> Option<Reading> {
    (from..to.min(tokens.len())).find_map(|at| {
        if let TokenTree::Group(group) = &tokens[at] {
            // Call arguments: a direct reading counts (`x.min(t.elapsed())`),
            // a local merely passed along does not.
            if group.delimiter() != Delimiter::Parenthesis {
                return None;
            }
            let inner: Vec<TokenTree> = group.stream().into_iter().collect();
            return expression_reading(&inner, 0, inner.len(), imports, &Locals::new());
        }
        starts_operand(tokens, at)
            .then(|| operand_reading(tokens, at, operand_end(tokens, at), imports, locals))
            .flatten()
    })
}

/// Whether `tokens[index]` is a lone `=` (not part of `==`, `<=`, `=>`...).
fn is_assignment(tokens: &[TokenTree], index: usize) -> bool {
    punct(tokens.get(index)).is_some_and(|p| p.as_char() == '=' && p.spacing() == Spacing::Alone)
        && !punct(index.checked_sub(1).and_then(|before| tokens.get(before)))
            .is_some_and(|p| p.spacing() == Spacing::Joint)
}

/// Add the locals that `tokens` binds (`let x = ..`) or assigns (`x = ..`,
/// `x += ..`) to an expression that reads the clock, in this block and the
/// blocks nested in it (not item bodies): a reading taints the name for the
/// whole function, wherever it is assigned.
fn collect_locals(tokens: &[TokenTree], imports: &Imports, locals: &mut Locals) {
    let mut at = 0;
    while at < tokens.len() {
        if let TokenTree::Group(group) = &tokens[at] {
            let nested = group.delimiter() != Delimiter::Brace || !is_item_body(tokens, at);
            if nested {
                let inner: Vec<TokenTree> = group.stream().into_iter().collect();
                collect_locals(&inner, imports, locals);
            }
            at += 1;
            continue;
        }
        let statement_start = at == 0
            || matches!(&tokens[at - 1], TokenTree::Punct(p) if p.as_char() == ';')
            || matches!(&tokens[at - 1], TokenTree::Group(g) if g.delimiter() == Delimiter::Brace);
        let is_let = ident(tokens.get(at)).as_deref() == Some("let")
            && !is_punct(at.checked_sub(1).and_then(|before| tokens.get(before)), '.');
        let end = |from: usize| {
            (from..tokens.len())
                .find(|&index| is_punct(tokens.get(index), ';'))
                .unwrap_or(tokens.len())
        };
        if is_let {
            let mut name_at = at + 1;
            if ident(tokens.get(name_at)).as_deref() == Some("mut") {
                name_at += 1;
            }
            let simple = matches!(tokens.get(name_at + 1), Some(TokenTree::Punct(p)) if matches!(p.as_char(), '=' | ':'));
            let stop = end(at);
            let assign = (name_at..stop).find(|&index| is_assignment(tokens, index));
            if let (Some(name), Some(assign), true) = (ident(tokens.get(name_at)), assign, simple) {
                if let Some(reading) = expression_reading(tokens, assign + 1, stop, imports, locals)
                {
                    locals.insert(name, reading);
                }
            }
            // The initializer's own blocks may bind more.
            at += 1;
            continue;
        }
        let compound = matches!(tokens.get(at + 1), Some(TokenTree::Punct(p))
            if matches!(p.as_char(), '+' | '-') && p.spacing() == Spacing::Joint)
            && is_assignment(tokens, at + 2);
        if statement_start && (is_assignment(tokens, at + 1) || compound) {
            if let Some(name) = ident(tokens.get(at)) {
                let stop = end(at);
                let from = if compound { at + 3 } else { at + 2 };
                if let Some(reading) = expression_reading(tokens, from, stop, imports, locals) {
                    locals.insert(name, reading);
                }
            }
        }
        at += 1;
    }
}

/// The locals in scope in `tokens` that hold a clock reading.
fn scope_locals(tokens: &[TokenTree], imports: &Imports, inherited: &Locals) -> Locals {
    let mut locals = inherited.clone();
    collect_locals(tokens, imports, &mut locals);
    locals
}

/// Report every comparison in `tokens` (one level) with an operand that
/// reads the clock.
fn clock_comparisons(
    tokens: &[TokenTree],
    imports: &Imports,
    locals: &Locals,
    lines: &Lines<'_>,
    found: &mut Vec<(usize, String)>,
) {
    let mut reported: Vec<usize> = Vec::new();
    for start in 0..tokens.len() {
        if !starts_operand(tokens, start) {
            continue;
        }
        if ident(tokens.get(start)).as_deref() == Some("let") {
            continue;
        }
        let end = operand_end(tokens, start);
        let Some(reading) = operand_reading(tokens, start, end, imports, locals) else {
            continue;
        };
        let Some(operator) =
            comparison_after(tokens, end).or_else(|| comparison_before(tokens, start))
        else {
            continue;
        };
        if reported.contains(&operator) {
            continue;
        }
        let line = tokens[operator].span().start().line;
        if (lines.counts)(line) && (reading.wall || !(lines.paused)(line)) {
            reported.push(operator);
            found.push((line, format!("{} comparison", reading.name)));
        }
    }
}

fn tokens(source: &str) -> Result<TokenStream, String> {
    source
        .parse::<TokenStream>()
        .map_err(|error| format!("not Rust tokens: {error}"))
}

/// Whether `attribute` is `#[tokio::test(.. start_paused = true ..)]`: the
/// test runs on tokio's paused, virtual clock, which is not a wall clock.
fn is_paused_clock_test(attribute: &syn::Attribute) -> bool {
    let syn::Meta::List(list) = &attribute.meta else {
        return false;
    };
    if list
        .path
        .segments
        .last()
        .is_none_or(|segment| segment.ident != "test")
    {
        return false;
    }
    let tokens: Vec<TokenTree> = list.tokens.clone().into_iter().collect();
    tokens.windows(3).any(|window| {
        ident(window.first()).as_deref() == Some("start_paused")
            && is_punct(window.get(1), '=')
            && ident(window.get(2)).as_deref() == Some("true")
    })
}

/// The line ranges of the paused-clock test functions among `items`.
fn collect_paused_tests(items: &[syn::Item], ranges: &mut Vec<(usize, usize)>) {
    use syn::spanned::Spanned;
    for item in items {
        match item {
            syn::Item::Fn(function) if function.attrs.iter().any(is_paused_clock_test) => {
                let span = function.span();
                ranges.push((span.start().line, span.end().line));
            }
            syn::Item::Mod(module) => {
                if let Some((_, inner)) = &module.content {
                    collect_paused_tests(inner, ranges);
                }
            }
            _ => {}
        }
    }
}

/// The line ranges of a file's paused-clock tests (TEST-042: virtual time is
/// exempt; wall-clock waits are not).
fn paused_test_lines(source: &str) -> Result<Vec<(usize, usize)>, String> {
    let file = syn::parse_file(source).map_err(|error| format!("not a Rust file: {error}"))?;
    let mut ranges = Vec::new();
    collect_paused_tests(&file.items, &mut ranges);
    Ok(ranges)
}

fn within(ranges: &[(usize, usize)], line: usize) -> bool {
    ranges
        .iter()
        .any(|&(start, end)| line >= start && line <= end)
}

/// The forbidden waits of a file that is test code throughout (a `tests/`
/// file or a `#[cfg(test)] mod x;` file), as `(line, call)`.
pub fn scan_test_source(source: &str) -> Result<Vec<(usize, String)>, String> {
    scan_test_source_in(source, &child_scope(&Imports::default()))
}

/// [`scan_test_source`] for a file whose code starts with `scope` in scope.
fn scan_test_source_in(source: &str, scope: &Imports) -> Result<Vec<(usize, String)>, String> {
    let paused = paused_test_lines(source)?;
    let mut found = Vec::new();
    let lines = Lines {
        counts: &|_| true,
        paused: &|line| within(&paused, line),
    };
    walk(tokens(source)?, scope, &Locals::new(), &lines, &mut found);
    found.sort_by_key(|&(line, _)| line);
    Ok(found)
}

/// The scope a module starts with: nothing imported yet, `super` being the
/// parent's imports (if the parent is known).
fn child_scope(parent: &Imports) -> Imports {
    Imports {
        parent: Some(Box::new(parent.clone())),
        ..Imports::default()
    }
}

/// The imports at the top level of `source`, whose `super` is `parent`.
fn file_imports(source: &str, parent: &Imports) -> Result<Imports, String> {
    let tokens: Vec<TokenTree> = tokens(source)?.into_iter().collect();
    Ok(scope_imports(&tokens, &child_scope(parent)).0)
}

/// Whether a `cfg` predicate holds under `cfg(test)`-only compilation in
/// the sense that matters here: it names `test` outside any `not(..)`.
fn predicate_names_test(predicate: TokenStream) -> bool {
    let tokens: Vec<TokenTree> = predicate.into_iter().collect();
    let mut at = 0;
    while at < tokens.len() {
        match &tokens[at] {
            TokenTree::Ident(ident) if ident == "not" => {
                at += 2;
                continue;
            }
            TokenTree::Ident(ident) if ident == "test" && !is_punct(tokens.get(at + 1), '=') => {
                return true;
            }
            TokenTree::Group(group) if predicate_names_test(group.stream()) => return true,
            _ => {}
        }
        at += 1;
    }
    false
}

fn is_test_attribute(attribute: &syn::Attribute) -> bool {
    let path = attribute.path();
    if path.is_ident("cfg") {
        return match &attribute.meta {
            syn::Meta::List(list) => predicate_names_test(list.tokens.clone()),
            _ => false,
        };
    }
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
}

fn item_attributes(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::ExternCrate(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        syn::Item::ForeignMod(item) => &item.attrs,
        syn::Item::Impl(item) => &item.attrs,
        syn::Item::Macro(item) => &item.attrs,
        syn::Item::Mod(item) => &item.attrs,
        syn::Item::Static(item) => &item.attrs,
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Trait(item) => &item.attrs,
        syn::Item::TraitAlias(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        syn::Item::Union(item) => &item.attrs,
        syn::Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

/// What a `src` file's test code is: the line ranges of its test items, and
/// the out-of-line test modules it declares (`#[cfg(test)] mod x;`, with any
/// `#[path]`).
#[derive(Debug, Default)]
pub struct SrcTestCode {
    pub lines: Vec<(usize, usize)>,
    pub modules: Vec<TestModule>,
}

/// An out-of-line test module: the inline modules it is nested in, its name
/// and its `#[path]`, if any.
#[derive(Debug, Clone)]
pub struct TestModule {
    pub inline: Vec<String>,
    pub name: String,
    pub path: Option<String>,
}

fn collect_test_code(items: &[syn::Item], inline: &[String], code: &mut SrcTestCode) {
    use syn::spanned::Spanned;
    for item in items {
        let attributes = item_attributes(item);
        let test = attributes.iter().any(is_test_attribute);
        if test {
            let span = item.span();
            code.lines.push((span.start().line, span.end().line));
        }
        if let syn::Item::Mod(module) = item {
            match &module.content {
                Some((_, inner)) if !test => {
                    let mut nested = inline.to_vec();
                    nested.push(module.ident.to_string());
                    collect_test_code(inner, &nested, code);
                }
                None if test => {
                    let path = attributes.iter().find_map(|attribute| {
                        let syn::Meta::NameValue(value) = &attribute.meta else {
                            return None;
                        };
                        if !value.path.is_ident("path") {
                            return None;
                        }
                        match &value.value {
                            syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(path),
                                ..
                            }) => Some(path.value()),
                            _ => None,
                        }
                    });
                    code.modules.push(TestModule {
                        inline: inline.to_vec(),
                        name: module.ident.to_string(),
                        path,
                    });
                }
                _ => {}
            }
        }
    }
}

/// The test code of a `src` file.
pub fn src_test_code(source: &str) -> Result<SrcTestCode, String> {
    let file = syn::parse_file(source).map_err(|error| format!("not a Rust file: {error}"))?;
    let mut code = SrcTestCode::default();
    if file.attrs.iter().any(is_test_attribute) {
        code.lines.push((1, source.lines().count().max(1)));
    }
    collect_test_code(&file.items, &[], &mut code);
    Ok(code)
}

/// The forbidden waits in the test code of a `src` file, as `(line, call)`.
pub fn scan_src_source(source: &str) -> Result<Vec<(usize, String)>, String> {
    scan_src_source_in(source, &child_scope(&Imports::default()))
}

/// [`scan_src_source`] for a file whose code starts with `scope` in scope.
fn scan_src_source_in(source: &str, scope: &Imports) -> Result<Vec<(usize, String)>, String> {
    let code = src_test_code(source)?;
    let paused = paused_test_lines(source)?;
    let mut found = Vec::new();
    let lines = Lines {
        counts: &|line| within(&code.lines, line),
        paused: &|line| within(&paused, line),
    };
    walk(tokens(source)?, scope, &Locals::new(), &lines, &mut found);
    found.sort_by_key(|&(line, _)| line);
    Ok(found)
}

/// The files `source` (the file at `file`) pulls in with
/// `include!("path")`, the path relative to `file`'s directory.
fn included_files(file: &Path, source: &str) -> Result<Vec<PathBuf>, String> {
    fn collect(tokens: &[TokenTree], directory: &Path, found: &mut Vec<PathBuf>) {
        for (at, token) in tokens.iter().enumerate() {
            if let TokenTree::Group(group) = token {
                let inner: Vec<TokenTree> = group.stream().into_iter().collect();
                collect(&inner, directory, found);
                continue;
            }
            if ident(Some(token)).as_deref() != Some("include")
                || !is_punct(tokens.get(at + 1), '!')
            {
                continue;
            }
            let Some(TokenTree::Group(arguments)) = tokens.get(at + 2) else {
                continue;
            };
            let literal = arguments.stream().into_iter().next();
            if let Some(TokenTree::Literal(literal)) = literal {
                if let Ok(syn::Lit::Str(path)) = syn::parse_str::<syn::Lit>(&literal.to_string()) {
                    found.push(directory.join(path.value()));
                }
            }
        }
    }
    let directory = file
        .parent()
        .ok_or_else(|| format!("{} has no directory", file.display()))?;
    let tokens: Vec<TokenTree> = tokens(source)?.into_iter().collect();
    let mut found = Vec::new();
    collect(&tokens, directory, &mut found);
    Ok(found)
}

/// The scope `file`'s code starts with: its includer's imports when another
/// file pulls it in with `include!`, else nothing imported and `super`
/// being its parent module's imports.
fn starting_scope(
    file: &Path,
    root: &Path,
    includers: &BTreeMap<PathBuf, PathBuf>,
    cache: &mut BTreeMap<PathBuf, Imports>,
) -> Result<Imports, String> {
    match includers.get(file) {
        Some(includer) => module_imports(includer, root, cache),
        None => Ok(child_scope(&super_imports(file, root, cache)?)),
    }
}

/// The module file that declares the module in `file`, by path convention
/// (`dir/x.rs` is declared in `dir.rs` or `dir/mod.rs`; a module directory's
/// `mod.rs` in its parent directory's owner), never above `root` (the
/// crate's `tests` or `src` directory).
fn parent_module_file(file: &Path, root: &Path) -> Option<PathBuf> {
    let directory = file.parent()?;
    let owns_directory = matches!(
        file.file_name().and_then(|name| name.to_str()),
        Some("lib.rs" | "main.rs" | "mod.rs")
    );
    let declaring_directory = if owns_directory {
        if directory == root {
            return None;
        }
        directory.parent()?
    } else {
        directory
    };
    if !declaring_directory.starts_with(root) {
        return None;
    }
    let owners = ["mod.rs", "lib.rs", "main.rs"]
        .into_iter()
        .map(|owner| declaring_directory.join(owner));
    let sibling = (declaring_directory != root).then(|| declaring_directory.with_extension("rs"));
    owners
        .chain(sibling)
        .find(|candidate| candidate.as_path() != file && candidate.is_file())
}

/// The top-level imports of `file`, with `super` resolved through its
/// parent module files (cached by path).
fn module_imports(
    file: &Path,
    root: &Path,
    cache: &mut BTreeMap<PathBuf, Imports>,
) -> Result<Imports, String> {
    if let Some(imports) = cache.get(file) {
        return Ok(imports.clone());
    }
    let parent = match parent_module_file(file, root) {
        Some(parent) => module_imports(&parent, root, cache)?,
        None => Imports::default(),
    };
    let source = std::fs::read_to_string(file)
        .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
    let imports = file_imports(&source, &parent)?;
    cache.insert(file.to_path_buf(), imports.clone());
    Ok(imports)
}

/// What `super` stands for in `file`.
fn super_imports(
    file: &Path,
    root: &Path,
    cache: &mut BTreeMap<PathBuf, Imports>,
) -> Result<Imports, String> {
    match parent_module_file(file, root) {
        Some(parent) => module_imports(&parent, root, cache),
        None => Ok(Imports::default()),
    }
}

/// The `.rs` files under `directory`, sorted.
fn rust_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
                .path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// The file an out-of-line test module declared in `parent` lives in.
fn module_file(parent: &Path, module: &TestModule) -> Result<PathBuf, String> {
    let directory = parent
        .parent()
        .ok_or_else(|| format!("{} has no directory", parent.display()))?;
    let owns_directory = matches!(
        parent.file_name().and_then(|file| file.to_str()),
        Some("lib.rs" | "main.rs" | "mod.rs")
    );
    let mut base = if owns_directory {
        directory.to_path_buf()
    } else {
        let stem = parent
            .file_stem()
            .ok_or_else(|| format!("{} has no file name", parent.display()))?;
        directory.join(stem)
    };
    if let Some(path) = &module.path {
        let base = if module.inline.is_empty() {
            directory.to_path_buf()
        } else {
            module
                .inline
                .iter()
                .fold(base, |base, name| base.join(name))
        };
        return Ok(base.join(path));
    }
    for name in &module.inline {
        base = base.join(name);
    }
    [
        base.join(format!("{}.rs", module.name)),
        base.join(&module.name).join("mod.rs"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
    .ok_or_else(|| {
        format!(
            "{} declares the test module {} but its file is not found",
            parent.display(),
            module.name
        )
    })
}

/// Every forbidden wait in the test code of `crate_directory`: its whole
/// `tests/` tree, and the test code of its `src` files (out-of-line test
/// modules count as test code throughout).
pub fn crate_wall_clock_waits(crate_directory: &Path) -> Result<Vec<WallClockWait>, String> {
    let mut found = Vec::new();
    let mut report = |file: &Path, waits: Vec<(usize, String)>| {
        found.extend(waits.into_iter().map(|(line, call)| WallClockWait {
            file: file.to_path_buf(),
            line,
            call,
        }));
    };
    let read = |file: &Path| {
        std::fs::read_to_string(file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))
    };
    let mut cache = BTreeMap::new();
    let includers_of = |files: &[PathBuf]| -> Result<BTreeMap<PathBuf, PathBuf>, String> {
        let mut includers = BTreeMap::new();
        for file in files {
            let source = std::fs::read_to_string(file)
                .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
            for included in included_files(file, &source)
                .map_err(|error| format!("{}: {error}", file.display()))?
            {
                includers.insert(included, file.clone());
            }
        }
        Ok(includers)
    };
    let tests = crate_directory.join("tests");
    if tests.is_dir() {
        let files = rust_files(&tests)?;
        let includers = includers_of(&files)?;
        for file in files {
            let scope = starting_scope(&file, &tests, &includers, &mut cache)?;
            let waits = scan_test_source_in(&read(&file)?, &scope)
                .map_err(|error| format!("{}: {error}", file.display()))?;
            report(&file, waits);
        }
    }
    let src = crate_directory.join("src");
    if src.is_dir() {
        let files = rust_files(&src)?;
        let mut test_files: Vec<PathBuf> = Vec::new();
        for file in &files {
            let code = src_test_code(&read(file)?)
                .map_err(|error| format!("{}: {error}", file.display()))?;
            for module in &code.modules {
                test_files.push(module_file(file, module)?);
            }
        }
        // A test module's own submodules are test code too.
        let mut index = 0;
        while index < test_files.len() {
            let file = test_files[index].clone();
            let module_directory = match file.file_name().and_then(|name| name.to_str()) {
                Some("mod.rs") => file.parent().map(Path::to_path_buf),
                _ => file
                    .parent()
                    .zip(file.file_stem())
                    .map(|(parent, stem)| parent.join(stem)),
            };
            if let Some(directory) = module_directory.filter(|directory| directory.is_dir()) {
                for nested in rust_files(&directory)? {
                    if !test_files.contains(&nested) {
                        test_files.push(nested);
                    }
                }
            }
            index += 1;
        }
        let includers = includers_of(&files)?;
        for file in &files {
            let source = read(file)?;
            let scope = starting_scope(file, &src, &includers, &mut cache)?;
            let waits = if test_files.contains(file) {
                scan_test_source_in(&source, &scope)
            } else {
                scan_src_source_in(&source, &scope)
            }
            .map_err(|error| format!("{}: {error}", file.display()))?;
            report(file, waits);
        }
    }
    found.sort();
    Ok(found)
}
