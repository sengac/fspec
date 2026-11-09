//! AK-9: the footer shows the keys that work in each mode.

use codelet_test_helpers::{OrFail, TestError, TestResult};
use crossterm::event::KeyCode;

use super::support::{app, cloud, mock, model, open, press, selector};

/// The footer row of the selector drawn at 140 columns.
fn footer(app: &mut codelet_fspec_tui::App) -> Result<String, TestError> {
    let lines = selector(app, 140).0;
    Ok(lines
        .into_iter()
        .rev()
        .find(|line| line.contains("Esc"))
        .or_fail("the footer row")?)
}

#[tokio::test(start_paused = true)]
async fn the_footer_shows_the_filter_keys_while_i_type_a_filter() -> TestResult {
    // @step Given the model selector is open with its list loaded
    let mock = mock(vec![cloud("openai", "OpenAI", vec![model("o1")])]);
    let mut app = app(&mock);
    open(&mut app).await?;

    // @step When I press /
    press(&mut app, KeyCode::Char('/'));

    // @step Then the footer says "Type to filter | Enter Apply | Esc Clear"
    let filtering = footer(&mut app)?;
    assert!(
        filtering.contains("Type to filter | Enter Apply | Esc Clear"),
        "{filtering}"
    );

    // @step And after Enter the footer lists the browse keys again
    press(&mut app, KeyCode::Enter);
    let browsing = footer(&mut app)?;
    assert!(browsing.contains("Enter Select"), "{browsing}");
    assert!(!browsing.contains("Type to filter"), "{browsing}");
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_browse_footer_lists_every_browse_key() -> TestResult {
    // @step Given the model selector is open with its list loaded
    let mock = mock(vec![cloud("openai", "OpenAI", vec![model("o1")])]);
    let mut app = app(&mock);
    open(&mut app).await?;

    // @step When the selector is drawn
    let browsing = footer(&mut app)?;

    // @step Then the footer lists "Tab Providers" and "a/e/d Custom model" next to the other browse keys
    for key in [
        "Enter Select",
        "/ Filter",
        "c Copy",
        "r Refresh",
        "Tab Providers",
        "a/e/d Custom model",
        "Esc Close",
    ] {
        assert!(browsing.contains(key), "{key} missing: {browsing}");
    }
    Ok(())
}
