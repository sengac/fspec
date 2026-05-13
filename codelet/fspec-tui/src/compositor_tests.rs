//! Unit tests for [`crate::compositor::Compositor`] (RPC-008 rule [21]).
//!
//! Feature: spec/features/fspec-tui-compositor.feature
//!
//! 12 tests per RPC-002 doc 09 §A.7 + §D.1. Lives in a sibling module
//! (rather than #[cfg(test)] inside compositor.rs) so the production
//! file stays under the project's 300-LoC ceiling.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::redundant_clone)]

use std::sync::{Arc, Mutex};


use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::components::{Action, Callback, Component, EventResult, Priority};
use crate::compositor::Compositor;

/// Recorder used by every test below. Tracks (a) which ids called
/// handle_event in order, (b) which ids called render in order, (c)
/// which ids saw an Action via update.
#[derive(Default, Clone)]
struct Recorder {
    handle_event_log: Arc<Mutex<Vec<String>>>,
    render_log: Arc<Mutex<Vec<String>>>,
    update_log: Arc<Mutex<Vec<(String, Action)>>>,
}

impl Recorder {
    fn handle_events(&self) -> Vec<String> {
        self.handle_event_log.lock().unwrap().clone()
    }
    fn renders(&self) -> Vec<String> {
        self.render_log.lock().unwrap().clone()
    }
    fn updates(&self) -> Vec<(String, Action)> {
        self.update_log.lock().unwrap().clone()
    }
}

/// Test Component with configurable priority + return value.
struct TestComp {
    id: String,
    priority: Priority,
    active: bool,
    on_event: EventResultFactory,
    paint: char,
    rec: Recorder,
}

enum EventResultFactory {
    AlwaysIgnore,
    AlwaysConsume,
    /// Consumed with a callback that pops itself.
    ConsumeAndPop,
}

impl TestComp {
    fn new(id: &str, priority: Priority, rec: Recorder) -> Self {
        Self {
            id: id.to_string(),
            priority,
            active: true,
            on_event: EventResultFactory::AlwaysIgnore,
            paint: ' ',
            rec,
        }
    }
}

impl Component for TestComp {
    fn priority(&self) -> Priority {
        self.priority
    }
    fn is_active(&self) -> bool {
        self.active
    }
    fn id(&self) -> &str {
        &self.id
    }
    fn handle_event(&mut self, _event: &Event) -> EventResult {
        self.rec.handle_event_log.lock().unwrap().push(self.id.clone());
        match &self.on_event {
            EventResultFactory::AlwaysIgnore => EventResult::ignored(),
            EventResultFactory::AlwaysConsume => EventResult::consumed(),
            EventResultFactory::ConsumeAndPop => {
                let cb: Callback = Box::new(|c: &mut Compositor| {
                    let _ = c.pop();
                });
                EventResult::Consumed(Some(cb))
            }
        }
    }
    fn update(&mut self, action: Action) -> Option<Action> {
        self.rec
            .update_log
            .lock()
            .unwrap()
            .push((self.id.clone(), action));
        None
    }
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.rec.render_log.lock().unwrap().push(self.id.clone());
        if self.paint != ' ' {
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    buf[(x, y)].set_symbol(&self.paint.to_string());
                }
            }
        }
    }
}

fn key_event() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))
}

/// Scenario: Higher priority intercepts events first
#[test]
fn higher_priority_intercepts_events_first() {
    // @step Given a Compositor with a Background-priority HelloComponent and a Critical-priority HelpDialog pushed
    let rec = Recorder::default();
    let mut compositor = Compositor::new();
    let mut hello = TestComp::new("hello", Priority::Background, rec.clone());
    hello.on_event = EventResultFactory::AlwaysIgnore;
    let mut help = TestComp::new("help-dialog", Priority::Critical, rec.clone());
    help.on_event = EventResultFactory::AlwaysConsume;
    compositor.push(Box::new(hello));
    compositor.push(Box::new(help));

    // @step When a synthetic key event is dispatched via compositor.handle_event(&event)
    let _ = compositor.handle_event(&key_event());

    // @step Then the HelpDialog's handle_event was invoked
    // @step And the HelloComponent's handle_event was NOT invoked
    let log = rec.handle_events();
    assert_eq!(log, vec!["help-dialog".to_string()]);
}

/// Scenario: Ignored events propagate to the next handler
#[test]
fn ignored_events_propagate_to_the_next_handler() {
    // @step Given a Compositor with two layers: a Critical-priority component returning Ignored(None) and a Background-priority component recording its invocation
    let rec = Recorder::default();
    let mut compositor = Compositor::new();
    let crit = TestComp::new("crit", Priority::Critical, rec.clone());
    let mut bg = TestComp::new("bg", Priority::Background, rec.clone());
    bg.on_event = EventResultFactory::AlwaysIgnore;
    compositor.push(Box::new(crit));
    compositor.push(Box::new(bg));

    // @step When a key event is dispatched
    let result = compositor.handle_event(&key_event());

    // @step Then both layers received the event in priority order
    let log = rec.handle_events();
    assert_eq!(log, vec!["crit".to_string(), "bg".to_string()]);

    // @step And the dispatch returned Ignored(None) overall
    assert!(!result.is_consumed());
}

