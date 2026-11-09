//! The exact filters of a board query (CLI-017): OR within a filter, AND
//! across filters. Building a [`Query`] validates every named value.
//!
//! Feature: spec/features/board-queries-with-list-work-units.feature

use std::collections::{HashMap, HashSet};
use std::path::Path;

use indexmap::IndexMap;

use super::args::{Args, List};
use super::card::{Card, Time};
use super::clock::BoardClock;
use super::time::{self, Range};
use super::{STATUSES, TYPES};
use crate::types::work_unit::{WorkUnit, WorkUnitStatus};

/// The registered values a filter may name.
pub(super) struct Known<'a> {
    pub(super) board: &'a IndexMap<String, WorkUnit>,
    pub(super) prefixes: Vec<String>,
    pub(super) epics: Vec<String>,
    pub(super) project_root: &'a Path,
}

enum IdSel {
    One(String),
    Span(String, u64, u64),
}

enum Link {
    Any,
    Ids(Vec<String>),
}

/// The time a day or week group is keyed on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum TimeKind {
    Created,
    Updated,
    Done,
    Entered,
}

/// card id -> the Feature-line and scenario tags of its feature files.
type TagsByCard = HashMap<String, HashSet<String>>;

pub(super) struct Query {
    statuses: Option<Vec<String>>,
    types: Option<Vec<String>>,
    prefixes: Option<Vec<String>>,
    epics: Option<Vec<String>>,
    ids: Option<Vec<IdSel>>,
    parent: Option<String>,
    has_children: bool,
    estimate_min: Option<i64>,
    estimate_max: Option<i64>,
    has_estimate: bool,
    no_estimate: bool,
    created: Range,
    updated: Range,
    done: Range,
    entered: Option<(String, Range)>,
    text: Option<String>,
    title: Option<String>,
    has_open_questions: bool,
    has_assumptions: bool,
    has_rules: bool,
    no_rules: bool,
    has_examples: bool,
    has_attachments: bool,
    has_architecture_notes: bool,
    tags: Option<(Vec<String>, TagsByCard)>,
    links: Vec<(&'static str, Link)>,
    blocked: bool,
}

fn known_list(
    list: &Option<List>,
    what: &str,
    plural: &str,
    valid: &[String],
) -> Result<Option<Vec<String>>, String> {
    let Some(list) = list else { return Ok(None) };
    for v in list.values() {
        if !valid.contains(v) {
            return Err(format!(
                "Unknown {what} '{v}'. Valid {plural}: {}",
                valid.join(", ")
            ));
        }
    }
    Ok(Some(list.values().to_vec()))
}

fn on_board(known: &Known, id: &str) -> Result<(), String> {
    if known.board.contains_key(id) {
        Ok(())
    } else {
        Err(format!("Work unit '{id}' is not on the board"))
    }
}

fn split_id(id: &str) -> Option<(&str, u64)> {
    let (prefix, num) = id.rsplit_once('-')?;
    Some((prefix, num.parse().ok()?))
}

fn id_selectors(list: &List, known: &Known) -> Result<Vec<IdSel>, String> {
    list.values()
        .iter()
        .map(|v| match v.split_once("..") {
            Some((from, to)) => {
                let bad = || format!("A --work-unit range must use one prefix: {v}");
                let (p1, lo) = split_id(from).ok_or_else(bad)?;
                let (p2, hi) = split_id(to).ok_or_else(bad)?;
                if p1 != p2 {
                    return Err(bad());
                }
                if lo > hi {
                    return Err(format!(
                        "A --work-unit range must go from the lower id to the higher: {v}"
                    ));
                }
                Ok(IdSel::Span(p1.to_string(), lo, hi))
            }
            None => {
                on_board(known, v)?;
                Ok(IdSel::One(v.clone()))
            }
        })
        .collect()
}

fn link(list: &Option<List>, known: &Known) -> Result<Option<Link>, String> {
    let Some(list) = list else { return Ok(None) };
    if list.values().len() == 1 && list.values()[0] == "any" {
        return Ok(Some(Link::Any));
    }
    for id in list.values() {
        on_board(known, id)?;
    }
    Ok(Some(Link::Ids(list.values().to_vec())))
}

/// card id -> the tags of every feature file whose Feature line holds @<id>:
/// its Feature-line tags and the tags of each of its scenarios, those inside
/// a Rule included (CLI-018). A card's file that does not parse is an error
/// naming it.
fn feature_tags(root: &Path) -> Result<TagsByCard, String> {
    let files = crate::io::feature_glob::glob_feature_files(root).map_err(|e| e.to_string())?;
    let mut out: TagsByCard = HashMap::new();
    for rel in files {
        let path = root.join(&rel);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        let Some(header) = crate::commands::list_features::parse_feature_header(&content) else {
            continue;
        };
        let cards: Vec<&str> = header
            .tags
            .iter()
            .map(|tag| tag.trim_start_matches('@'))
            .filter(|id| split_id(id).is_some())
            .collect();
        if cards.is_empty() {
            continue;
        }
        let feature = crate::io::gherkin::parse_feature_lenient(&content)
            .map_err(|e| format!("Failed to parse {rel} for --tag: {e}"))?;
        let scenarios = feature
            .scenarios
            .iter()
            .chain(feature.rules.iter().flat_map(|rule| rule.scenarios.iter()));
        let mut tags: HashSet<String> = header.tags.iter().cloned().collect();
        tags.extend(scenarios.flat_map(|s| s.tags.iter().map(|t| format!("@{t}"))));
        for id in cards {
            out.entry(id.to_string())
                .or_default()
                .extend(tags.iter().cloned());
        }
    }
    Ok(out)
}

fn lower(s: &Option<String>) -> Option<String> {
    s.as_ref().map(|s| s.to_lowercase())
}

impl Query {
    pub(super) fn build(a: &Args, known: &Known, clock: &BoardClock) -> Result<Self, String> {
        let strings = |list: &[&str]| list.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        let entered = match (&a.entered, &a.entered_since, &a.entered_until) {
            (None, None, None) => None,
            (None, _, _) => {
                return Err("--entered-since and --entered-until need --entered <status>".into())
            }
            (Some(status), since, until) => {
                if !STATUSES.contains(&status.as_str()) {
                    return Err(format!(
                        "Unknown status '{status}'. Valid statuses: {}",
                        STATUSES.join(", ")
                    ));
                }
                let range = time::range("entered", since.as_deref(), until.as_deref(), clock)?;
                Some((status.clone(), range))
            }
        };
        if let Some(parent) = &a.parent {
            on_board(known, parent)?;
        }
        let mut links = Vec::new();
        for (key, list) in [
            ("dependsOn", &a.depends_on),
            ("blockedBy", &a.blocked_by),
            ("blocks", &a.blocks),
            ("relatesTo", &a.relates_to),
        ] {
            if let Some(l) = link(list, known)? {
                links.push((key, l));
            }
        }
        let tags = match &a.tag {
            None => None,
            Some(list) => {
                for t in list.values() {
                    if !t.starts_with('@') {
                        return Err(format!("A --tag value starts with @: {t}"));
                    }
                }
                Some((list.values().to_vec(), feature_tags(known.project_root)?))
            }
        };
        Ok(Self {
            statuses: known_list(&a.status, "status", "statuses", &strings(STATUSES))?,
            types: known_list(&a.r#type, "type", "types", &strings(TYPES))?,
            prefixes: known_list(&a.prefix, "prefix", "prefixes", &known.prefixes)?,
            epics: known_list(&a.epic, "epic", "epics", &known.epics)?,
            ids: a
                .work_unit_id
                .as_ref()
                .map(|l| id_selectors(l, known))
                .transpose()?,
            parent: a.parent.clone(),
            has_children: a.has_children,
            estimate_min: a.estimate_min,
            estimate_max: a.estimate_max,
            has_estimate: a.has_estimate,
            no_estimate: a.no_estimate,
            created: time::range(
                "created",
                a.created_since.as_deref(),
                a.created_until.as_deref(),
                clock,
            )?,
            updated: time::range(
                "updated",
                a.updated_since.as_deref(),
                a.updated_until.as_deref(),
                clock,
            )?,
            done: time::range(
                "done",
                a.done_since.as_deref(),
                a.done_until.as_deref(),
                clock,
            )?,
            entered,
            text: lower(&a.text),
            title: lower(&a.title),
            has_open_questions: a.has_open_questions,
            has_assumptions: a.has_assumptions,
            has_rules: a.has_rules,
            no_rules: a.no_rules,
            has_examples: a.has_examples,
            has_attachments: a.has_attachments,
            has_architecture_notes: a.has_architecture_notes,
            tags,
            links,
            blocked: a.blocked,
        })
    }

    /// The kinds of time filter in the query.
    pub(super) fn time_kinds(&self) -> Vec<TimeKind> {
        let mut kinds = Vec::new();
        if self.created.is_set() {
            kinds.push(TimeKind::Created);
        }
        if self.updated.is_set() {
            kinds.push(TimeKind::Updated);
        }
        if self.done.is_set() {
            kinds.push(TimeKind::Done);
        }
        if self.entered.is_some() {
            kinds.push(TimeKind::Entered);
        }
        kinds
    }

    /// The time of `kind` on a card that matched the query.
    pub(super) fn time_of(&self, card: &Card, kind: TimeKind) -> Result<Option<Time>, String> {
        match kind {
            TimeKind::Created => card.created(),
            TimeKind::Updated => card.updated(),
            TimeKind::Done => card.done_at(),
            TimeKind::Entered => Ok(self.entered_match(card)?),
        }
    }

    /// The latest transition into the --entered status inside its range.
    fn entered_match(&self, card: &Card) -> Result<Option<Time>, String> {
        let Some((status, range)) = &self.entered else {
            return Ok(None);
        };
        Ok(card
            .history()?
            .iter()
            .rev()
            .find(|(state, at)| state == status && range.contains(at))
            .map(|(_, at)| *at))
    }

    pub(super) fn matches(
        &self,
        card: &Card,
        board: &IndexMap<String, WorkUnit>,
    ) -> Result<bool, String> {
        let wu = card.wu;
        let one_of = |set: &Option<Vec<String>>, v: &str| {
            set.as_ref()
                .is_none_or(|s| s.iter().any(|x| x.as_str() == v))
        };
        if !one_of(&self.statuses, wu.status.as_str())
            || !one_of(&self.types, wu.type_str())
            || !one_of(&self.prefixes, card.prefix())
            || !one_of(&self.epics, wu.epic.as_deref().unwrap_or(""))
        {
            return Ok(false);
        }
        if let Some(sels) = &self.ids {
            let hit = sels.iter().any(|s| match s {
                IdSel::One(id) => wu.id == *id,
                IdSel::Span(p, lo, hi) => split_id(&wu.id)
                    .is_some_and(|(cp, n)| cp == p.as_str() && (*lo..=*hi).contains(&n)),
            });
            if !hit {
                return Ok(false);
            }
        }
        if self
            .parent
            .as_deref()
            .is_some_and(|p| card.text_field("parent") != Some(p))
            || (self.has_children && card.strings("children").is_empty())
        {
            return Ok(false);
        }
        let estimate = card.estimate();
        if (self.has_estimate && estimate.is_none())
            || (self.no_estimate && estimate.is_some())
            || self
                .estimate_min
                .is_some_and(|m| estimate.is_none_or(|e| e < m))
            || self
                .estimate_max
                .is_some_and(|m| estimate.is_none_or(|e| e > m))
        {
            return Ok(false);
        }
        if !self.text_matches(card) || !self.content_matches(card)? {
            return Ok(false);
        }
        if !self.times_match(card)? {
            return Ok(false);
        }
        for (key, l) in &self.links {
            let have = card.strings(key);
            let hit = match l {
                Link::Any => !have.is_empty(),
                Link::Ids(ids) => ids.iter().any(|id| have.contains(id)),
            };
            if !hit {
                return Ok(false);
            }
        }
        if self.blocked && !is_blocked(card, board) {
            return Ok(false);
        }
        if let Some((wanted, by_card)) = &self.tags {
            let tags = by_card.get(&wu.id);
            if !wanted
                .iter()
                .any(|t| tags.is_some_and(|set| set.contains(t)))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn text_matches(&self, card: &Card) -> bool {
        let title = card.wu.title.to_lowercase();
        if self.title.as_ref().is_some_and(|t| !title.contains(t)) {
            return false;
        }
        self.text.as_ref().is_none_or(|t| {
            title.contains(t)
                || card
                    .text_field("description")
                    .is_some_and(|d| d.to_lowercase().contains(t))
        })
    }

    fn content_matches(&self, card: &Card) -> Result<bool, String> {
        let has = |key: &str| -> Result<bool, String> { Ok(!card.active_items(key)?.is_empty()) };
        Ok(
            !(self.has_open_questions && card.open_questions()?.is_empty()
                || self.has_assumptions && !has("assumptions")?
                || self.has_rules && !has("rules")?
                || self.no_rules && has("rules")?
                || self.has_examples && !has("examples")?
                || self.has_architecture_notes && !has("architectureNotes")?
                || self.has_attachments && card.strings("attachments").is_empty()),
        )
    }

    fn times_match(&self, card: &Card) -> Result<bool, String> {
        let within = |range: &Range, at: Option<Time>| {
            !range.is_set() || at.is_some_and(|t| range.contains(&t))
        };
        if self.created.is_set() && !within(&self.created, card.created()?) {
            return Ok(false);
        }
        if self.updated.is_set() && !within(&self.updated, card.updated()?) {
            return Ok(false);
        }
        if self.done.is_set() && !within(&self.done, card.done_at()?) {
            return Ok(false);
        }
        if self.entered.is_some() && self.entered_match(card)?.is_none() {
            return Ok(false);
        }
        Ok(true)
    }
}

/// Status blocked, or a blockedBy card on the board that is not done.
fn is_blocked(card: &Card, board: &IndexMap<String, WorkUnit>) -> bool {
    card.wu.status == WorkUnitStatus::Blocked
        || card.strings("blockedBy").iter().any(|id| {
            board
                .get(id)
                .is_some_and(|b| b.status != WorkUnitStatus::Done)
        })
}
