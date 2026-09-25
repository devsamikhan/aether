use super::span::Span;
use super::token::{Token, TokenKind};

pub struct Lexer<'a> {
    source: &'a str,
    chars: Vec<(usize, char)>,
    cursor: usize,
    line: usize,
    col: usize,
    indent_stack: Vec<usize>,
    paren_depth: usize,
    at_line_start: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let source = if source.starts_with('\u{feff}') {
            &source['\u{feff}'.len_utf8()..]
        } else {
            source
        };
        let chars: Vec<(usize, char)> = source.char_indices().collect();
        Self {
            source,
            chars,
            cursor: 0,
            line: 1,
            col: 1,
            indent_stack: vec![0],
            paren_depth: 0,
            at_line_start: true,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, (String, Span)> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            if self.at_line_start {
                self.handle_indentation(&mut tokens);
                if self.is_at_end() {
                    break;
                }
            }

            let start_pos = self.current_pos();
            let start_line = self.line;
            let start_col = self.col;

            let ch = self.advance();

            match ch {
                // Whitespace
                ' ' | '\t' | '\r' => {
                    // Ignore inline whitespace
                }
                '\n' => {
                    self.line += 1;
                    self.col = 1;
                    self.at_line_start = true;
                    if self.paren_depth == 0 {
                        // Avoid emitting consecutive or leading newlines
                        if let Some(last) = tokens.last() {
                            if last.kind != TokenKind::Newline && last.kind != TokenKind::Indent {
                                let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                                tokens.push(Token::new(TokenKind::Newline, span));
                            }
                        }
                    }
                }
                // Comments (# or //)
                '#' => {
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                '/' if self.peek() == '/' => {
                    self.advance();
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                '/' if self.peek() == '*' => {
                    self.advance();
                    while !self.is_at_end() {
                        if self.advance() == '*' && self.peek() == '/' {
                            self.advance();
                            break;
                        }
                    }
                }

                // Delimiters
                '(' => {
                    self.paren_depth += 1;
                    tokens.push(Token::new(TokenKind::LParen, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                }
                ')' => {
                    if self.paren_depth > 0 { self.paren_depth -= 1; }
                    tokens.push(Token::new(TokenKind::RParen, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                }
                '[' => {
                    self.paren_depth += 1;
                    tokens.push(Token::new(TokenKind::LBracket, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                }
                ']' => {
                    if self.paren_depth > 0 { self.paren_depth -= 1; }
                    tokens.push(Token::new(TokenKind::RBracket, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                }
                '{' => {
                    tokens.push(Token::new(TokenKind::LBrace, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                }
                '}' => {
                    tokens.push(Token::new(TokenKind::RBrace, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                }
                ',' => tokens.push(Token::new(TokenKind::Comma, Span::new(start_pos, self.current_pos(), start_line, start_col))),
                ';' => tokens.push(Token::new(TokenKind::Semicolon, Span::new(start_pos, self.current_pos(), start_line, start_col))),
                '?' => tokens.push(Token::new(TokenKind::Question, Span::new(start_pos, self.current_pos(), start_line, start_col))),

                // Operators & Colons
                ':' => {
                    if self.peek() == ':' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::DoubleColon, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Colon, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '.' => {
                    if self.peek() == '.' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::DotDot, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Dot, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '+' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::PlusAssign, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Plus, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '-' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::MinusAssign, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else if self.peek() == '>' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::Arrow, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Minus, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '*' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::StarAssign, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else if self.peek() == '*' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::Power, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Star, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '/' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::SlashAssign, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Slash, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '%' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::PercentAssign, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Percent, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '^' => tokens.push(Token::new(TokenKind::Power, Span::new(start_pos, self.current_pos(), start_line, start_col))),
                '=' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::EqualEqual, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else if self.peek() == '>' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::FatArrow, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Assign, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '!' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::NotEqual, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Not, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '<' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::LessEqual, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Less, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '>' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::GreaterEqual, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Greater, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '|' => {
                    if self.peek() == '>' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::PipeRight, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else if self.peek() == '|' {
                        self.advance();
                        tokens.push(Token::new(TokenKind::Or, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    } else {
                        tokens.push(Token::new(TokenKind::Pipe, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                    }
                }
                '&' if self.peek() == '&' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::And, Span::new(start_pos, self.current_pos(), start_line, start_col)));
                }

                // String Literals & Multi-line Strings
                '"' | '\'' => {
                    let quote = ch;
                    let is_triple = self.peek() == quote && self.peek_next() == Some(quote);
                    if is_triple {
                        self.advance(); // consume 2nd quote
                        self.advance(); // consume 3rd quote
                    }
                    let mut s = String::new();
                    let mut closed = false;
                    while !self.is_at_end() {
                        if is_triple {
                            if self.peek() == quote && self.peek_next() == Some(quote) {
                                if self.cursor + 2 < self.chars.len() && self.chars[self.cursor + 2].1 == quote {
                                    self.advance();
                                    self.advance();
                                    self.advance();
                                    closed = true;
                                    break;
                                }
                            }
                            let c = self.advance();
                            if c == '\n' {
                                self.line += 1;
                                self.col = 1;
                            }
                            s.push(c);
                        } else {
                            let c = self.advance();
                            if c == quote {
                                closed = true;
                                break;
                            } else if c == '\\' {
                                if !self.is_at_end() {
                                    match self.advance() {
                                        'n' => s.push('\n'),
                                        'r' => s.push('\r'),
                                        't' => s.push('\t'),
                                        '\\' => s.push('\\'),
                                        '\'' => s.push('\''),
                                        '"' => s.push('"'),
                                        other => {
                                            s.push('\\');
                                            s.push(other);
                                        }
                                    }
                                }
                            } else {
                                s.push(c);
                            }
                        }
                    }
                    if !closed {
                        let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                        return Err(("Unterminated string literal".to_string(), span));
                    }
                    let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                    tokens.push(Token::new(TokenKind::String(s), span));
                }

                // Numbers
                '0'..='9' => {
                    let mut num_str = String::new();
                    num_str.push(ch);
                    while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                        let digit = self.advance();
                        if digit != '_' {
                            num_str.push(digit);
                        }
                    }

                    if !self.is_at_end() && self.peek() == '.' {
                        // Lookahead to distinguish float from method call like 10.abs()
                        if self.peek_next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                            num_str.push(self.advance()); // consume '.'
                            while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                                let digit = self.advance();
                                if digit != '_' {
                                    num_str.push(digit);
                                }
                            }
                            let val: f64 = num_str.parse().unwrap_or(0.0);
                            let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                            tokens.push(Token::new(TokenKind::Float(val), span));
                        } else {
                            let val: i64 = num_str.parse().unwrap_or(0);
                            let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                            tokens.push(Token::new(TokenKind::Int(val), span));
                        }
                    } else {
                        let val: i64 = num_str.parse().unwrap_or(0);
                        let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                        tokens.push(Token::new(TokenKind::Int(val), span));
                    }
                }

                // Identifiers & Keywords
                c if c.is_alphabetic() || c == '_' => {
                    let mut ident = String::new();
                    ident.push(c);
                    while !self.is_at_end() && (self.peek().is_alphanumeric() || self.peek() == '_') {
                        ident.push(self.advance());
                    }

                    let kind = match ident.as_str() {
                        "fn" => TokenKind::Fn,
                        "def" => TokenKind::Def,
                        "lambda" => TokenKind::Lambda,
                        "global" => TokenKind::Global,
                        "let" => TokenKind::Let,
                        "mut" => TokenKind::Mut,
                        "if" => TokenKind::If,
                        "elif" => TokenKind::Elif,
                        "else" => TokenKind::Else,
                        "for" => TokenKind::For,
                        "in" => TokenKind::In,
                        "while" => TokenKind::While,
                        "loop" => TokenKind::Loop,
                        "break" => TokenKind::Break,
                        "continue" => TokenKind::Continue,
                        "return" => TokenKind::Return,
                        "yield" => TokenKind::Yield,
                        "match" => TokenKind::Match,
                        "intent" => TokenKind::Intent,
                        "require" => TokenKind::Require,
                        "ensure" => TokenKind::Ensure,
                        "type" => TokenKind::Type,
                        "struct" => TokenKind::Struct,
                        "class" => TokenKind::Class,
                        "super" => TokenKind::Super,
                        "trait" => TokenKind::Trait,
                        "impl" => TokenKind::Impl,
                        "use" => TokenKind::Use,
                        "spawn" => TokenKind::Spawn,
                        "channel" => TokenKind::Channel,
                        "defer" => TokenKind::Defer,
                        "pass" => TokenKind::Pass,
                        "case" => TokenKind::Case,
                        "try" => TokenKind::Try,
                        "except" => TokenKind::Except,
                        "finally" => TokenKind::Finally,
                        "raise" => TokenKind::Raise,
                        "import" => TokenKind::Import,
                        "from" => TokenKind::From,
                        "as" => TokenKind::As,
                        "true" | "True" => TokenKind::True,
                        "false" | "False" => TokenKind::False,
                        "nil" | "None" => TokenKind::Nil,
                        "and" => TokenKind::And,
                        "or" => TokenKind::Or,
                        "not" => TokenKind::Not,
                        _ => TokenKind::Ident(ident),
                    };

                    let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                    tokens.push(Token::new(kind, span));
                }

                other => {
                    let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                    return Err((format!("Unexpected character: '{}'", other), span));
                }
            }
        }

        // Emit any remaining dedents
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            let pos = self.current_pos();
            tokens.push(Token::new(TokenKind::Dedent, Span::new(pos, pos, self.line, self.col)));
        }

        let pos = self.current_pos();
        tokens.push(Token::new(TokenKind::Eof, Span::new(pos, pos, self.line, self.col)));

        Ok(tokens)
    }

    fn handle_indentation(&mut self, tokens: &mut Vec<Token>) {
        if self.paren_depth > 0 {
            self.at_line_start = false;
            return;
        }

        let mut indent_spaces = 0;
        let mut idx = self.cursor;

        while idx < self.chars.len() {
            let (_, ch) = self.chars[idx];
            if ch == ' ' {
                indent_spaces += 1;
                idx += 1;
            } else if ch == '\t' {
                indent_spaces += 4;
                idx += 1;
            } else if ch == '\n' || ch == '\r' {
                // Empty line, ignore
                self.cursor = idx;
                return;
            } else if ch == '#' || (ch == '/' && idx + 1 < self.chars.len() && self.chars[idx + 1].1 == '/') {
                // Comment line, ignore indentation
                return;
            } else {
                break;
            }
        }

        if idx >= self.chars.len() {
            self.cursor = idx;
            return;
        }

        let current_indent = *self.indent_stack.last().unwrap_or(&0);
        let pos = self.current_pos();

        if indent_spaces > current_indent {
            self.indent_stack.push(indent_spaces);
            tokens.push(Token::new(TokenKind::Indent, Span::new(pos, pos + (indent_spaces - current_indent), self.line, self.col)));
        } else if indent_spaces < current_indent {
            while self.indent_stack.len() > 1 && *self.indent_stack.last().unwrap() > indent_spaces {
                self.indent_stack.pop();
                tokens.push(Token::new(TokenKind::Dedent, Span::new(pos, pos, self.line, self.col)));
            }
        }

        // Advance cursor past the indentation spaces
        while self.cursor < idx {
            self.advance();
        }

        self.at_line_start = false;
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.chars.len()
    }

    fn peek(&self) -> char {
        if self.is_at_end() { '\0' } else { self.chars[self.cursor].1 }
    }

    fn peek_next(&self) -> Option<char> {
        if self.cursor + 1 < self.chars.len() {
            Some(self.chars[self.cursor + 1].1)
        } else {
            None
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.chars[self.cursor].1;
        self.cursor += 1;
        self.col += 1;
        ch
    }

    fn current_pos(&self) -> usize {
        if self.cursor < self.chars.len() {
            self.chars[self.cursor].0
        } else {
            self.source.len()
        }
    }
}