/// Scenario: is_active=false skips a handler without consuming
#[test]
fn is_active_false_skips_handler_without_consuming() {
    // @step Given a Compositor with a Critical-priority component whose is_active() returns false and a Background-priority component returning Consumed(None)
    let rec = Recorder::default();
    let mut compositor = Compositor::new();
    let mut inactive = TestComp::new("inactive", Priority::Critical, rec.clone());
    inactive.active = false;
    inactive.on_event = EventResultFactory::AlwaysConsume;
    let mut bg = TestComp::new("bg", Priority::Background, rec.clone());
    bg.on_event = EventResultFactory::AlwaysConsume;
    compositor.push(Box::new(inactive));
    compositor.push(Box::new(bg));

    // @step When a key event is dispatched
    let result = compositor.handle_event(&key_event());

    // @step Then the inactive Critical-priority component's handle_event was NOT invoked
    // @step And the Background-priority component's handle_event was invoked
    let log = rec.handle_events();
    assert_eq!(log, vec!["bg".to_string()]);

    // @step And the dispatch returned Consumed
    assert!(result.is_consumed());
}

/// Scenario: FIFO tiebreak at equal priority — newer registrations win
#[test]
fn fifo_tiebreak_at_equal_priority_newer_registrations_win() {
    // @step Given a Compositor with two Medium-priority components A and B pushed in that order
    let rec = Recorder::default();
    let mut compositor = Compositor::new();
    let mut a = TestComp::new("A", Priority::Medium, rec.clone());
    a.on_event = EventResultFactory::AlwaysIgnore;
    let mut b = TestComp::new("B", Priority::Medium, rec.clone());
    b.on_event = EventResultFactory::AlwaysConsume;
    compositor.push(Box::new(a));
    compositor.push(Box::new(b));

    // @step When a key event is dispatched
    let result = compositor.handle_event(&key_event());

    // @step Then B's handle_event was invoked BEFORE A's handle_event
    // @step And iteration short-circuited if B returned Consumed
    let log = rec.handle_events();
    assert_eq!(log, vec!["B".to_string()]);
    assert!(result.is_consumed());
}

/// Scenario: Callback inside Consumed runs after event handling completes
#[test]
fn callback_inside_consumed_runs_after_event_handling_completes() {
    // @step Given a Compositor with a layer that returns `Consumed(Some(Box::new(|c| { c.pop(); })))` on a key event
    let rec = Recorder::default();
    let mut compositor = Compositor::new();
    let mut layer = TestComp::new("popper", Priority::Critical, rec.clone());
    layer.on_event = EventResultFactory::ConsumeAndPop;
    compositor.push(Box::new(layer));
    assert_eq!(compositor.len(), 1);

    // @step When the App dispatches the key event and runs the returned callback against the compositor
    let result = compositor.handle_event(&key_event());
    let cb = match result {
        EventResult::Consumed(Some(cb)) => cb,
        _ => panic!("expected Consumed(Some(callback))"),
    };
    // The dispatch path itself did not mutate the compositor before
    // the callback ran:
    assert_eq!(compositor.len(), 1, "dispatch must not mutate compositor before callback runs");
    cb(&mut compositor);

    // @step Then the layer was popped off the compositor stack
    // @step And the dispatch path itself did not mutate the compositor before the callback ran
    assert_eq!(compositor.len(), 0);
}

/// Scenario: A Critical-priority modal pushed on top intercepts subsequent keystrokes
#[test]
fn critical_priority_modal_pushed_on_top_intercepts_subsequent_keystrokes() {
    // @step Given a Compositor with a single Background-priority HelloComponent
    let rec = Recorder::default();
    let mut compositor = Compositor::new();
    let mut hello = TestComp::new("hello", Priority::Background, rec.clone());
    hello.on_event = EventResultFactory::AlwaysIgnore;
    compositor.push(Box::new(hello));

    // @step When a Critical-priority HelpDialog is pushed onto the compositor
    let mut help = TestComp::new("help", Priority::Critical, rec.clone());
    help.on_event = EventResultFactory::AlwaysConsume;
    compositor.push(Box::new(help));

    // @step And a key event is dispatched
    let result = compositor.handle_event(&key_event());

    // @step Then the HelpDialog's handle_event was invoked first
    // @step And iteration short-circuited at the HelpDialog because it returned Consumed
    let log = rec.handle_events();
    assert_eq!(log, vec!["help".to_string()]);
    assert!(result.is_consumed());
}

/// Scenario: pop() removes the most recently pushed layer regardless of priority
#[test]
fn pop_removes_most_recently_pushed_layer_regardless_of_priority() {
    // @step Given a Compositor with a Background-priority HelloComponent and then a Critical-priority HelpDialog pushed in that order
    let rec = Recorder::default();
    let mut compositor = Compositor::new();
    compositor.push(Box::new(TestComp::new("hello", Priority::Background, rec.clone())));
    compositor.push(Box::new(TestComp::new("help", Priority::Critical, rec.clone())));

    // @step When compositor.pop() is invoked
    let popped = compositor.pop().expect("pop must return Some");

    // @step Then the returned Option contains the HelpDialog
    assert_eq!(popped.id(), "help");

    // @step And the compositor's remaining layer count is 1
    assert_eq!(compositor.len(), 1);
    assert!(compositor.contains("hello"));
}

