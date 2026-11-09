//! AMGR-022 — the root view's relayed prompt and every chain view's
//! waiting rows.
//!
//! Feature: spec/features/a-sub-agents-request-shows-in-the-root-view.feature
//!
//! A sub-agent's request held by the user is drawn in the view of the
//! highest session of its chain: a header row naming who asks ("sub-agent
//! #2 of bot #1 asks"), then the origin's own HITL or pause prompt. The
//! prompt caches name the ORIGIN session, so the existing key routing sends
//! the id-checked reply (AMGR-020) to the sub-agent that asked. A view's own
//! question or approval is painted before this (input_area.rs order).
//!
//! Every view above the origin shows one yellow waiting row per pending
//! request ("Waiting: sub-agent #2 asks for approval; agent #1 holds it"),
//! above the composer.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};

use codelet_rpc_types::{PauseState, SessionId};

use crate::store::agent_view::hitl_state::HitlPromptState;
use crate::store::AgentViewStore;

use super::super::hitl_keys::HitlKeyMode;
use super::super::multiline_input::MultiLineInput;
use super::super::text_wrap::wrapped_lines;
use super::super::{hitl_prompt, multiline_input_render, pause_prompt, AgentView};

/// The origin's prompt a root view shows for a relayed request.
enum OriginPrompt<'a> {
    Hitl(&'a HitlPromptState),
    Pause(&'a PauseState),
}

/// The relayed request `sid` shows: its origin, header and prompt.
fn relayed<'a>(
    store: &'a AgentViewStore,
    sid: Option<&SessionId>,
) -> Option<(SessionId, String, OriginPrompt<'a>)> {
    let route = store.relayed_request_for(sid?)?;
    let origin = &route.origin;
    let header = format!("{} asks", store.request_asker(route));
    let hitl = store
        .hitl_prompt_for(origin)
        .filter(|slot| slot.route.request_id == route.request_id);
    let prompt = match hitl {
        Some(slot) => OriginPrompt::Hitl(slot),
        None => OriginPrompt::Pause(store.pause_state_for(origin)?),
    };
    Some((origin.clone(), header, prompt))
}

fn header_lines(header: &str, width: u16) -> Vec<Line<'static>> {
    let style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    wrapped_lines(header, usize::from(width), style)
}

fn rows(lines: &[Line<'static>]) -> u16 {
    u16::try_from(lines.len()).unwrap_or(u16::MAX)
}

/// Input-area rows of the relayed prompt at the padded `width`.
pub(super) fn relayed_height(
    store: &AgentViewStore,
    sid: Option<&SessionId>,
    width: u16,
    input: &MultiLineInput,
) -> Option<u16> {
    let (_, header, prompt) = relayed(store, sid)?;
    let prompt_rows = match prompt {
        OriginPrompt::Hitl(state) => hitl_prompt::prompt_height(state, width, input),
        OriginPrompt::Pause(state) => pause_prompt::prompt_height(state, width),
    };
    Some(rows(&header_lines(&header, width)).saturating_add(prompt_rows))
}

/// The waiting rows of view `sid` wrapped to `width`.
fn waiting_lines(
    store: &AgentViewStore,
    sid: Option<&SessionId>,
    width: u16,
) -> Vec<Line<'static>> {
    let Some(sid) = sid else {
        return Vec::new();
    };
    let style = Style::default().fg(Color::Yellow);
    store
        .waiting_requests_for(sid)
        .into_iter()
        .flat_map(|route| wrapped_lines(&store.waiting_row(route), usize::from(width), style))
        .collect()
}

/// Rows the waiting rows of view `sid` take at `width`.
pub(super) fn waiting_rows(store: &AgentViewStore, sid: Option<&SessionId>, width: u16) -> u16 {
    rows(&waiting_lines(store, sid, width))
}

/// Paint the waiting rows at the top of `area`; returns the rows used.
pub(super) fn paint_waiting(
    area: Rect,
    buf: &mut Buffer,
    store: &AgentViewStore,
    sid: Option<&SessionId>,
) -> u16 {
    let lines = waiting_lines(store, sid, area.width);
    let used = rows(&lines).min(area.height);
    if used > 0 {
        Paragraph::new(lines).render(
            Rect {
                height: used,
                ..area
            },
            buf,
        );
    }
    used
}

impl AgentView {
    /// Paint the relayed prompt of root view `sid` into `padded` (the
    /// padded input area); `false` when the view shows none.
    pub(super) fn paint_relayed_prompt(
        &mut self,
        input_area: Rect,
        padded: Rect,
        buf: &mut Buffer,
        store: &AgentViewStore,
        sid: Option<&SessionId>,
    ) -> bool {
        let Some((origin, header, prompt)) = relayed(store, sid) else {
            return false;
        };
        let lines = header_lines(&header, padded.width);
        let header_rows = rows(&lines).min(padded.height);
        Paragraph::new(lines).render(
            Rect {
                height: header_rows,
                ..padded
            },
            buf,
        );
        let below = Rect {
            y: padded.y.saturating_add(header_rows),
            height: padded.height.saturating_sub(header_rows),
            ..padded
        };
        match prompt {
            OriginPrompt::Hitl(state) => {
                if state.freeform_active() {
                    let body_width = multiline_input_render::input_body_width(input_area.width);
                    self.input.sync_viewport(body_width, input_area.height);
                }
                self.last_hitl_input_offset =
                    hitl_prompt::render_hitl_prompt(below, buf, state, &self.input)
                        .map(|offset| offset.saturating_add(header_rows));
                let mode = if state.freeform_active() {
                    HitlKeyMode::Freeform {
                        other: state.other_active,
                        hint: state.show_empty_hint,
                    }
                } else {
                    HitlKeyMode::Options
                };
                self.last_hitl = Some((origin, mode));
                self.last_pause = None;
            }
            OriginPrompt::Pause(state) => {
                let selection = store.triple_pause_selection_for(&origin);
                pause_prompt::render_pause_prompt(below, buf, state, selection);
                self.last_pause = Some((origin, state.kind));
            }
        }
        true
    }
}
