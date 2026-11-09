//! Feature: spec/features/add-hook-accepts-only-events-whose-hooks-run.feature
//!
//! BUG-590: the CLI-016 guidance contract also checks that every
//! `fspec add-hook <event>` mention in guidance and help names an event
//! add-hook accepts, so guidance never suggests a hook that never runs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::guidance_contract_tests::{mentions_in, source_mentions, violations, with_big_stack};

#[test]
fn a_guidance_mention_of_add_hook_with_a_rejected_event_fails_the_guidance_contract() {
    with_big_stack(|| {
        // @step Given a source line "fspec add-hook pre-lint check --command x.sh" at line 3 of "sample.md"
        let text = "# Hooks\n\nfspec add-hook pre-lint check --command x.sh\n";

        // @step When the guidance mentions are checked
        let bad = violations(&mentions_in("sample.md", text, 1), false);

        // @step Then the mention fails naming "sample.md:3" and "unknown hook event 'pre-lint'"
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert!(
            bad[0].starts_with(
                "sample.md:3: `fspec add-hook pre-lint check --command x.sh`: \
                 unknown hook event 'pre-lint': "
            ),
            "{bad:?}"
        );

        // @step And every add-hook mention in the real guidance sources and help configs names an accepted event
        let mut mentions: Vec<_> = source_mentions()
            .into_iter()
            .filter(|m| m.text.starts_with("fspec add-hook "))
            .collect();
        let config = codelet_fspec_core::help::configs::add_hook::CONFIG;
        for example in config.examples {
            mentions.extend(mentions_in("help config add-hook", example.command, 1));
        }
        assert!(
            mentions.len() > 8,
            "expected the add-hook mentions: {mentions:?}"
        );
        let bad = violations(&mentions, false);
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    });
}
