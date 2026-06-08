use std::ops::Range;

use crop::Rope;
use proc_macro2::{LineColumn, Span, extra::DelimSpan};
use rustc_lexer::{TokenKind, tokenize};
use syn::spanned::Spanned as _;

#[derive(Clone, Debug)]
pub(crate) struct Comment {
    pub raw: String,
}

#[derive(Clone, Debug)]
pub(crate) struct LocatedComment {
    pub start: usize,
    pub end: usize,
    pub comment: Comment,
}

#[derive(Clone, Debug)]
struct CommentEntry {
    start: usize,
    end: usize,
    comment: Comment,
    consumed: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Trivia {
    comments: Vec<CommentEntry>,
}

impl Trivia {
    pub fn new(source: &Rope, range: Range<usize>) -> Self {
        let text = source.byte_slice(range.clone()).to_string();
        let mut comments = Vec::new();
        let mut offset = range.start;

        for token in tokenize(&text) {
            let len = token.len;
            let start = offset;
            let end = offset + len;

            if matches!(
                token.kind,
                TokenKind::LineComment | TokenKind::BlockComment { .. }
            ) {
                comments.push(CommentEntry {
                    start,
                    end,
                    comment: Comment {
                        raw: text[(start - range.start)..(end - range.start)].to_string(),
                    },
                    consumed: false,
                });
            }

            offset = end;
        }

        Self { comments }
    }

    pub fn line_column_to_byte(source: &Rope, point: LineColumn) -> Option<usize> {
        if point.line == 0 || point.line > source.line_len() + 1 {
            return None;
        }
        if point.line == source.line_len() + 1 {
            return (point.column == 0).then(|| source.byte_len());
        }

        let line_idx = point.line - 1;
        let line_byte = source.byte_of_line(line_idx);
        let line = source.line(line_idx);
        let char_byte: usize = line.chars().take(point.column).map(|c| c.len_utf8()).sum();
        Some(line_byte + char_byte)
    }

    pub fn span_range(source: &Rope, span: Span) -> Option<Range<usize>> {
        let start = Self::line_column_to_byte(source, span.start())?;
        let end = Self::line_column_to_byte(source, span.end())?;
        Some(start..end)
    }

    pub fn delim_range(source: &Rope, span: DelimSpan) -> Option<Range<usize>> {
        Self::span_range(source, span.span())
    }

    pub fn delim_inner_range(source: &Rope, span: DelimSpan) -> Option<Range<usize>> {
        let start = Self::line_column_to_byte(source, span.open().end())?;
        let end = Self::line_column_to_byte(source, span.close().start())?;
        Some(start..end)
    }

    pub fn has_comments_in_range(&self, range: Range<usize>) -> bool {
        self.comments
            .iter()
            .any(|comment| range.start <= comment.start && comment.end <= range.end)
    }

    pub fn has_comments_in_span(&self, source: &Rope, span: Span) -> bool {
        Self::span_range(source, span)
            .map(|range| self.has_comments_in_range(range))
            .unwrap_or(false)
    }

    pub fn has_comments_in_delim(&self, source: &Rope, span: DelimSpan) -> bool {
        Self::delim_range(source, span)
            .map(|range| self.has_comments_in_range(range))
            .unwrap_or(false)
    }

    pub fn consume_comments_in_span(&mut self, source: &Rope, span: Span) {
        if let Some(range) = Self::span_range(source, span) {
            self.consume_comments_in_range(range);
        }
    }

    pub fn consume_comments_in_range(&mut self, range: Range<usize>) {
        for comment in &mut self.comments {
            if range.start <= comment.start && comment.end <= range.end {
                comment.consumed = true;
            }
        }
    }

    pub fn has_blank_line_in_range(&self, source: &Rope, range: Range<usize>) -> bool {
        let mut range = range;
        range.start = range.start.min(source.byte_len());
        range.end = range.end.min(source.byte_len());

        if range.start >= range.end {
            return false;
        }

        let comment_ranges = self
            .comments
            .iter()
            .filter(|comment| comment.start < range.end && range.start < comment.end)
            .map(|comment| comment.start.max(range.start)..comment.end.min(range.end))
            .collect::<Vec<_>>();

        let mut cursor = range.start;
        let mut after_newline_only_whitespace = false;

        for comment_range in comment_ranges {
            if cursor < comment_range.start
                && range_has_blank_line(
                    source,
                    cursor..comment_range.start,
                    &mut after_newline_only_whitespace,
                )
            {
                return true;
            }

            after_newline_only_whitespace = false;
            cursor = cursor.max(comment_range.end);
        }

        cursor < range.end
            && range_has_blank_line(
                source,
                cursor..range.end,
                &mut after_newline_only_whitespace,
            )
    }

