use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use unicode_width::UnicodeWidthStr;

struct Glyph {
    character: char,
    position: (u16, u16),
}

fn highlighted_runs(buffer: &Buffer) -> Vec<Vec<Glyph>> {
    let mut groups: Vec<Vec<Glyph>> = Vec::new();
    for row in buffer.area.y..buffer.area.bottom() {
        let mut column = buffer.area.x;
        while column < buffer.area.right() {
            let mut run = Vec::new();
            while column < buffer.area.right() {
                let cell = &buffer[(column, row)];
                let highlighted = cell.modifier.contains(Modifier::REVERSED)
                    || (cell.fg == Color::Black && !matches!(cell.bg, Color::Black | Color::Reset));
                if !highlighted {
                    column += 1;
                    break;
                }
                run.extend(cell.symbol().chars().map(|character| Glyph {
                    character,
                    position: (column, row),
                }));
                column = column.saturating_add(cell.symbol().width().max(1) as u16);
            }
            let Some(first) = run
                .iter()
                .position(|glyph| !glyph.character.is_whitespace())
            else {
                continue;
            };
            let Some(last) = run
                .iter()
                .rposition(|glyph| !glyph.character.is_whitespace())
            else {
                continue;
            };
            let run: Vec<_> = run.into_iter().skip(first).take(last - first + 1).collect();
            let continuation = groups.last().is_some_and(|previous| {
                previous
                    .last()
                    .is_some_and(|glyph| glyph.position.1 + 1 == row)
                    && previous[0].position.0.abs_diff(run[0].position.0) <= 2
            });
            if continuation {
                if let Some(previous) = groups.last_mut() {
                    previous.extend(run);
                }
            } else {
                groups.push(run);
            }
        }
    }
    groups
}

fn path_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '/' | '\\' | '.' | '_' | '-' | '%' | '#')
}

fn matches_label(group: &[Glyph], start: usize, label: &[char]) -> bool {
    let attachment = label.contains(&'/');
    if attachment
        && group[..start]
            .iter()
            .any(|glyph| !glyph.character.is_whitespace() && glyph.character != '▸')
    {
        return false;
    }
    if start > 0 && path_character(group[start - 1].character) {
        return false;
    }
    let mut expected = 0;
    let mut position = start;
    while expected < label.len() {
        let Some(glyph) = group.get(position) else {
            return false;
        };
        if position > start && group[position - 1].position.1 != glyph.position.1 {
            while label
                .get(expected)
                .is_some_and(|character| character.is_whitespace())
            {
                expected += 1;
            }
        }
        if label.get(expected) != Some(&glyph.character) {
            return false;
        }
        expected += 1;
        position += 1;
    }
    if attachment {
        return position == group.len();
    }
    !group
        .get(position)
        .is_some_and(|glyph| path_character(glyph.character))
}

pub fn selected_position(buffer: &Buffer, label: &str) -> Option<(u16, u16)> {
    let label: Vec<_> = label.chars().collect();
    if label.is_empty() {
        return None;
    }
    highlighted_runs(buffer).into_iter().find_map(|group| {
        group.iter().enumerate().find_map(|(start, glyph)| {
            matches_label(&group, start, &label).then_some(glyph.position)
        })
    })
}
