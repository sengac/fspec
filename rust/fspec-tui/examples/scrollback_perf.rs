//! BUG-192 research simulation — per-frame cost of an unbounded AgentView
//! scrollback.
//!
//! Drives the REAL production surface (`ScrollbackList::push` +
//! `render_count_visited` with the `RowReflowCache` from BUG-190) with a
//! realistic streaming-session chunk mix (long assistant prose, collapsed
//! tool cards, thinking blocks, user inputs) and measures:
//!
//!   1. per-frame render time (sticky-to-bottom, cold cache, warm cache)
//!      at 1k / 5k / 20k / 50k / 100k / 200k visual rows
//!   2. cold-cache scroll cost — a wheel tick that moves the viewport to a
//!      never-seen offset (the worst realistic steady-state interaction)
//!   3. scroll offset recomputation cost at 200k rows
//!   4. rough memory footprint of the chunk list
//!
//! This is diagnostic scratch (like `probe_pause.rs`) — `println!` is
//! fine here. Run with:
//!
//!   cargo run --release -p codelet-fspec-tui --example scrollback_perf
//!
//! The 16ms render tick (`app/run_loop.rs::RENDER_TICK`) is the budget
//! every measurement below is judged against.

use std::time::Instant;

use codelet_fspec_tui::store::agent_view::chunk_wrap::wrap_source;
use codelet_fspec_tui::views::agent::{ChunkKind, ChunkSource, RenderedChunk, ScrollbackList};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

const VIEWPORT_W: u16 = 200;
const VIEWPORT_H: u16 = 45;

fn push_src(
    list: &mut ScrollbackList,
    seq: &mut u64,
    text: &str,
    color: Color,
    kind: ChunkKind,
) -> usize {
    let source = ChunkSource {
        text: text.to_string(),
        color,
        kind,
        is_streaming: false,
        full_text: None,
    };
    let lines = wrap_source(&source, 80);
    list.push(RenderedChunk {
        seq: *seq,
        lines: lines.clone(),
        source: Some(source),
    });
    *seq += 1;
    lines.len()
}

/// One realistic agent-turn: user input + thinking + assistant prose +
/// tool card (collapsed to ~10 lines like the RPC-389/399 settle window)
/// + trailing assistant prose.
fn turn_text(turn: usize) -> (Vec<(String, Color, ChunkKind)>, usize) {
    let prose_len = 4000 + (turn % 17) * 900; // 4k..~18k chars ≈ 50..230 wrapped lines
    let tool_lines = 60 + (turn % 23) * 10; // body lines; collapses to 8/10 visual lines
    let mut body = String::with_capacity(prose_len + 200);
    for p in 0..4 {
        for w in 0..prose_len / 4 / 50 {
            body.push_str(
                &format!(
                    "This is paragraph {p} word {w} of a fairly typical assistant explanation that runs on for a while before the tool runs. "
                ),
            );
        }
        body.push('\n');
    }
    let tool_body: String = (0..tool_lines)
        .map(|i| format!("tool output line {i} with a payload that is mostly noise"))
        .collect::<Vec<_>>()
        .join("\n");
    let thinking_len = 300 + (turn % 11) * 150;
    let thinking = "the agent thinks about the best approach and weighs the options"
        .repeat((thinking_len / 60).max(1));
    let user = format!("user turn {turn}: do the thing and show me the results please");
    let tool_card = format!("Bash(command: ls -la | head -n {tool_lines})\n{tool_body}");

    (
        vec![
            (user, Color::Green, ChunkKind::UserInput),
            (thinking, Color::Yellow, ChunkKind::Thinking),
            (body, Color::White, ChunkKind::AssistantText),
            (
                tool_card,
                Color::White,
                ChunkKind::ToolCall {
                    tool_call_id: format!("call_{turn}"),
                    is_error: false,
                    is_diff: false,
                },
            ),
        ],
        60 + (turn % 9) * 12, // trailing assistant prose lines
    )
}

