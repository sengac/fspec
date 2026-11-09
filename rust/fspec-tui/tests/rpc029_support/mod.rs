//! Shared render helpers for the RPC-029 AgentView tests
//! (tests/view_agent_unit_rpc029.rs and tests/rpc029_agent_header_alignment.rs, REFAC-051).

use codelet_fspec_tui::store::AgentViewStore;
use codelet_fspec_tui::views::AgentView;
use codelet_rpc_types::SessionId;
use codelet_test_helpers::OrFail;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

/// Render AgentView against an N×M TestBackend and return the buffer.
pub fn render_buffer(
    width: u16,
    height: u16,
    store: &mut AgentViewStore,
    view: &mut AgentView,
) -> Result<Buffer, codelet_test_helpers::TestError> {
    let backend = TestBackend::new(width, height);
    let mut term = Terminal::new(backend).or_fail("Terminal::new")?;
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .or_fail("draw")?;
    Ok(term.backend().buffer().clone())
}

/// Collect the glyph rows of a buffer.
pub fn rows_of(buf: &Buffer) -> Vec<String> {
    let mut rows = Vec::new();
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows
}

/// Locate the y-coordinate of the first row whose glyph content
/// contains `needle`.
pub fn find_row(buf: &Buffer, needle: &str) -> Option<u16> {
    rows_of(buf)
        .iter()
        .position(|r| r.contains(needle))
        .map(|i| i as u16)
}

/// Locate the (x, y) of the first occurrence of `needle` in any row.
pub fn find_substr_xy(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        if let Some(byte_idx) = row.find(needle) {
            // Convert byte index to character/column index assuming each
            // grapheme rendered as one cell (true for ASCII; the inputs
            // we search for here are ASCII).
            let prefix = &row[..byte_idx];
            let col = prefix.chars().count();
            return Some((col as u16, y));
        }
    }
    None
}

pub fn fresh_view() -> AgentView {
    let (tx, _rx) = unbounded_channel();
    AgentView::new(tx)
}

pub fn fresh_view_with_session(sid: &str) -> (AgentViewStore, AgentView) {
    let mut store = AgentViewStore::default();
    let session = SessionId::new(sid);
    store.append_session(codelet_fspec_tui::SessionContext::new(session));
    (store, fresh_view())
}
