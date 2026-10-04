//! Navigator — render entry point (extracted from `navigator.rs` so
//! that file stays under the 300-LoC ceiling pinned by
//! `source_shape_rpc013`).
//!
//! Feature files:
//! spec/features/rpc013-source-shape.feature,
//! spec/features/mux-surface-single-top-of-mux-menu-bar-per-pane-suppression-spinner-draw-tick.feature
//!
//! Renders EXACTLY ONE child view per frame — either BoardView OR
//! AgentView OR the mux grid — over the full area, then caches the
//! MENU-004 R-TICK "menu bar animating" gate for this frame.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::store::{AgentViewStore, BoardStore};
use crate::views::multiplex::render as mux_render;

use super::navigator::Navigator;
use super::ViewMode;

impl Navigator {
    /// Render against the live stores. Caller is App.
    ///
    /// RPC-013: the active child receives the full `area` — the
    /// Navigator no longer reserves a 1-row footer chunk because each
    /// view now paints its own view-specific footer.
    pub fn render_with_stores(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        board_store: &BoardStore,
        agent_store: &mut AgentViewStore,
    ) {
        match self.active_view {
            ViewMode::Board => {
                self.board
                    .render_with_store(area, buf, board_store, agent_store, false);
            }
            ViewMode::Agent => {
                self.agent.render_with_store(area, buf, agent_store);
            }
            ViewMode::ProviderSettings => {
                self.provider_settings.render(area, buf);
            }
            ViewMode::Blocklist => {
                let empty = std::collections::HashSet::new();
                let disabled = agent_store
                    .current_session()
                    .and_then(|sid| agent_store.blocklist_disabled_for(sid))
                    .unwrap_or(&empty);
                self.blocklist.render(area, buf, disabled);
            }
            ViewMode::ModelSelector => {
                self.model_selector.render(area, buf);
            }
            ViewMode::ChangedFiles => {
                self.changed_files.render(area, buf);
            }
            ViewMode::Checkpoints => {
                self.checkpoints.render(area, buf);
            }
            ViewMode::Mux => {
                mux_render::render_with_stores(
                    &mut self.mux,
                    area,
                    buf,
                    board_store,
                    agent_store,
                    &mut mux_render::MuxRenderViews {
                        board: &self.board,
                        agent: &mut self.agent,
                        changed_files: &mut self.changed_files,
                        checkpoints: &mut self.checkpoints,
                    },
                    &self.board.theme,
                );
            }
        }
        // MENU-004 R-TICK: cache the "menu bar animating" gate from this
        // frame (the active surface's bar-geometry caches are now
        // current — Board/Agent/Mux only).
        self.recompute_menu_bar_gate(agent_store);
    }
}
