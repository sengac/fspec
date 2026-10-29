use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::ops::Range;

use proc_macro2::{Group, TokenStream, TokenTree};

use super::{invalid, spans};

pub(super) fn expand(
    source: &str,
    definitions: &BTreeMap<String, &syn::ItemMacro>,
    entry: &str,
    used: &mut BTreeSet<String>,
) -> io::Result<String> {
    let mut current = entry.to_string();
    let mut members = String::new();
    loop {
        if !used.insert(current.clone()) {
            return Err(invalid("cyclic or reused composition stage"));
        }
        let definition = definitions
            .get(&current)
            .ok_or_else(|| invalid(format!("missing composition stage {current}")))?;
        let body = body(&definition.mac.tokens)?;
        if let Ok(next) = syn::parse2::<syn::Macro>(body.stream()) {
            let name = next
                .path
                .get_ident()
                .ok_or_else(|| invalid("qualified stage"))?;
            if !name.to_string().starts_with("compose_") {
                return Err(invalid("composition successor must be a composition macro"));
            }
            let forwards = forwards(source, next.tokens.clone())?;
            if forwards.len() != 1 {
                return Err(invalid("composition must forward exactly once"));
            }
            let first = next
                .tokens
                .clone()
                .into_iter()
                .next()
                .ok_or_else(|| invalid("empty composition stage"))?;
            if spans::range(source, first.span())?.start != forwards[0].start {
                return Err(invalid("composition must forward before appending"));
            }
            let last = next
                .tokens
                .into_iter()
                .last()
                .ok_or_else(|| invalid("empty composition stage"))?;
            let end = spans::range(source, last.span())?.end;
            members.push_str(&source[forwards[0].end..end]);
            members.push('\n');
            current = name.to_string();
        } else {
            let forwards = forwards(source, body.stream())?;
            if forwards.len() != 1 {
                return Err(invalid(
                    "terminal must insert accumulated members exactly once",
                ));
            }
            let start = spans::offset(source, body.span_open().end())?;
            let end = spans::offset(source, body.span_close().start())?;
            let replacement = forwards[0].start - start..forwards[0].end - start;
            let completed =
                spans::replace(source[start..end].to_string(), vec![(replacement, members)])?;
            let parsed = syn::parse_file(&completed).map_err(invalid)?;
            if parsed.items.len() != 1
                || !matches!(
                    parsed.items[0],
                    syn::Item::Enum(_) | syn::Item::Trait(_) | syn::Item::Impl(_)
                )
            {
                return Err(invalid(
                    "terminal must emit one whole enum trait or implementation",
                ));
            }
            return Ok(completed);
        }
    }
}

fn body(tokens: &TokenStream) -> io::Result<Group> {
    let tokens = tokens.clone().into_iter().collect::<Vec<_>>();
    match tokens.as_slice() {
        [TokenTree::Group(matcher), TokenTree::Punct(equal), TokenTree::Punct(arrow), TokenTree::Group(body), rest @ ..]
            if matcher.stream().to_string().replace(' ', "") == "$($collected:tt)*"
                && equal.as_char() == '='
                && arrow.as_char() == '>'
                && (rest.is_empty()
                    || matches!(rest, [TokenTree::Punct(end)] if end.as_char() == ';')) =>
        {
            Ok(body.clone())
        }
        _ => Err(invalid("unsupported composition rule")),
    }
}

fn forwards(source: &str, stream: TokenStream) -> io::Result<Vec<Range<usize>>> {
    let tokens = stream.into_iter().collect::<Vec<_>>();
    let mut result = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if let Some([TokenTree::Punct(dollar), TokenTree::Group(group), TokenTree::Punct(star)]) =
            tokens.get(index..index + 3)
        {
            if dollar.as_char() == '$'
                && star.as_char() == '*'
                && group.stream().to_string().replace(' ', "") == "$collected"
            {
                result.push(
                    spans::range(source, dollar.span())?.start
                        ..spans::range(source, star.span())?.end,
                );
                index += 3;
                continue;
            }
        }
        if let TokenTree::Group(group) = &tokens[index] {
            result.extend(forwards(source, group.stream())?);
        }
        index += 1;
    }
    Ok(result)
}
