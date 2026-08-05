use std::fmt;

pub type Identifier = String;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub package: Option<PackageDecl>,
    pub imports: Vec<ImportStmt>,
    pub items: Vec<TopLevel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageDecl {
    pub path: Vec<Identifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportStmt {
    pub path: String,
    pub alias: Option<Identifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TopLevel {
    Function(FunctionDecl),
    Struct(StructDecl),
    DataClass(DataClassDecl),
    Class(ClassDecl),
    Enum(EnumDecl),
    Trait(TraitDecl),
    Impl(ImplDecl),
    Stmt(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub is_async: bool,
    pub name: Identifier,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: Identifier,
    pub ty: Type,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub name: Identifier,
    pub bound: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: Identifier,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: Identifier,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataClassDecl {
    pub name: Identifier,
    pub generics: Vec<GenericParam>,
    pub params: Vec<ConstructorParam>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorParam {
    pub is_val: bool,
    pub name: Identifier,
    pub ty: Type,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: Identifier,
    pub generics: Vec<GenericParam>,
    pub supers: Vec<Type>,
    pub members: Vec<ClassMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Field(FieldDecl),
    Function(FunctionDecl),
    Constructor(ConstructorDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub is_val: bool,
    pub name: Identifier,
    pub ty: Type,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorDecl {
    pub params: Vec<Param>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: Identifier,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: Identifier,
    pub types: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitDecl {
    pub name: Identifier,
    pub generics: Vec<GenericParam>,
    pub members: Vec<TraitMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TraitMember {
    Function(FunctionDecl),
    Signature(FunctionSignature),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    pub name: Identifier,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplDecl {
    pub trait_ty: Option<Type>,
    pub ty: Type,
    pub functions: Vec<FunctionDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VarDecl(VarDecl),
    Assign(Assign),
    Expr(Expr),
    If(IfStmt),
    Match(MatchStmt),
    While(WhileStmt),
    DoWhile(DoWhileStmt),
    Until(UntilStmt),
    For(ForStmt),
    ForIn(ForInStmt),
    Return(Option<Expr>),
    Break,
    Continue,
    Throw(Expr),
    Try(TryStmt),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub is_val: bool,
    pub name: Identifier,
    pub ty: Option<Type>,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    pub target: Expr,
    pub op: AssignOp,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub cond: Box<Expr>,
    pub then_branch: Block,
    pub else_branch: Option<Box<ElseBranch>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch {
    If(IfStmt),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchStmt {
    pub expr: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard,
    Literal(Literal),
    Binding(Identifier),
    Constructor(Identifier, Vec<Pattern>),
    At(Identifier, Box<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub cond: Expr,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoWhileStmt {
    pub body: Block,
    pub cond: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UntilStmt {
    pub cond: Expr,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub init: Option<Box<Stmt>>,
    pub cond: Option<Expr>,
    pub step: Option<Expr>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForInStmt {
    pub name: Identifier,
    pub expr: Expr,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TryStmt {
    pub body: Block,
    pub catches: Vec<CatchClause>,
    pub finally: Option<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    pub name: Option<Identifier>,
    pub ty: Type,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Await(Box<Expr>),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    FieldAccess(FieldAccessExpr),
    Index(IndexExpr),
    Identifier(Identifier),
    Literal(Literal),
    Grouping(Box<Expr>),
    Block(Block),
    If(Box<IfStmt>),
    Match(Box<MatchStmt>),
    Lambda(LambdaExpr),
    StructLiteral(StructLiteralExpr),
    DataClassLiteral(DataClassLiteralExpr),
    ArrayLiteral(Vec<Expr>),
    Spawn(Box<Expr>),
    Receive(Option<Box<Expr>>),
    Nullable(Box<Expr>),
    NonNull(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub op: BinaryOp,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    NotEq,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    Range,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
    Plus,
    Ref,
    Deref,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldAccessExpr {
    pub object: Box<Expr>,
    pub field: Identifier,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexExpr {
    pub object: Box<Expr>,
    pub index: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaExpr {
    pub params: Vec<LambdaParam>,
    pub return_type: Option<Type>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaParam {
    pub name: Identifier,
    pub ty: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteralExpr {
    pub name: Identifier,
    pub generic_args: Vec<Type>,
    pub fields: Vec<FieldInit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataClassLiteralExpr {
    pub name: Identifier,
    pub generic_args: Vec<Type>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldInit {
    pub name: Identifier,
    pub value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Named(Identifier, Vec<Type>),
    Tuple(Vec<Type>),
    Array(Box<Type>),
    Func(Vec<Type>, Option<Box<Type>>),
    Result(Box<Type>, Box<Type>),
    Option(Box<Type>),
    Union(Vec<Type>),
    Nullable(Box<Type>),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Named(name, args) => {
                write!(f, "{}", name)?;
                if !args.is_empty() {
                    write!(f, "<{}", args[0])?;
                    for arg in &args[1..] {
                        write!(f, ", {}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            Type::Array(ty) => write!(f, "[{}]", ty),
            Type::Func(params, ret) => {
                write!(f, "func(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ")")?;
                if let Some(r) = ret {
                    write!(f, ": {}", r)?;
                }
                Ok(())
            }
            Type::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            Type::Option(ty) => write!(f, "Option<{}>", ty),
            Type::Union(types) => {
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                Ok(())
            }
            Type::Nullable(ty) => write!(f, "{}?", ty),
        }
    }
}
