use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Group, TokenStream, TokenTree};

type ExpansionResult<T> = Result<T, Box<dyn std::error::Error>>;

fn compact(tokens: &TokenStream) -> String {
    tokens.to_string().replace(' ', "")
}

fn body(definition: &syn::ItemMacro) -> ExpansionResult<TokenStream> {
    let tokens = definition.mac.tokens.clone().into_iter().collect::<Vec<_>>();
    match tokens.as_slice() {
        [TokenTree::Group(matcher), TokenTree::Punct(equal), TokenTree::Punct(arrow), TokenTree::Group(body), ..]
            if equal.as_char() == '=' && arrow.as_char() == '>' =>
        {
            if compact(&matcher.stream()) != "$command:expr,$workspace:expr,$forward:ident;$($collected:tt)*" {
                return Err("unsupported dispatch matcher".into());
            }
            if tokens.len() > 5 || (tokens.len() == 5 && tokens[4].to_string() != ";") {
                return Err("dispatch definition must have one rule".into());
            }
            Ok(body.stream())
        }
        _ => Err("invalid dispatch definition".into()),
    }
}

fn sections(tokens: TokenStream) -> ExpansionResult<(TokenStream, Vec<TokenTree>)> {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    let split = tokens.iter().position(|token| token.to_string() == ";")
        .ok_or("dispatch argument separator missing")?;
    Ok((tokens[..split].iter().cloned().collect(), tokens[split + 1..].to_vec()))
}

fn marker(tokens: &[TokenTree]) -> bool {
    matches!(tokens, [TokenTree::Punct(dollar), TokenTree::Group(group), TokenTree::Punct(star)]
        if dollar.as_char() == '$' && star.as_char() == '*'
        && compact(&group.stream()) == "$collected")
}

fn substitute(
    stream: TokenStream,
    replacements: &BTreeMap<&str, TokenStream>,
    accumulated: &TokenStream,
    occurrences: &mut usize,
) -> ExpansionResult<TokenStream> {
    let tokens = stream.into_iter().collect::<Vec<_>>();
    let mut output = TokenStream::new();
    let mut position = 0;
    while position < tokens.len() {
        if tokens.get(position..position + 3).is_some_and(marker) {
            output.extend(accumulated.clone());
            *occurrences += 1;
            position += 3;
            continue;
        }
        if tokens[position].to_string() == "$" {
            let name = tokens.get(position + 1).ok_or("unfinished parameter")?.to_string();
            output.extend(replacements.get(name.as_str()).ok_or("unknown dispatch parameter")?.clone());
            position += 2;
            continue;
        }
        match &tokens[position] {
            TokenTree::Group(group) => output.extend([TokenTree::Group(Group::new(
                group.delimiter(), substitute(group.stream(), replacements, accumulated, occurrences)?,
            ))]),
            token => output.extend([token.clone()]),
        }
        position += 1;
    }
    Ok(output)
}

struct Arguments {
    command: syn::Expr,
    workspace: syn::Expr,
    forward: syn::Ident,
}

impl syn::parse::Parse for Arguments {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let command = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let workspace = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let forward = input.parse()?;
        Ok(Self { command, workspace, forward })
    }
}

pub fn expand(source: &str, invocation: &syn::Macro) -> ExpansionResult<syn::Expr> {
    use quote::ToTokens;

    let file = syn::parse_file(source)?;
    let mut definitions = BTreeMap::new();
    for item in file.items {
        if let syn::Item::Macro(item) = item {
            if item.mac.path.is_ident("macro_rules") {
                if let Some(name) = &item.ident {
                    if name.to_string().starts_with("route_") {
                        if !item.attrs.is_empty() || definitions.insert(name.to_string(), item).is_some() {
                            return Err("ambiguous dispatch definition".into());
                        }
                    }
                }
            }
        }
    }
    let (prefix, initial) = sections(invocation.tokens.clone())?;
    if !initial.is_empty() {
        return Err("dispatch entry must not supply hidden arms".into());
    }
    let arguments = syn::parse2::<Arguments>(prefix)?;
    let replacements = BTreeMap::from([
        ("command", arguments.command.to_token_stream()),
        ("workspace", arguments.workspace.to_token_stream()),
        ("forward", arguments.forward.to_token_stream()),
    ]);
    let mut current = invocation.path.get_ident().ok_or("qualified entry")?.to_string();
    let mut used = BTreeSet::new();
    let mut accumulated = TokenStream::new();
    loop {
        if !used.insert(current.clone()) {
            return Err("cyclic dispatch composition".into());
        }
        let emitted = body(definitions.get(&current).ok_or("missing dispatch stage")?)?;
        if let Ok(next) = syn::parse2::<syn::Macro>(emitted.clone()) {
            let (prefix, members) = sections(next.tokens)?;
            if compact(&prefix) != "$command,$workspace,$forward"
                || !members.get(..3).is_some_and(marker)
            {
                return Err("dispatch must forward parameters and accumulated arms first".into());
            }
            let appended = members[3..].iter().cloned().collect();
            let mut extra = 0;
            accumulated.extend(substitute(appended, &replacements, &TokenStream::new(), &mut extra)?);
            if extra != 0 {
                return Err("dispatch forwards accumulated arms more than once".into());
            }
            current = next.path.get_ident().ok_or("qualified successor")?.to_string();
        } else {
            if compact(&emitted) != "match$command{$($collected)*}" {
                return Err("dispatch terminal must emit the complete original match".into());
            }
            if used.len() != definitions.len() {
                return Err("disconnected dispatch definition".into());
            }
            let mut occurrences = 0;
            let result = substitute(emitted, &replacements, &accumulated, &mut occurrences)?;
            if occurrences != 1 {
                return Err("dispatch terminal must insert arms exactly once".into());
            }
            return Ok(syn::parse2(result)?);
        }
    }
}

pub fn verify_resolver_controls() -> crate::TestResult {
    let matcher = "($command:expr, $workspace:expr, $forward:ident; $($collected:tt)*)";
    let finish = format!("macro_rules! route_finish {{ {matcher} => {{ match $command {{ $($collected)* }} }}; }}");
    let entry: syn::Macro = syn::parse_str("route_first!(cli.cmd, cli.workspace, forward;)")?;
    let valid = format!("macro_rules! route_first {{ {matcher} => {{ route_finish! {{ $command, $workspace, $forward; $($collected)* None => combined::run($workspace).await, }} }}; }} {finish}");
    let actual = expand(&valid, &entry)?;
    let expected: syn::Expr = syn::parse_str("match cli.cmd { None => combined::run(cli.workspace).await, }")?;
    assert_eq!(crate::source::tokens(&actual), crate::source::tokens(&expected));
    for broken in [
        valid.replace("route_finish!", "route_first!"),
        valid.replace("macro_rules! route_finish", "macro_rules! route_missing"),
        valid.replace("macro_rules! route_first", "#[cfg(any())] macro_rules! route_first"),
        valid.replace("$forward; $($collected)*", "$forward;"),
        valid.replace("$forward; $($collected)*", "$forward; $($collected)* $($collected)*"),
        valid.replace("$command, $workspace, $forward;", "$workspace, $command, $forward;"),
        format!("{valid} macro_rules! route_unused {{ {matcher} => {{ unreachable!() }}; }}"),
    ] {
        assert!(expand(&broken, &entry).is_err(), "invalid chain must not satisfy the contract");
    }
    Ok(())
}
