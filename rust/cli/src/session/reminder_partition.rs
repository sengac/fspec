use super::system_reminders::SystemReminderType;
use rig::message::{Message, UserContent};
use std::collections::HashMap;

fn reminder_type(text: &str) -> Option<SystemReminderType> {
    let body = text
        .trim()
        .strip_prefix("<system-reminder>\n")?
        .strip_suffix("</system-reminder>")?;
    let marker = body
        .lines()
        .next()?
        .strip_prefix("<!-- type:")?
        .strip_suffix(" -->")?;
    SystemReminderType::parse(marker)
}

pub(super) fn partition(messages: &[Message]) -> (Vec<Message>, Vec<Message>) {
    let types: Vec<Vec<_>> = messages
        .iter()
        .map(|message| match message {
            Message::User { content } => content
                .iter()
                .filter_map(|block| match block {
                    UserContent::Text(text) => reminder_type(&text.text),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        })
        .collect();
    let mut latest = HashMap::new();
    for (index, kinds) in types.iter().enumerate() {
        for kind in kinds {
            latest.insert(*kind, index);
        }
    }
    let mut retained = Vec::new();
    let mut compactable = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        let mixed = matches!(message, Message::User { content } if content.len() > 1);
        let preserve = !types[index].is_empty()
            && (mixed
                || types[index]
                    .iter()
                    .any(|kind| latest.get(kind) == Some(&index)));
        if preserve {
            retained.push(message.clone());
        } else {
            compactable.push(message.clone());
        }
    }
    (retained, compactable)
}
