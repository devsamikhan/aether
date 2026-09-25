use std::fmt;

/// Represents a source span in code with byte offsets, line, and column information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, col: usize) -> Self {
        Self {
            start,
            end,
            line,
            col,
        }
    }

    pub fn len(&self) -> usize {
        if self.end >= self.start {
            self.end - self.start
        } else {
            1
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Combines two spans into a single span covering both.
    pub fn merge(&self, other: Span) -> Span {
        let start = self.start.min(other.start);
        let end = self.end.max(other.end);
        let line = if self.start <= other.start { self.line } else { other.line };
        let col = if self.start <= other.start { self.col } else { other.col };
        Span { start, end, line, col }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}