fn fill(list: &mut ScrollbackList, target_rows: usize) -> usize {
    let mut seq = 0u64;
    let mut turn = 0usize;
    let mut total = 0usize;
    while total < target_rows {
        let (items, trail) = turn_text(turn);
        for (text, color, kind) in items {
            total += push_src(list, &mut seq, &text, color, kind);
            if total >= target_rows {
                break;
            }
        }
        if total < target_rows {
            let prose = "trailing assistant prose line".repeat(trail * 3);
            total += push_src(
                list,
                &mut seq,
                &prose,
                Color::White,
                ChunkKind::AssistantText,
            );
        }
        turn += 1;
    }
    total
}

fn total_rows(list: &ScrollbackList) -> usize {
    list.chunks().iter().map(|c| c.lines.len()).sum()
}

fn bench(label: &str, rows: usize, iters: usize, f: impl Fn(&mut ScrollbackList, usize) -> u64) {
    let mut list = ScrollbackList::new();
    let filled = fill(&mut list, rows);
    let chunks = list.chunk_count();
    list.set_viewport_width(VIEWPORT_W);
    list.set_viewport_height(VIEWPORT_H);
    // prime: one render at tail
    let area = Rect {
        x: 0,
        y: 0,
        width: VIEWPORT_W,
        height: VIEWPORT_H,
    };
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);

    let t0 = Instant::now();
    for i in 0..iters {
        f(&mut list, i);
    }
    let total_ns = t0.elapsed().as_nanos() as u64;
    let avg_us = total_ns / iters as u64 / 1000;
    println!(
        "{label:<58} rows={filled:<8} chunks={chunks:<6} avg={avg_us:>7}us/frame  est_memory≈{}",
        rough_mb(chunks, filled)
    );
    let _ = buf;
}

fn rough_mb(chunks: usize, rows: usize) -> String {
    // Empirical: a wrapped Line<String> is ~80..150 B payload + Vec overhead;
    // a RenderedChunk (seq + lines Vec + source String) adds ~200..400 B.
    let per_row: usize = 160;
    let per_chunk: usize = 300;
    let bytes = rows * per_row + chunks * per_chunk;
    format!("{:.1} MB", bytes as f64 / 1e6)
}

fn render_once(list: &mut ScrollbackList, i: usize) -> u64 {
    let area = Rect {
        x: 0,
        y: 0,
        width: VIEWPORT_W,
        height: VIEWPORT_H,
    };
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    let _ = i;
    0
}

fn render_scrolling(list: &mut ScrollbackList, i: usize) -> u64 {
    // cold-cache wheel ticks: jump to a fresh offset every iteration
    let total = total_rows(list);
    let max_off = total.saturating_sub(VIEWPORT_H as usize);
    if max_off == 0 {
        return 0;
    }
    let off = (i * max_off / 300 + i * 97) % max_off;
    list.jump_to_offset(off);
    let area = Rect {
        x: 0,
        y: 0,
        width: VIEWPORT_W,
        height: VIEWPORT_H,
    };
    let mut buf = Buffer::empty(area);
    list.render_count_visited(area, &mut buf);
    0
}

fn recompute_offset(list: &mut ScrollbackList, _i: usize) -> u64 {
    list.jump_to_bottom();
    0
}

fn main() {
    println!(
        "BUG-192 scrollback simulation — viewport {VIEWPORT_W}x{VIEWPORT_H}, \
budget 16ms/frame (16000us), debug=false (release)"
    );
    println!("\n=== per-frame render, stick-to-bottom, WARM cache (steady-state busy repaint) ===");
    for rows in [1_000, 5_000, 20_000, 50_000, 100_000, 200_000] {
        bench(
            &format!("steady frame @ {rows} rows"),
            rows,
            50,
            render_once,
        );
    }

    println!("\n=== per-frame render, COLD cache + scrolling wheel ticks (new rows reflected) ===");
    for rows in [1_000, 5_000, 20_000, 50_000, 100_000, 200_000] {
        bench(
            &format!("scrolling frame @ {rows} rows"),
            rows,
            50,
            render_scrolling,
        );
    }

    println!("\n=== scroll offset recomputation (jump_to_bottom) ===");
    for rows in [1_000, 5_000, 20_000, 50_000, 100_000, 200_000] {
        bench(
            &format!("recompute @ {rows} rows"),
            rows,
            200,
            recompute_offset,
        );
    }
}
