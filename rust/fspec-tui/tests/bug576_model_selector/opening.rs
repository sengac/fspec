//! Opening /model starts from a clean list; a failed list says so; with
//! nothing selected no row is drawn selected.

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::model_selector::CustomModelMode;
use codelet_test_helpers::TestResult;
use crossterm::event::KeyCode;

use super::support::{
    app, cloud, custom, drain, drawn_selected, mock, model, open, press, profile, pump_until,
    screen, selector, type_text, view, LIST,
};

#[tokio::test(start_paused = true)]
async fn reopening_the_selector_starts_from_a_clean_list() -> TestResult {
    // @step Given I opened /model, typed the filter gpt, pressed Enter and then Esc
    let mock = mock(vec![cloud(
        "openai",
        "OpenAI",
        vec![model("gpt-6"), model("o9")],
    )]);
    let mut app = app(&mock);
    open(&mut app).await?;
    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "gpt");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Esc);

    // @step When I open /model again before its list answers
    mock.gates.gate(LIST);
    app.dispatch(Action::OpenModelSelectorView);
    pump_until(&mut app, "the list to be held", |_| {
        mock.gates.held(LIST) == 1
    })
    .await?;

    // @step Then it says "Loading models…" and shows no filter
    let text = selector(&mut app, 120).0.join("\n");
    assert!(text.contains("Loading models…"), "{text}");
    assert!(!text.contains("Filter:"), "{text}");
    mock.gates.open(LIST);
    drain(&mut app).await?;
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_failed_list_says_the_models_could_not_be_loaded() -> TestResult {
    // @step Given the provider list cannot be read
    let mock = mock(Vec::new());
    mock.fail_list_providers("providers/broken.json is not valid JSON");
    let mut app = app(&mock);

    // @step When I open /model
    open(&mut app).await?;

    // @step Then the provider list error dialog shows
    let whole = screen(&mut app);
    assert!(
        whole.contains("The provider list could not be read"),
        "{whole}"
    );

    // @step And the selector says the models could not be loaded and why, not "Loading models…"
    let text = selector(&mut app, 120).0.join("\n");
    assert!(
        text.contains("Models could not be loaded: providers/broken.json is not valid JSON"),
        "{text}"
    );
    assert!(!text.contains("Loading models…"), "{text}");
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn with_nothing_selected_the_cursor_stays_inside_the_list_and_no_row_is_drawn_selected(
) -> TestResult {
    // @step Given I left /model with the cursor on row 40 of an expanded list
    let models = (1..=45).map(|n| custom(&format!("m{n}"))).collect();
    let mock = mock(vec![
        profile("openai", "local", models),
        cloud("anthropic", "Anthropic", vec![model("a1")]),
    ]);
    let mut app = app(&mock);
    open(&mut app).await?;
    press(&mut app, KeyCode::Right);
    for _ in 0..40 {
        press(&mut app, KeyCode::Down);
    }
    assert_eq!(view(&app).selected_index(), 40);
    press(&mut app, KeyCode::Esc);

    // @step When I open /model again with no current model
    open(&mut app).await?;

    // @step Then the cursor is on a row of the list, no row is drawn selected and d does nothing
    assert!(
        view(&app).selected_index() < 2,
        "the cursor is on row {} of 2",
        view(&app).selected_index()
    );
    let (lines, buffer) = selector(&mut app, 120);
    assert!(!drawn_selected(&lines, &buffer, "openai: local"));
    assert!(!drawn_selected(&lines, &buffer, "Anthropic"));
    press(&mut app, KeyCode::Char('d'));
    assert!(matches!(
        view(&app).custom_model_mode(),
        CustomModelMode::Browse
    ));
    Ok(())
}
