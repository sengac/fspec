use std::io;
use std::ops::Range;

use proc_macro2::{LineColumn, Span};

use super::invalid;

pub(super) fn offset(source: &str, position: LineColumn) -> io::Result<usize> {
    if position.line == 0 {
        return Err(invalid("source position has no line"));
    }
    let mut start = 0;
    for (index, line) in source.split_inclusive('\n').enumerate() {
        if index + 1 == position.line {
            let column = line
                .char_indices()
                .map(|(offset, _)| offset)
                .chain(std::iter::once(line.len()))
                .nth(position.column)
                .ok_or_else(|| invalid("source column out of range"))?;
            return Ok(start + column);
        }
        start += line.len();
    }
    if position.column == 0 && position.line == source.lines().count() + 1 {
        return Ok(source.len());
    }
    Err(invalid("source line out of range"))
}

pub(super) fn range(source: &str, span: Span) -> io::Result<Range<usize>> {
    Ok(offset(source, span.start())?..offset(source, span.end())?)
}

pub(super) fn replace(
    mut source: String,
    mut edits: Vec<(Range<usize>, String)>,
) -> io::Result<String> {
    edits.sort_by_key(|(range, _)| range.start);
    let mut previous = source.len();
    for (range, replacement) in edits.into_iter().rev() {
        if range.end > previous || range.start > range.end {
            return Err(invalid("overlapping composition replacements"));
        }
        previous = range.start;
        source.replace_range(range, &replacement);
    }
    Ok(source)
}
