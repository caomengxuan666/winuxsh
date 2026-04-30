//! Source code span for error reporting.

use std::fmt;

/// A span in the source code, represented as byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// Start byte offset (inclusive)
    pub start: usize,
    /// End byte offset (exclusive)
    pub end: usize,
}

impl Span {
    /// Create a new span.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Create a span at a single position (zero width).
    pub fn at(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    /// Create an empty span.
    pub fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    /// Get the length of this span.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if this span is empty.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Merge two spans into one that covers both.
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Extract the text for this span from the source.
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_new() {
        let span = Span::new(5, 10);
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 10);
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());
    }

    #[test]
    fn test_span_at() {
        let span = Span::at(5);
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 5);
        assert_eq!(span.len(), 0);
        assert!(span.is_empty());
    }

    #[test]
    fn test_span_merge() {
        let a = Span::new(5, 10);
        let b = Span::new(8, 15);
        let merged = a.merge(b);
        assert_eq!(merged.start, 5);
        assert_eq!(merged.end, 15);
    }

    #[test]
    fn test_span_text() {
        let source = "hello world";
        let span = Span::new(6, 11);
        assert_eq!(span.text(source), "world");
    }
}
