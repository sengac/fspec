//! BUG-393: find per-file line ceilings that measure module-tree text.

use syn::visit::Visit;

/// Every function in `source` that enforces a line ceiling
/// (`.lines().count()` compared with a numeric literal) on text read with
/// `read_tree_or_file` / `read_module_tree`, directly or through a local
/// helper function. A source that does not parse is an error.
pub fn tree_measured_line_ceilings(source: &str) -> Result<Vec<String>, String> {
    let file =
        syn::parse_file(source).map_err(|error| format!("cannot parse test source: {error}"))?;
    let mut collector = Functions {
        lines: source.lines().collect(),
        functions: Vec::new(),
    };
    collector.visit_file(&file);
    let tree_readers: Vec<&str> = collector
        .functions
        .iter()
        .filter(|(_, body)| reads_tree(body))
        .map(|(name, _)| name.as_str())
        .collect();
    let mut found: Vec<String> = collector
        .functions
        .iter()
        .filter(|(_, body)| enforces_line_ceiling(body))
        .filter(|(_, body)| {
            reads_tree(body)
                || tree_readers
                    .iter()
                    .any(|reader| body.contains(&format!("{reader}(")))
        })
        .map(|(name, _)| name.clone())
        .collect();
    found.sort();
    found.dedup();
    Ok(found)
}

fn reads_tree(body: &str) -> bool {
    body.contains("read_tree_or_file(") || body.contains("read_module_tree(")
}

fn enforces_line_ceiling(body: &str) -> bool {
    body.contains(".lines().count()")
        && ["<", "<=", ">", ">="].iter().any(|op| {
            body.match_indices(op).any(|(at, _)| {
                body[at + op.len()..]
                    .trim_start_matches('=')
                    .starts_with(|c: char| c.is_ascii_digit())
            })
        })
}

/// Function name → body text (from the source, via span locations) with
/// whitespace removed.
struct Functions<'a> {
    lines: Vec<&'a str>,
    functions: Vec<(String, String)>,
}

impl<'ast> Visit<'ast> for Functions<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let open = item.block.brace_token.span.open().start();
        let close = item.block.brace_token.span.close().end();
        let body: String = self
            .lines
            .iter()
            .skip(open.line.saturating_sub(1))
            .take(close.line.saturating_sub(open.line) + 1)
            .flat_map(|line| line.chars())
            .filter(|c| !c.is_whitespace())
            .collect();
        self.functions.push((item.sig.ident.to_string(), body));
        syn::visit::visit_item_fn(self, item);
    }
}
