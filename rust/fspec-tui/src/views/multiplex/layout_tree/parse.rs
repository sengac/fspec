//! MUX-010 — the mux layout grammar.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! ```text
//! layout := item { "," item }
//! item   := ( pane | "[" layout "]" ) [ ":" weight ]
//! pane   := "board" | "agent" | "files" | "checkpoints"
//! weight := positive integer
//! ```
//!
//! Whitespace is free between tokens. Every error is one line and names
//! the 1-based column (in characters) of the layout string.

use thiserror::Error;

use super::{MuxLayout, MuxNode};
use crate::views::multiplex::MuxPaneKind;

/// A layout-string parse error (exact text, 1-based columns).
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MuxLayoutError {
    #[error("unbalanced bracket: '[' at column {0} is never closed")]
    UnclosedBracket(usize),
    #[error("unbalanced bracket: ']' at column {0} has no matching '['")]
    UnmatchedClose(usize),
    #[error("empty group at column {0}")]
    EmptyGroup(usize),
    #[error("missing pane at column {0}")]
    MissingPane(usize),
    #[error(
        "unknown pane '{name}' at column {column}; panes are board, agent, files, checkpoints"
    )]
    UnknownPane { name: String, column: usize },
    #[error("bad weight '{text}' at column {column}; weights are positive integers")]
    BadWeight { text: String, column: usize },
    #[error("unexpected '{found}' at column {column}")]
    Unexpected { found: char, column: usize },
}

/// Parse a layout string into a [`MuxLayout`].
pub fn parse_layout(text: &str) -> Result<MuxLayout, MuxLayoutError> {
    let mut parser = Parser {
        chars: text.chars().collect(),
        pos: 0,
    };
    let items = parser.list(None)?;
    // `list(None)` only returns at the end of the input with ≥ 1 item.
    MuxLayout::from_items(items).ok_or(MuxLayoutError::MissingPane(1))
}

fn is_delimiter(c: char) -> bool {
    matches!(c, ',' | '[' | ']' | ':') || c.is_whitespace()
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn column(&self) -> usize {
        self.pos + 1
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
    }

    fn word(&mut self) -> String {
        let start = self.pos;
        while self.peek().is_some_and(|c| !is_delimiter(c)) {
            self.pos += 1;
        }
        self.chars[start..self.pos].iter().collect()
    }

    /// Parse items until the end (`open == None`) or the `]` closing the
    /// `[` at column `open`.
    fn list(&mut self, open: Option<usize>) -> Result<Vec<MuxNode>, MuxLayoutError> {
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None => {
                    return Err(match open {
                        Some(col) => MuxLayoutError::UnclosedBracket(col),
                        None => MuxLayoutError::MissingPane(self.column()),
                    })
                }
                Some(']') => {
                    return Err(match open {
                        Some(col) if items.is_empty() => MuxLayoutError::EmptyGroup(col),
                        Some(_) => MuxLayoutError::MissingPane(self.column()),
                        None => MuxLayoutError::UnmatchedClose(self.column()),
                    })
                }
                Some(',') | Some(':') => return Err(MuxLayoutError::MissingPane(self.column())),
                Some(_) => {}
            }
            let mut node = self.item()?;
            self.skip_ws();
            if self.peek() == Some(':') {
                self.pos += 1;
                self.skip_ws();
                node.set_weight(self.weight()?);
                self.skip_ws();
            }
            items.push(node);
            match self.peek() {
                None => {
                    return match open {
                        Some(col) => Err(MuxLayoutError::UnclosedBracket(col)),
                        None => Ok(items),
                    }
                }
                Some(',') => self.pos += 1,
                Some(']') => {
                    return match open {
                        Some(_) => {
                            self.pos += 1;
                            Ok(items)
                        }
                        None => Err(MuxLayoutError::UnmatchedClose(self.column())),
                    }
                }
                Some(found) => {
                    return Err(MuxLayoutError::Unexpected {
                        found,
                        column: self.column(),
                    })
                }
            }
        }
    }

    fn item(&mut self) -> Result<MuxNode, MuxLayoutError> {
        let column = self.column();
        if self.peek() == Some('[') {
            self.pos += 1;
            let children = self.list(Some(column))?;
            return Ok(MuxNode::Group {
                children,
                weight: 1,
            });
        }
        let name = self.word();
        let kind = match name.as_str() {
            "board" => MuxPaneKind::Board,
            "agent" => MuxPaneKind::Agent,
            "files" => MuxPaneKind::ChangedFiles,
            "checkpoints" => MuxPaneKind::Checkpoints,
            _ => return Err(MuxLayoutError::UnknownPane { name, column }),
        };
        Ok(MuxNode::pane(kind))
    }

    fn weight(&mut self) -> Result<u32, MuxLayoutError> {
        let column = self.column();
        let text = self.word();
        match text.parse::<u32>() {
            Ok(value) if value > 0 && text.chars().all(|c| c.is_ascii_digit()) => Ok(value),
            _ => Err(MuxLayoutError::BadWeight { text, column }),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn err(text: &str) -> String {
        parse_layout(text).unwrap_err().to_string()
    }

    #[test]
    fn errors_name_the_column() {
        assert_eq!(
            err("agent, [board, board"),
            "unbalanced bracket: '[' at column 8 is never closed"
        );
        assert_eq!(
            err("board, agent]"),
            "unbalanced bracket: ']' at column 13 has no matching '['"
        );
        assert_eq!(err("agent, [], files"), "empty group at column 8");
        assert_eq!(
            err("agent, bord"),
            "unknown pane 'bord' at column 8; panes are board, agent, files, checkpoints"
        );
        assert_eq!(
            err("agent:0, board"),
            "bad weight '0' at column 7; weights are positive integers"
        );
        assert_eq!(
            err("agent:x"),
            "bad weight 'x' at column 7; weights are positive integers"
        );
        assert_eq!(err("agent,, board"), "missing pane at column 7");
        assert_eq!(err("agent, "), "missing pane at column 8");
        assert_eq!(err(""), "missing pane at column 1");
        assert_eq!(err("[board, ]"), "missing pane at column 9");
        assert_eq!(err("board agent"), "unexpected 'a' at column 7");
    }

    #[test]
    fn nesting_and_weights_parse() {
        let layout = parse_layout("[agent, [board, files]:2]:3").unwrap();
        let [MuxNode::Group { children, weight }] = layout.items() else {
            panic!("one top-level group");
        };
        assert_eq!(*weight, 3);
        assert_eq!(children.len(), 2);
        assert_eq!(children[1].weight(), 2);
        assert_eq!(children[1].leaf_count(), 2);
    }
}
