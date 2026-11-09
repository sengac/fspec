//! BUG-626: capping the learnings context at 8000 bytes never cuts a
//! multi-byte character, so building it never panics.
//!
//! Feature: spec/features/learnings-context-cap-on-multi-byte-text.feature
//!
//! Each Learnings graph lives in a temp directory and holds one decision. The
//! decision's title length sets where its rationale of 2-byte characters
//! starts, so the fixture chooses whether byte 8000 is inside a character and
//! checks that itself before the context is built.

use codelet_graph::database::GraphDatabase;
use codelet_graph::graph_entities::entities_to_jsonl;
use codelet_graph::learnings_context::build_learnings_context_from_db;
use codelet_test_helpers::{setup_temp_directory, TestError, TestResult};

pub mod graph_test_helpers;
use graph_test_helpers::make_decision;

const CAP: usize = 8000;
const MARKER: &str = "\n\n[Truncated — context volume limit reached]\n";
const SLUG: &str = "utf8-cut-decision";

/// The context text before the decision's rationale, for a decision titled
/// `title` and nothing else in the graph.
fn context_head(title: &str) -> String {
    format!("# Learnings Context\n\n## Active Decisions\n\n- **{title}** (`{SLUG}`, active)\n  Rationale: ")
}

/// The shortest title whose rationale starts so that byte 8000 of the
/// context is inside a 2-byte character (`inside`) or on a boundary.
fn title_for(inside: bool) -> Result<String, TestError> {
    for length in 1..4 {
        let title = "W".repeat(length);
        let into_rationale = CAP - context_head(&title).len();
        if (into_rationale % 2 == 1) == inside {
            return Ok(title);
        }
    }
    Err("no title length gives the wanted parity".into())
}

/// Builds the context of a Learnings graph that holds one decision.
async fn context_for(title: &str, rationale: &str) -> Result<String, TestError> {
    let directory = setup_temp_directory()?;
    let schema = include_str!("../schemas/learnings.pg");
    let db = GraphDatabase::init(&directory.path().join("learnings.nano"), schema).await?;
    let decision = make_decision(SLUG, title, "architecture", "active", rationale);
    db.load_jsonl(&entities_to_jsonl(&[decision])).await?;
    build_learnings_context_from_db(&db, SLUG)
        .await
        .ok_or_else(|| "the decision gave no context".into())
}

/// Scenario: A context cut inside a multi-byte character keeps only whole characters
#[tokio::test]
async fn cut_inside_a_character_keeps_whole_characters() -> TestResult {
    // @step Given a Learnings graph holding one decision whose 12000-byte rationale of 2-byte characters puts a character across byte 8000 of the context
    let title = title_for(true)?;
    let rationale = "é".repeat(6000);
    let full = format!("{}{rationale}\n\n", context_head(&title));
    assert!(
        !full.is_char_boundary(CAP),
        "byte 8000 must be inside a character"
    );

    // @step When the learnings context is built for the decision's slug
    let context = context_for(&title, &rationale).await?;

    // @step Then the context is its first 7999 bytes followed by the truncation marker
    assert_eq!(context, format!("{}{MARKER}", &full[..CAP - 1]));
    Ok(())
}

/// Scenario: A context cut on a character boundary keeps exactly 8000 bytes
#[tokio::test]
async fn cut_on_a_boundary_keeps_8000_bytes() -> TestResult {
    // @step Given a Learnings graph holding the same decision with a title one byte longer, which puts byte 8000 on a character boundary
    let title = format!("{}W", title_for(true)?);
    let rationale = "é".repeat(6000);
    let full = format!("{}{rationale}\n\n", context_head(&title));
    assert!(
        full.is_char_boundary(CAP),
        "byte 8000 must be a character boundary"
    );

    // @step When the learnings context is built for the decision's slug
    let context = context_for(&title, &rationale).await?;

    // @step Then the context is its first 8000 bytes followed by the truncation marker
    assert_eq!(context, format!("{}{MARKER}", &full[..CAP]));
    Ok(())
}

/// Scenario: A context under the cap is returned whole
#[tokio::test]
async fn context_under_the_cap_is_whole() -> TestResult {
    // @step Given a Learnings graph holding one decision with a short rationale of 2-byte characters
    let title = title_for(false)?;
    let rationale = "é".repeat(100);

    // @step When the learnings context is built for the decision's slug
    let context = context_for(&title, &rationale).await?;

    // @step Then the context is returned whole, without the truncation marker
    assert_eq!(context, format!("{}{rationale}\n\n", context_head(&title)));
    Ok(())
}
