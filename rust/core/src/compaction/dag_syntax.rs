use super::{DagDepth, DagNodeMeta};
use regex::Regex;
use std::collections::BTreeMap;

#[derive(Debug)]
pub(super) struct ParsedNode {
    pub metadata: DagNodeMeta,
    pub body: String,
}

pub(super) fn parse_nodes(content: &str) -> Result<Vec<ParsedNode>, String> {
    let blocks = Regex::new(r"(?s)<dag-node\s+([^<>]*?)>(.*?)</dag-node>")
        .map_err(|error| error.to_string())?;
    let attributes = Regex::new(r#"\s*([a-z-]+)="([^"]*)""#).map_err(|error| error.to_string())?;
    let mut nodes = Vec::new();
    for captures in blocks.captures_iter(content) {
        let mut remaining = &captures[1];
        let mut values = BTreeMap::new();
        while !remaining.trim().is_empty() {
            let matched = attributes
                .captures(remaining)
                .ok_or_else(|| "malformed dag-node attributes".to_string())?;
            let entire = matched.get(0).ok_or("missing attributes")?;
            if entire.start() != 0
                || values
                    .insert(matched[1].to_string(), matched[2].to_string())
                    .is_some()
            {
                return Err("malformed or duplicate dag-node attribute".into());
            }
            remaining = &remaining[entire.end()..];
        }
        if values.len() != 3
            || !values.contains_key("depth")
            || !values.contains_key("turns")
            || !values.contains_key("label")
        {
            return Err("dag-node requires exactly depth, turns and label".into());
        }
        let depth = match values["depth"].as_str() {
            "D0" => DagDepth::D0,
            "D1" => DagDepth::D1,
            "D2" => DagDepth::D2,
            _ => return Err("unsupported dag-node depth".into()),
        };
        let (start, end) = values["turns"]
            .split_once('-')
            .ok_or("invalid turn range")?;
        if !start.bytes().all(|byte| byte.is_ascii_digit())
            || !end.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("invalid turn coordinates".into());
        }
        let turn_start = start.parse::<usize>().map_err(|error| error.to_string())?;
        let turn_end = end.parse::<usize>().map_err(|error| error.to_string())?;
        if turn_start > turn_end
            || values["label"].trim().is_empty()
            || captures[2].trim().is_empty()
        {
            return Err("inverted range or empty dag-node label/body".into());
        }
        if captures[2].contains("<dag-node") || captures[2].contains("<dag-files") {
            return Err("nested DAG structures are unsupported".into());
        }
        nodes.push(ParsedNode {
            metadata: DagNodeMeta {
                depth,
                turn_start,
                turn_end,
                label: values["label"].clone(),
            },
            body: captures[2].to_string(),
        });
    }
    let remainder = blocks.replace_all(content, "");
    if nodes.is_empty() || remainder.contains("<dag-node") || remainder.contains("</dag-node") {
        return Err("missing or unclosed dag-node structure".into());
    }
    nodes.sort_by_key(|node| node.metadata.turn_start);
    let mut previous = [None; 3];
    for node in &nodes {
        let index = match node.metadata.depth {
            DagDepth::D0 => 0,
            DagDepth::D1 => 1,
            DagDepth::D2 => 2,
        };
        let conflict = previous[index].filter(|(_, end)| node.metadata.turn_start <= *end);
        if let Some((start, end)) = conflict {
            return Err(format!(
                "overlapping same-depth dag-node ranges: depth={:?}, ranges={start}-{end} and {}-{} (inclusive)",
                node.metadata.depth, node.metadata.turn_start, node.metadata.turn_end,
            ));
        }
        previous[index] = Some((node.metadata.turn_start, node.metadata.turn_end));
    }
    Ok(nodes)
}

pub(super) fn parse_files(content: &str) -> Result<BTreeMap<String, String>, String> {
    let mut files = BTreeMap::new();
    let Some((before, after)) = content.split_once("<dag-files>") else {
        if content.contains("<dag-files") || content.contains("</dag-files") {
            return Err("malformed dag-files structure".into());
        }
        return Ok(files);
    };
    let (body, tail) = after
        .split_once("</dag-files>")
        .ok_or("unclosed dag-files")?;
    if [before, body, tail]
        .iter()
        .any(|part| part.contains("<dag-files") || part.contains("</dag-files"))
    {
        return Err("duplicate or malformed dag-files structure".into());
    }
    for (index, line) in body.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let invalid_entry = || {
            format!(
                "invalid dag-files line {}: {line:?}; expected a unique - path (Created|Modified|Deleted) entry, or an empty <dag-files></dag-files> section when no files changed",
                index + 1
            )
        };
        let (path, operation) = line
            .strip_prefix("- ")
            .and_then(|line| line.rsplit_once(" ("))
            .ok_or_else(invalid_entry)?;
        let operation = operation.strip_suffix(')').ok_or_else(invalid_entry)?;
        if path.trim().is_empty()
            || !matches!(operation, "Created" | "Modified" | "Deleted")
            || files
                .insert(path.to_string(), operation.to_string())
                .is_some()
        {
            return Err(invalid_entry());
        }
    }
    Ok(files)
}
