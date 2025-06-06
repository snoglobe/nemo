use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<TopLevelItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TopLevelItem {
    TypeDecl(TypeDecl),
    FunctionDecl(FunctionDecl),
    StaticDecl(StaticDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub type_expr: TypeExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub where_clause: Vec<Constraint>,
    pub body: Option<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticDecl {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub type_expr: TypeExpr,
    pub where_clause: Vec<Constraint>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GenericParam {
    Type { name: String, constraint: Option<TypeExpr> },
    Static { name: String, type_expr: TypeExpr },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    pub name: String,
    pub type_expr: TypeExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Name(String, Vec<GenericArg>),
    Ptr { mutable: bool, inner: Box<TypeExpr> },
    Array { size: Option<Box<Expr>>, element: Box<TypeExpr> },
    Struct { fields: Vec<StructField>, methods: Vec<MethodDecl> },
    Enum { backing_type: Option<String>, variants: Vec<EnumVariant>, methods: Vec<MethodDecl> },
    Function { params: Vec<Param>, return_type: Box<TypeExpr> },
    Union(Vec<TypeExpr>),
    Intersection(Vec<TypeExpr>),
    Never,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GenericArg {
    Type(TypeExpr),
    Value(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub type_expr: TypeExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub where_clause: Vec<Constraint>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub data: EnumVariantData,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantData {
    Unit,
    Value(i64),
    Type(TypeExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub mutable: bool,
    pub name: String,
    pub type_expr: Option<TypeExpr>, // None for self
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    Decl { mutable: bool, name: String, type_expr: TypeExpr, value: Expr },
    Block(Block),
    If { condition: Expr, then_block: Block, else_part: Option<Box<Stmt>> },
    While { label: Option<String>, condition: Expr, body: Block },
    For { label: Option<String>, kind: ForKind, body: Block },
    Match { is_type: bool, expr: Expr, arms: Vec<MatchArm> },
    Return(Option<Expr>),
    Break(Option<String>),
    Continue(Option<String>),
    Labeled { label: String, stmt: Box<Stmt> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForKind {
    In { mutable: bool, var: String, iter: Expr },
    C { init: Option<Box<ForInit>>, condition: Option<Expr>, update: Option<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForInit {
    pub mutable: bool,
    pub name: String,
    pub type_expr: TypeExpr,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: MatchArmBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchArmBody {
    Block(Block),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Identifier(String),
    Type(TypeExpr, String),
    Literal(Literal),
    Wildcard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub expr: Option<Box<Expr>>, // Final expression for expression blocks
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Identifier(String),
    Literal(Literal),
    Binary { op: BinaryOp, left: Box<Expr>, right: Box<Expr> },
    Unary { op: UnaryOp, expr: Box<Expr> },
    Call { func: Box<Expr>, args: Vec<Expr> },
    Index { array: Box<Expr>, index: Box<Expr> },
    Field { expr: Box<Expr>, field: String },
    ArrayLiteral(Vec<Expr>),
    StructLiteral { fields: Vec<(String, Expr)> },
    FunctionLiteral { params: Vec<Param>, return_type: TypeExpr, body: Block },
    If { condition: Box<Expr>, then_expr: Box<Expr>, else_expr: Box<Expr> },
    Match { is_type: bool, expr: Box<Expr>, arms: Vec<MatchArm> },
    Sizeof(SizeofOperand),
    Cast { expr: Box<Expr>, type_expr: TypeExpr },
}

#[derive(Debug, Clone, PartialEq)]
pub enum SizeofOperand {
    Type(TypeExpr),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Nil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add, Sub, Mul, Div, Mod,
    // Bitwise
    BitAnd, BitOr, BitXor, Shl, Shr,
    // Comparison
    Eq, Ne, Lt, Le, Gt, Ge,
    // Logical
    And, Or,
    // Assignment
    Assign,
    AddAssign, SubAssign, MulAssign, DivAssign, ModAssign,
    BitAndAssign, BitOrAssign, BitXorAssign, ShlAssign, ShrAssign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus, Minus, BitNot, Not, Deref, AddrOf,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BinaryOp::*;
        match self {
            Add => write!(f, "+"),
            Sub => write!(f, "-"),
            Mul => write!(f, "*"),
            Div => write!(f, "/"),
            Mod => write!(f, "%"),
            BitAnd => write!(f, "&"),
            BitOr => write!(f, "|"),
            BitXor => write!(f, "^"),
            Shl => write!(f, "<<"),
            Shr => write!(f, ">>"),
            Eq => write!(f, "=="),
            Ne => write!(f, "!="),
            Lt => write!(f, "<"),
            Le => write!(f, "<="),
            Gt => write!(f, ">"),
            Ge => write!(f, ">="),
            And => write!(f, "&&"),
            Or => write!(f, "||"),
            Assign => write!(f, "="),
            AddAssign => write!(f, "+="),
            SubAssign => write!(f, "-="),
            MulAssign => write!(f, "*="),
            DivAssign => write!(f, "/="),
            ModAssign => write!(f, "%="),
            BitAndAssign => write!(f, "&="),
            BitOrAssign => write!(f, "|="),
            BitXorAssign => write!(f, "^="),
            ShlAssign => write!(f, "<<="),
            ShrAssign => write!(f, ">>="),
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use UnaryOp::*;
        match self {
            Plus => write!(f, "+"),
            Minus => write!(f, "-"),
            BitNot => write!(f, "~"),
            Not => write!(f, "!"),
            Deref => write!(f, "*"),
            AddrOf => write!(f, "^"),
        }
    }
}