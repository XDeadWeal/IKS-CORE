use crate::diag::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    Fn,
    Var,
    Let,
    If,
    Else,
    While,
    Return,
    Break,
    Continue,
    True,
    False,
    Struct,
    Enum,
    Extern,
    Import,
    Nil,
    Switch,
    Case,
    Default,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    EqEq,
    Bang,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AmpAmp,
    PipePipe,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    DotDot,
    For,
    In,
    Arrow,
    Colon,
    Comma,
    Semi,
    Dot,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl TokenKind {
    pub fn same_variant(&self, other: &TokenKind) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    pub fn describe(&self) -> String {
        match self {
            TokenKind::Ident(s) => format!("identifier '{}'", s),
            TokenKind::Int(v) => format!("integer literal '{}'", v),
            TokenKind::Float(v) => format!("float literal '{}'", v),
            TokenKind::Str(_) => "string literal".to_string(),
            TokenKind::Fn => "'fn'".to_string(),
            TokenKind::Var => "'var'".to_string(),
            TokenKind::Let => "'let'".to_string(),
            TokenKind::If => "'if'".to_string(),
            TokenKind::Else => "'else'".to_string(),
            TokenKind::While => "'while'".to_string(),
            TokenKind::Return => "'return'".to_string(),
            TokenKind::Break => "'break'".to_string(),
            TokenKind::Continue => "'continue'".to_string(),
            TokenKind::True => "'true'".to_string(),
            TokenKind::False => "'false'".to_string(),
            TokenKind::Struct => "'struct'".to_string(),
            TokenKind::Enum => "'enum'".to_string(),
            TokenKind::Extern => "'extern'".to_string(),
            TokenKind::Import => "'import'".to_string(),
            TokenKind::Nil => "'nil'".to_string(),
            TokenKind::Switch => "'switch'".to_string(),
            TokenKind::Case => "'case'".to_string(),
            TokenKind::Default => "'default'".to_string(),
            TokenKind::Plus => "'+'".to_string(),
            TokenKind::Minus => "'-'".to_string(),
            TokenKind::Star => "'*'".to_string(),
            TokenKind::Slash => "'/'".to_string(),
            TokenKind::Percent => "'%'".to_string(),
            TokenKind::Assign => "'='".to_string(),
            TokenKind::EqEq => "'=='".to_string(),
            TokenKind::Bang => "'!'".to_string(),
            TokenKind::BangEq => "'!='".to_string(),
            TokenKind::Lt => "'<'".to_string(),
            TokenKind::LtEq => "'<='".to_string(),
            TokenKind::Gt => "'>'".to_string(),
            TokenKind::GtEq => "'>='".to_string(),
            TokenKind::AmpAmp => "'&&'".to_string(),
            TokenKind::PipePipe => "'||'".to_string(),
            TokenKind::Amp => "'&'".to_string(),
            TokenKind::Pipe => "'|'".to_string(),
            TokenKind::Caret => "'^'".to_string(),
            TokenKind::Tilde => "'~'".to_string(),
            TokenKind::Shl => "'<<'".to_string(),
            TokenKind::Shr => "'>>'".to_string(),
            TokenKind::PlusEq => "'+='".to_string(),
            TokenKind::MinusEq => "'-='".to_string(),
            TokenKind::StarEq => "'*='".to_string(),
            TokenKind::SlashEq => "'/='".to_string(),
            TokenKind::PercentEq => "'%='".to_string(),
            TokenKind::DotDot => "'..'".to_string(),
            TokenKind::For => "'for'".to_string(),
            TokenKind::In => "'in'".to_string(),
            TokenKind::Arrow => "'->'".to_string(),
            TokenKind::Colon => "':'".to_string(),
            TokenKind::Comma => "','".to_string(),
            TokenKind::Semi => "';'".to_string(),
            TokenKind::Dot => "'.'".to_string(),
            TokenKind::LParen => "'('".to_string(),
            TokenKind::RParen => "')'".to_string(),
            TokenKind::LBracket => "'['".to_string(),
            TokenKind::RBracket => "']'".to_string(),
            TokenKind::LBrace => "'{'".to_string(),
            TokenKind::RBrace => "'}'".to_string(),
            TokenKind::Eof => "end of file".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Int,
    Float,
    Bool,
    Str,
    Void,
    Ptr,
    Array(Box<Ty>),
    Named(String),
    HashMap,
    Tuple(Vec<Ty>),
    Vec2,
    Vec3,
    Vec4,
}

impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Int => write!(f, "int"),
            Ty::Float => write!(f, "float"),
            Ty::Bool => write!(f, "bool"),
            Ty::Str => write!(f, "string"),
            Ty::Void => write!(f, "void"),
            Ty::Ptr => write!(f, "ptr"),
            Ty::Array(e) => write!(f, "{}[]", e),
            Ty::Named(n) => write!(f, "{}", n),
            Ty::HashMap => write!(f, "map"),
            Ty::Vec2 => write!(f, "vec2"),
            Ty::Vec3 => write!(f, "vec3"),
            Ty::Vec4 => write!(f, "vec4"),
            Ty::Tuple(ts) => {
                write!(f, "(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl Ty {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float)
    }

    pub fn is_vec(&self) -> bool {
        matches!(self, Ty::Vec2 | Ty::Vec3 | Ty::Vec4)
    }

    pub fn vec_dim(&self) -> Option<usize> {
        match self {
            Ty::Vec2 => Some(2),
            Ty::Vec3 => Some(3),
            Ty::Vec4 => Some(4),
            _ => None,
        }
    }

    pub fn vec_of_dim(n: usize) -> Option<Ty> {
        match n {
            2 => Some(Ty::Vec2),
            3 => Some(Ty::Vec3),
            4 => Some(Ty::Vec4),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64, Span),
    Float(f64, Span),
    Str(String, Span),
    Bool(bool, Span),
    Nil(Span),
    Var(String, Span),
    Array(Vec<Expr>, Span),
    Tuple(Vec<Expr>, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    Field(Box<Expr>, String, Span),
    StructLit { name: String, fields: Vec<(String, Expr, Span)>, span: Span },
    Call { name: String, args: Vec<Expr>, span: Span },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr>, span: Span },
    Unary { op: UnOp, operand: Box<Expr>, span: Span },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, s)
            | Expr::Float(_, s)
            | Expr::Str(_, s)
            | Expr::Bool(_, s)
            | Expr::Nil(s)
            | Expr::Var(_, s)
            | Expr::Array(_, s)
            | Expr::Tuple(_, s)
            | Expr::Index(_, _, s)
            | Expr::Field(_, _, s)
            | Expr::StructLit { span: s, .. }
            | Expr::Call { span: s, .. }
            | Expr::Binary { span: s, .. }
            | Expr::Unary { span: s, .. } => *s,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Var {
        name: String,
        ty: Option<Ty>,
        init: Expr,
        mutable: bool,
        span: Span,
    },
    Assign {
        target: Expr,
        value: Expr,
        span: Span,
    },
    If {
        cond: Expr,
        then: Block,
        els: Option<Box<Stmt>>,
        span: Span,
    },
    While {
        cond: Expr,
        body: Block,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    Block(Block),
    Expr(Expr, Span),
    Switch {
        value: Expr,
        cases: Vec<SwitchCase>,
        default: Option<Block>,
        span: Span,
    },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Var { span, .. }
            | Stmt::Assign { span, .. }
            | Stmt::If { span, .. }
            | Stmt::While { span, .. }
            | Stmt::Return { span, .. }
            | Stmt::Expr(_, span) => *span,
            Stmt::Break(s) | Stmt::Continue(s) => *s,
            Stmt::Block(b) => b.span,
            Stmt::Switch { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Pattern {
    /// A literal or expression pattern, used by ordinary value switches.
    Value(Expr),
    /// Bare `Name`: a payload-less variant of an enum, or (in a value
    /// switch) simply a variable reference.
    Bare { name: String, span: Span },
    /// `Name(binding, ...)`: a payload variant with pattern bindings.
    Variant { name: String, bindings: Vec<String>, span: Span },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Value(e) => e.span(),
            Pattern::Bare { span, .. } | Pattern::Variant { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub patterns: Vec<Pattern>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Ty,
    pub body: Block,
    pub span: Span,
    pub is_extern: bool,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct VariantDecl {
    pub name: String,
    pub params: Vec<Ty>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<VariantDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub funcs: Vec<FnDecl>,
    pub structs: Vec<StructDecl>,
    pub enums: Vec<EnumDecl>,
    pub imports: Vec<ImportDecl>,
    pub warnings: Vec<crate::diag::Diag>,
}
