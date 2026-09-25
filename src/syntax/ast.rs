use super::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        name: String,
        is_mut: bool,
        type_annotation: Option<String>,
        initializer: Option<Expr>,
        span: Span,
    },
    Assign {
        target: Expr,
        op: AssignOp,
        value: Expr,
        span: Span,
    },
    FnDef {
        name: String,
        params: Vec<Param>,
        return_type: Option<String>,
        body: Block,
        span: Span,
    },
    IntentDef {
        name: String,
        params: Vec<Param>,
        require: Vec<Expr>,
        ensure: Vec<Expr>,
        body: Block,
        span: Span,
    },
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    For {
        item: String,
        iter: Expr,
        body: Block,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Yield {
        value: Option<Expr>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    Pass(Span),
    Spawn {
        body: Block,
        span: Span,
    },
    StructDef {
        name: String,
        fields: Vec<String>,
        span: Span,
    },
    ClassDef {
        name: String,
        bases: Vec<String>,
        body: Vec<Statement>,
        span: Span,
    },
    Defer {
        expr: Expr,
        span: Span,
    },
    Global(Vec<String>, Span),
    TryCatch {
        try_block: Block,
        handlers: Vec<ExceptHandler>,
        finally_block: Option<Block>,
        span: Span,
    },
    Raise {
        expr: Option<Expr>,
        span: Span,
    },
    Import {
        module: String,
        alias: Option<String>,
        span: Span,
    },
    FromImport {
        module: String,
        symbols: Vec<(String, Option<String>)>,
        span: Span,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExceptHandler {
    pub exception_type: Option<String>,
    pub as_name: Option<String>,
    pub body: Block,
    pub span: Span,
}

impl Statement {
    pub fn span(&self) -> Span {
        match self {
            Statement::Let { span, .. } => *span,
            Statement::Assign { span, .. } => *span,
            Statement::FnDef { span, .. } => *span,
            Statement::IntentDef { span, .. } => *span,
            Statement::While { span, .. } => *span,
            Statement::For { span, .. } => *span,
            Statement::Return { span, .. } => *span,
            Statement::Yield { span, .. } => *span,
            Statement::Break(span) => *span,
            Statement::Continue(span) => *span,
            Statement::Pass(span) => *span,
            Statement::Spawn { span, .. } => *span,
            Statement::StructDef { span, .. } => *span,
            Statement::ClassDef { span, .. } => *span,
            Statement::Defer { span, .. } => *span,
            Statement::Global(_, span) => *span,
            Statement::TryCatch { span, .. } => *span,
            Statement::Raise { span, .. } => *span,
            Statement::Import { span, .. } => *span,
            Statement::FromImport { span, .. } => *span,
            Statement::Expr(expr) => expr.span(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub result: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_ann: Option<String>,
    pub default_val: Option<Expr>,
    pub is_vararg: bool,
    pub is_kwarg: bool,
}

impl Param {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_ann: None,
            default_val: None,
            is_vararg: false,
            is_kwarg: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal, Span),
    Ident(String, Span),
    Unary(UnaryOp, Box<Expr>, Span),
    Binary(Box<Expr>, BinaryOp, Box<Expr>, Span),
    Call(Box<Expr>, Vec<Expr>, Span),
    NamedArg(String, Box<Expr>, Span),
    StarArg(Box<Expr>, Span),
    DoubleStarArg(Box<Expr>, Span),
    Member(Box<Expr>, String, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    Array(Vec<Expr>, Span),
    Tuple(Vec<Expr>, Span),
    Set(Vec<Expr>, Span),
    Map(Vec<(Expr, Expr)>, Span),
    Pipe(Box<Expr>, Box<Expr>, Span),
    Lambda(Vec<Param>, Box<Block>, Span),
    If {
        condition: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Block>,
        span: Span,
    },
    Match {
        target: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Block(Block, Span),
    Super(Span),
    ListComp {
        element: Box<Expr>,
        item: String,
        iter: Box<Expr>,
        condition: Option<Box<Expr>>,
        span: Span,
    },
    DictComp {
        key: Box<Expr>,
        value: Box<Expr>,
        item: String,
        iter: Box<Expr>,
        condition: Option<Box<Expr>>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    Literal(Literal),
    Range(Literal, Literal),
    Ident(String),
    Wildcard,
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal(_, s) => *s,
            Expr::Ident(_, s) => *s,
            Expr::Unary(_, _, s) => *s,
            Expr::Binary(_, _, _, s) => *s,
            Expr::Call(_, _, s) => *s,
            Expr::NamedArg(_, _, s) => *s,
            Expr::StarArg(_, s) => *s,
            Expr::DoubleStarArg(_, s) => *s,
            Expr::Member(_, _, s) => *s,
            Expr::Index(_, _, s) => *s,
            Expr::Array(_, s) => *s,
            Expr::Tuple(_, s) => *s,
            Expr::Set(_, s) => *s,
            Expr::Map(_, s) => *s,
            Expr::Pipe(_, _, s) => *s,
            Expr::Lambda(_, _, s) => *s,
            Expr::If { span, .. } => *span,
            Expr::Match { span, .. } => *span,
            Expr::Block(_, s) => *s,
            Expr::Super(s) => *s,
            Expr::ListComp { span, .. } => *span,
            Expr::DictComp { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    In,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
}
