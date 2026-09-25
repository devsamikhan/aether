use super::span::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Int(i64),
    Float(f64),
    String(String),
    Ident(String),

    // Core Keywords (~20 clean keywords)
    Fn,
    Def,
    Lambda,
    Global,
    Let,
    Mut,
    If,
    Elif,
    Else,
    For,
    In,
    While,
    Loop,
    Break,
    Continue,
    Return,
    Yield,
    Match,
    Intent,
    Require,
    Ensure,
    Type,
    Struct,
    Class,
    Super,
    Trait,
    Impl,
    Use,
    Spawn,
    Channel,
    Defer,
    Pass,
    Case,
    Try,
    Except,
    Finally,
    Raise,
    Import,
    From,
    As,

    // Values & logic
    True,
    False,
    Nil,
    And,
    Or,
    Not,

    // Operators
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Power,      // ^
    Pipe,       // |
    PipeRight,  // |> (Pipeline Operator)
    Arrow,      // ->
    FatArrow,   // =>

    // Comparison & Assignment
    Assign,       // =
    PlusAssign,   // +=
    MinusAssign,  // -=
    StarAssign,   // *=
    SlashAssign,  // /=
    PercentAssign, // %=
    EqualEqual,   // ==
    NotEqual,     // !=
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=

    // Punctuation
    Dot,          // .
    DotDot,       // ..
    Comma,        // ,
    Colon,        // :
    DoubleColon,  // ::
    Semicolon,    // ;
    Question,     // ?

    // Delimiters
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LBrace,   // {
    RBrace,   // }

    // Indentation & Whitespace
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Int(v) => write!(f, "{}", v),
            TokenKind::Float(v) => write!(f, "{}", v),
            TokenKind::String(s) => write!(f, "\"{}\"", s),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Def => write!(f, "def"),
            TokenKind::Lambda => write!(f, "lambda"),
            TokenKind::Global => write!(f, "global"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Mut => write!(f, "mut"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Elif => write!(f, "elif"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Loop => write!(f, "loop"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Yield => write!(f, "yield"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Intent => write!(f, "intent"),
            TokenKind::Require => write!(f, "require"),
            TokenKind::Ensure => write!(f, "ensure"),
            TokenKind::Type => write!(f, "type"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Class => write!(f, "class"),
            TokenKind::Super => write!(f, "super"),
            TokenKind::Trait => write!(f, "trait"),
            TokenKind::Impl => write!(f, "impl"),
            TokenKind::Use => write!(f, "use"),
            TokenKind::Spawn => write!(f, "spawn"),
            TokenKind::Channel => write!(f, "channel"),
            TokenKind::Defer => write!(f, "defer"),
            TokenKind::Pass => write!(f, "pass"),
            TokenKind::Case => write!(f, "case"),
            TokenKind::Try => write!(f, "try"),
            TokenKind::Except => write!(f, "except"),
            TokenKind::Finally => write!(f, "finally"),
            TokenKind::Raise => write!(f, "raise"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::From => write!(f, "from"),
            TokenKind::As => write!(f, "as"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Nil => write!(f, "nil"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Power => write!(f, "^"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::PipeRight => write!(f, "|>"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::Assign => write!(f, "="),
            TokenKind::PlusAssign => write!(f, "+="),
            TokenKind::MinusAssign => write!(f, "-="),
            TokenKind::StarAssign => write!(f, "*="),
            TokenKind::SlashAssign => write!(f, "/="),
            TokenKind::PercentAssign => write!(f, "%="),
            TokenKind::EqualEqual => write!(f, "=="),
            TokenKind::NotEqual => write!(f, "!="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::Dot => write!(f, "."),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::DoubleColon => write!(f, "::"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Question => write!(f, "?"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::Newline => write!(f, "\\n"),
            TokenKind::Indent => write!(f, "INDENT"),
            TokenKind::Dedent => write!(f, "DEDENT"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

impl TokenKind {
    pub fn as_ident_str(&self) -> Option<&'static str> {
        match self {
            TokenKind::Fn => Some("fn"),
            TokenKind::Def => Some("def"),
            TokenKind::Lambda => Some("lambda"),
            TokenKind::Global => Some("global"),
            TokenKind::Let => Some("let"),
            TokenKind::Mut => Some("mut"),
            TokenKind::If => Some("if"),
            TokenKind::Elif => Some("elif"),
            TokenKind::Else => Some("else"),
            TokenKind::For => Some("for"),
            TokenKind::In => Some("in"),
            TokenKind::While => Some("while"),
            TokenKind::Loop => Some("loop"),
            TokenKind::Break => Some("break"),
            TokenKind::Continue => Some("continue"),
            TokenKind::Return => Some("return"),
            TokenKind::Yield => Some("yield"),
            TokenKind::Match => Some("match"),
            TokenKind::Intent => Some("intent"),
            TokenKind::Require => Some("require"),
            TokenKind::Ensure => Some("ensure"),
            TokenKind::Type => Some("type"),
            TokenKind::Struct => Some("struct"),
            TokenKind::Class => Some("class"),
            TokenKind::Super => Some("super"),
            TokenKind::Trait => Some("trait"),
            TokenKind::Impl => Some("impl"),
            TokenKind::Use => Some("use"),
            TokenKind::Spawn => Some("spawn"),
            TokenKind::Channel => Some("channel"),
            TokenKind::Defer => Some("defer"),
            TokenKind::Pass => Some("pass"),
            TokenKind::Case => Some("case"),
            TokenKind::Try => Some("try"),
            TokenKind::Except => Some("except"),
            TokenKind::Finally => Some("finally"),
            TokenKind::Raise => Some("raise"),
            TokenKind::Import => Some("import"),
            TokenKind::From => Some("from"),
            TokenKind::As => Some("as"),
            TokenKind::True => Some("true"),
            TokenKind::False => Some("false"),
            TokenKind::Nil => Some("nil"),
            _ => None,
        }
    }
}
