use codelet_core::persistence::compaction_boundary::{
    validate_committed_boundary, CompactionSource,
};
use codelet_core::persistence::{load_session, SessionManifest};
use codelet_tools::session_search::originals::OriginalsError;
use codelet_tools::session_search::types::{SessionSearchAction, SessionSearchResult};
use codelet_tools::session_search::SessionSearchHandler;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct CompactionEvidence {
    source: CompactionSource,
    existing_dag: Option<(String, usize)>,
}

impl CompactionEvidence {
    pub fn capture(source: CompactionSource) -> Result<Self, String> {
        let manifest = load_session(source.session_id)?;
        validate_committed_boundary(&manifest)?;
        let evidence = Self {
            source,
            existing_dag: manifest.compaction.as_ref().map(|state| {
                (
                    state.summary.clone(),
                    state.compacted_before_index.saturating_sub(1),
                )
            }),
        };
        evidence.verify(&manifest)?;
        Ok(evidence)
    }

    pub(crate) fn source(&self) -> &CompactionSource {
        &self.source
    }

    pub fn existing_dag(&self) -> Option<(String, usize)> {
        self.existing_dag.clone()
    }

    fn verify(&self, manifest: &SessionManifest) -> Result<(), String> {
        let generation = manifest
            .compaction_commit
            .as_ref()
            .map_or(0, |commit| commit.identity.generation);
        let prefix_matches = manifest.messages.len() >= self.source.prefix.len()
            && manifest
                .messages
                .iter()
                .zip(&self.source.prefix)
                .all(|(reference, identity)| reference.message_id == *identity);
        if generation != self.source.generation
            || manifest.provider != self.source.provider
            || !prefix_matches
            || manifest.compaction.as_ref().map(|state| &state.summary)
                != self.existing_dag.as_ref().map(|(content, _)| content)
        {
            return Err("captured compaction evidence is no longer authoritative".into());
        }
        Ok(())
    }

    pub fn search_handler(&self, project: PathBuf) -> SessionSearchHandler {
        let evidence = self.clone();
        let previews = crate::session_search_handler::create_handler(
            project,
            Arc::new(std::sync::atomic::AtomicBool::new(true)),
        );
        Arc::new(move |mut action, _| {
            if let Err(message) = load_session(evidence.source.session_id)
                .and_then(|manifest| evidence.verify(&manifest))
            {
                return SessionSearchResult::Error { message };
            }
            let target = evidence.source.session_id;
            let length = evidence.source.prefix.len();
            let end = length.saturating_sub(1);
            match &mut action {
                SessionSearchAction::Originals { request } => {
                    if request.session_id.parse::<uuid::Uuid>().ok() != Some(target) {
                        return Err(OriginalsError::InvalidRequest {
                            message: "compaction originals must target the captured session".into(),
                        })
                        .into();
                    }
                    if length == 0 {
                        return Err(OriginalsError::InvalidRequest {
                            message: "captured compaction prefix contains no originals".into(),
                        })
                        .into();
                    }
                    request.end_turn = Some(request.end_turn.unwrap_or(end).min(end));
                    let result =
                        codelet_core::persistence::originals::retrieve_originals(request.clone())
                            .and_then(|page| {
                                if page.records.iter().any(|record| {
                                    evidence.source.prefix.get(record.turn_index)
                                        != Some(&record.message_id)
                                }) {
                                    return Err(OriginalsError::SourceUnavailable {
                                        message: "captured original identity changed".into(),
                                    });
                                }
                                Ok(page)
                            });
                    return result.into();
                }
                SessionSearchAction::Show {
                    session_id,
                    end_turn,
                    ..
                } => {
                    if session_id.as_deref().is_some_and(|id| {
                        id != "current" && id.parse::<uuid::Uuid>().ok() != Some(target)
                    }) {
                        return SessionSearchResult::Error {
                            message: "compaction previews must target the captured session".into(),
                        };
                    }
                    *session_id = Some(target.to_string());
                    *end_turn = Some(end_turn.unwrap_or(end).min(end));
                }
                SessionSearchAction::Search { end_turn, .. } => {
                    *end_turn = Some(end_turn.unwrap_or(end).min(end));
                }
                SessionSearchAction::Recent { .. } => {
                    return SessionSearchResult::Error {
                        message: format!("compaction evidence is scoped to session {target}"),
                    };
                }
            }
            let mut result = previews(action, target);
            match &mut result {
                SessionSearchResult::Session { messages, .. } => {
                    messages.retain(|message| message.turn_index < length);
                }
                SessionSearchResult::Search {
                    groups,
                    total_matches,
                    ..
                } => {
                    groups.retain_mut(|group| {
                        if group.session_id != target.to_string() {
                            return false;
                        }
                        group.message_count = length;
                        group.matches.retain(|matched| matched.turn_index < length);
                        if let Some(context) = &mut group.context {
                            context.retain(|turn| turn.turn_index < length);
                        }
                        !group.matches.is_empty()
                    });
                    *total_matches = groups.iter().map(|group| group.matches.len()).sum();
                }
                _ => {}
            }
            result
        })
    }
}
