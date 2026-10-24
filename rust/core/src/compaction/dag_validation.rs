use super::dag_syntax::{parse_files, parse_nodes};
use super::{DagDepth, DagNodeMeta};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, Default)]
pub struct DagValidationContext {
    pub persisted_turn_count: usize,
    pub required_coverage_end: Option<usize>,
    pub existing_dag: Option<String>,
    pub required_files: BTreeMap<String, String>,
    pub required_reminders: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DagValidationError {
    #[error("invalid DAG source: {0}")]
    InvalidSource(String),
    #[error("invalid DAG candidate: {0}")]
    InvalidCandidate(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedDagCandidate {
    pub content: String,
    pub nodes: Vec<DagNodeMeta>,
}

pub fn validate_dag_structure(content: &str) -> Result<Vec<DagNodeMeta>, DagValidationError> {
    let nodes = parse_nodes(content).map_err(DagValidationError::InvalidCandidate)?;
    parse_files(content).map_err(DagValidationError::InvalidCandidate)?;
    Ok(nodes.into_iter().map(|node| node.metadata).collect())
}

pub fn validate_dag_candidate(
    content: &str,
    context: &DagValidationContext,
) -> Result<ValidatedDagCandidate, DagValidationError> {
    if context
        .required_reminders
        .iter()
        .any(|reminder| reminder.trim().is_empty())
    {
        return Err(DagValidationError::InvalidSource(
            "empty authoritative reminder".into(),
        ));
    }
    let source = context
        .existing_dag
        .as_deref()
        .map(parse_nodes)
        .transpose()
        .map_err(DagValidationError::InvalidSource)?;
    let source_files = context
        .existing_dag
        .as_deref()
        .map(parse_files)
        .transpose()
        .map_err(DagValidationError::InvalidSource)?
        .unwrap_or_default();
    if context
        .required_coverage_end
        .is_some_and(|end| end >= context.persisted_turn_count)
    {
        return Err(DagValidationError::InvalidSource(
            "required coverage exceeds persisted history".into(),
        ));
    }
    let mut required_files = source_files;
    required_files.extend(context.required_files.clone());
    if required_files.iter().any(|(path, operation)| {
        path.trim().is_empty()
            || path.contains(['\n', '\r', '<', '>'])
            || !matches!(operation.as_str(), "Created" | "Modified" | "Deleted")
    }) {
        return Err(DagValidationError::InvalidSource(
            "invalid authoritative file identity".into(),
        ));
    }
    let assembled = if !required_files.is_empty()
        && !content.contains("<dag-files")
        && !content.contains("</dag-files")
    {
        let entries = required_files
            .iter()
            .map(|(path, operation)| format!("\n- {path} ({operation})"))
            .collect::<String>();
        format!("{content}\n<dag-files>{entries}\n</dag-files>")
    } else {
        content.to_string()
    };
    let content = assembled.as_str();
    if source.as_ref().is_some_and(|nodes| {
        nodes
            .iter()
            .any(|node| node.metadata.turn_end >= context.persisted_turn_count)
    }) {
        return Err(DagValidationError::InvalidSource(
            "source provenance exceeds persisted history".into(),
        ));
    }
    let nodes = parse_nodes(content).map_err(DagValidationError::InvalidCandidate)?;
    let files = parse_files(content).map_err(DagValidationError::InvalidCandidate)?;
    if nodes
        .iter()
        .any(|node| node.metadata.turn_end >= context.persisted_turn_count)
    {
        return Err(DagValidationError::InvalidCandidate(
            "provenance exceeds persisted history".into(),
        ));
    }
    if let Some(required_end) = context.required_coverage_end {
        let mut next = 0usize;
        for node in &nodes {
            if node.metadata.turn_start > next {
                break;
            }
            next = next.max(node.metadata.turn_end.saturating_add(1));
        }
        if next <= required_end {
            return Err(DagValidationError::InvalidCandidate(
                "required persisted prefix is not covered".into(),
            ));
        }
    }
    for durable in source
        .iter()
        .flatten()
        .filter(|node| node.metadata.depth == DagDepth::D2)
    {
        if !nodes
            .iter()
            .any(|node| node.metadata == durable.metadata && node.body == durable.body)
        {
            return Err(DagValidationError::InvalidCandidate(
                "required durable node changed or missing".into(),
            ));
        }
    }
    if required_files
        .iter()
        .any(|(path, operation)| files.get(path) != Some(operation))
    {
        return Err(DagValidationError::InvalidCandidate(
            "required file identity changed or missing".into(),
        ));
    }
    if context
        .required_reminders
        .iter()
        .any(|reminder| !content.contains(reminder))
    {
        return Err(DagValidationError::InvalidCandidate(
            "required reminder changed or missing".into(),
        ));
    }
    Ok(ValidatedDagCandidate {
        content: content.to_string(),
        nodes: nodes.into_iter().map(|node| node.metadata).collect(),
    })
}