    pub fn take_leading_comments(&mut self, source: &Rope, loc: LineColumn) -> Vec<Comment> {
        self.take_leading_located_comments(source, loc)
            .into_iter()
            .map(|comment| comment.comment)
            .collect()
    }

    pub fn take_leading_located_comments(
        &mut self,
        source: &Rope,
        loc: LineColumn,
    ) -> Vec<LocatedComment> {
        let Some(loc_byte) = Self::line_column_to_byte(source, loc) else {
            return Vec::new();
        };

        let mut cursor = loc_byte;
        let mut indices = Vec::new();

        while let Some((idx, comment)) =
            self.comments.iter().enumerate().rev().find(|(_, comment)| {
                !comment.consumed
                    && comment.end <= cursor
                    && is_whitespace_between(source, comment.end, cursor)
            })
        {
            indices.push(idx);
            cursor = comment.start;
        }

        indices.reverse();

        indices
            .into_iter()
            .map(|idx| {
                self.comments[idx].consumed = true;
                LocatedComment {
                    start: self.comments[idx].start,
                    end: self.comments[idx].end,
                    comment: self.comments[idx].comment.clone(),
                }
            })
            .collect()
    }

    pub fn take_trailing_comment(&mut self, source: &Rope, loc: LineColumn) -> Option<Comment> {
        let loc_byte = Self::line_column_to_byte(source, loc)?;
        let loc_line = source.line_of_byte(loc_byte);

        let (idx, _) = self
            .comments
            .iter()
            .enumerate()
            .filter(|(_, comment)| !comment.consumed && comment.start >= loc_byte)
            .filter(|(_, comment)| source.line_of_byte(comment.start) == loc_line)
            .filter(|(_, comment)| is_whitespace_between(source, loc_byte, comment.start))
            .min_by_key(|(_, comment)| comment.start)?;

        self.comments[idx].consumed = true;
        Some(self.comments[idx].comment.clone())
    }

    pub fn take_comments_in_delim(&mut self, source: &Rope, span: DelimSpan) -> Vec<Comment> {
        self.take_located_comments_in_delim(source, span)
            .into_iter()
            .map(|comment| comment.comment)
            .collect()
    }

    pub fn take_located_comments_in_delim(
        &mut self,
        source: &Rope,
        span: DelimSpan,
    ) -> Vec<LocatedComment> {
        let Some(range) = Self::delim_inner_range(source, span) else {
            return Vec::new();
        };

        self.take_located_comments_in_range(range)
    }

    pub fn take_comments_in_range(&mut self, range: Range<usize>) -> Vec<Comment> {
        self.take_located_comments_in_range(range)
            .into_iter()
            .map(|comment| comment.comment)
            .collect()
    }

    pub fn take_located_comments_in_range(&mut self, range: Range<usize>) -> Vec<LocatedComment> {
        let mut comments = Vec::new();
        for comment in &mut self.comments {
            if !comment.consumed && range.start <= comment.start && comment.end <= range.end {
                comment.consumed = true;
                comments.push(LocatedComment {
                    start: comment.start,
                    end: comment.end,
                    comment: comment.comment.clone(),
                });
            }
        }
        comments
    }
}

fn range_has_blank_line(
    source: &Rope,
    range: Range<usize>,
    after_newline_only_whitespace: &mut bool,
) -> bool {
    for ch in source.byte_slice(range).chars() {
        if ch == '\n' {
            if *after_newline_only_whitespace {
                return true;
            }
            *after_newline_only_whitespace = true;
        } else if !ch.is_whitespace() {
            *after_newline_only_whitespace = false;
        }
    }

    false
}

fn is_whitespace_between(source: &Rope, start: usize, end: usize) -> bool {
    start <= end
        && source
            .byte_slice(start..end)
            .chars()
            .all(char::is_whitespace)
}

#[cfg(test)]
mod test {
    use crop::Rope;

    use super::Trivia;

    #[test]
    fn long_same_line_gap_is_scanned_without_blank_line() {
        let source_text = format!("a{}b", " ".repeat(100_000));
        let source = Rope::from(source_text.as_str());
        let trivia = Trivia::new(&source, 0..source.byte_len());

        assert!(!trivia.has_blank_line_in_range(&source, 1..(source.byte_len() - 1)));
    }
}