/// Scenario: remove(id) removes the layer with the matching id
#[test]
fn remove_id_removes_the_layer_with_matching_id() {
    // @step Given a Compositor with two layers identified as "hello" (Background) and "help" (Critical)
    let rec = Recorder::default();
    let mut compositor = Compositor::new();
    compositor.push(Box::new(TestComp::new("hello", Priority::Background, rec.clone())));
    compositor.push(Box::new(TestComp::new("help", Priority::Critical, rec.clone())));

    // @step When compositor.remove("help") is invoked
    let removed = compositor.remove("help").expect("remove must return Some");

    // @step Then the returned Option contains the HelpDialog component
    assert_eq!(removed.id(), "help");

    // @step And only the "hello" layer remains in the compositor
    assert_eq!(compositor.layer_ids(), vec!["hello".to_string()]);
}

/// Scenario: Empty compositor returns Ignored from handle_event
#[test]
fn empty_compositor_returns_ignored_from_handle_event() {
    // @step Given a freshly constructed Compositor with zero layers
    let mut compositor = Compositor::new();
    assert!(compositor.is_empty());

    // @step When a key event is dispatched
    let result = compositor.handle_event(&key_event());

    // @step Then the dispatch returned Ignored(None)
    assert!(!result.is_consumed());
    // @step And no panic or borrow-checker error occurred
    // (Reaching this line proves no panic.)
}

/// Scenario: All-inactive compositor returns Ignored
#[test]
fn all_inactive_compositor_returns_ignored() {
    // @step Given a Compositor with three pushed layers, all of which return false from is_active()
    let rec = Recorder::default();
    let mut compositor = Compositor::new();
    for (id, p) in [
        ("a", Priority::Background),
        ("b", Priority::Medium),
        ("c", Priority::Critical),
    ] {
        let mut comp = TestComp::new(id, p, rec.clone());
        comp.active = false;
        compositor.push(Box::new(comp));
    }

    // @step When a key event is dispatched
    let result = compositor.handle_event(&key_event());

    // @step Then no layer's handle_event was invoked
    let log = rec.handle_events();
    assert!(log.is_empty(), "no inactive layer should be invoked, got {log:?}");

    // @step And the dispatch returned Ignored(None)
    assert!(!result.is_consumed());
}

/// Scenario: Render order is bottom-up so highest priority paints last
#[test]
fn render_order_is_bottom_up_so_highest_priority_paints_last() {
    // @step Given a Compositor with a Background-priority component drawing 'A' across the buffer and a Critical-priority component drawing 'B' across the buffer
    let rec = Recorder::default();
    let mut compositor = Compositor::new();
    let mut bg = TestComp::new("bg", Priority::Background, rec.clone());
    bg.paint = 'A';
    let mut crit = TestComp::new("crit", Priority::Critical, rec.clone());
    crit.paint = 'B';
    compositor.push(Box::new(bg));
    compositor.push(Box::new(crit));

    // @step When compositor.render(area, &mut buf) is invoked
    let area = Rect::new(0, 0, 4, 2);
    let mut buf = Buffer::empty(area);
    compositor.render(area, &mut buf);

    // @step Then every cell in the buffer contains 'B'
    // @step And no cell contains 'A'
    for y in 0..area.height {
        for x in 0..area.width {
            assert_eq!(buf[(x, y)].symbol(), "B", "cell ({x},{y}) must be 'B'");
        }
    }
    // The render call order should also be bg-then-crit so crit
    // overwrites bg, which is the bottom-up render contract:
    let renders = rec.renders();
    assert_eq!(renders, vec!["bg".to_string(), "crit".to_string()]);
}

/// Scenario: Action propagation in update fans out across all layers top-down
#[test]
fn action_propagation_in_update_fans_out_across_all_layers_top_down() {
    // @step Given a Compositor with three layers each recording the actions they observe in update()
    let rec = Recorder::default();
    let mut compositor = Compositor::new();
    compositor.push(Box::new(TestComp::new("first", Priority::Background, rec.clone())));
    compositor.push(Box::new(TestComp::new("second", Priority::Medium, rec.clone())));
    compositor.push(Box::new(TestComp::new("third", Priority::Critical, rec.clone())));

    // @step When compositor.update(Action::Quit) is invoked
    let follow = compositor.update(Action::Quit);

    // @step Then all three layers observed Action::Quit in registration order
    let updates = rec.updates();
    let ids: Vec<String> = updates.iter().map(|(id, _)| id.clone()).collect();
    assert_eq!(ids, vec!["first".to_string(), "second".to_string(), "third".to_string()]);
    for (_, a) in &updates {
        assert!(matches!(a, Action::Quit));
    }

    // @step And the call returned None because no layer produced a follow-up Action
    assert!(follow.is_none());
}
