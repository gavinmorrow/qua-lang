pub type Program = Vec<Stmt>;

#[derive(Clone, Debug)]
pub enum Stmt {
    Let(Binding),
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub pattern: Pattern,
    pub arguments: Option<Vec<Pattern>>,
    pub value: Expr,
}
#[derive(Clone, Debug)]
pub struct Pattern(pub Identifier);

#[derive(Clone, Debug)]
pub enum Expr {
    Block(Block),
    Call(Call),
    If(Box<IfExpr>),
    Binary(Box<BinaryExpr>),
    Unary(Box<UnaryExpr>),
    Literal(Literal),
    Identifier(Identifier),
}

#[derive(Clone, Debug)]
pub struct Block(pub Vec<Stmt>, pub Option<Box<Expr>>);

#[derive(Clone, Debug)]
pub struct Call {
    pub target: Box<Expr>,
    pub arguments: Vec<Expr>,
}

#[derive(Clone, Debug)]
pub struct IfExpr {
    pub condition: Expr,
    pub then_block: Block,
    pub else_block: Option<ElseBlock>,
}
#[derive(Clone, Debug)]
pub enum ElseBlock {
    ElseIf(Box<IfExpr>),
    Else(Block),
}

#[derive(Clone, Debug)]
pub struct BinaryExpr {
    pub lhs: Expr,
    pub op: BinaryOp,
    pub rhs: Expr,
}
#[derive(Clone, Debug)]
pub enum BinaryOp {
    Or,
    And,

    NotEq,
    Eq,

    Greater,
    GreaterEq,
    Less,
    LessEq,

    Subtract,
    Add,

    Divide,
    Multiply,
}

#[derive(Clone, Debug)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub rhs: Expr,
}
#[derive(Clone, Debug)]
pub enum UnaryOp {
    Not,
    Negate,
}

#[derive(Clone, Debug)]
pub enum Literal {
    Bool(bool),
    Number(f64),
    Str(String),
    Nil,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub name: String,
    pub stack_index: Option<usize>,
}
impl Identifier {
    pub fn new(name: String) -> Self {
        Self {
            name,
            stack_index: None,
        }
    }

    pub fn resolve(mut self, stack_index: usize) -> Self {
        self.stack_index = Some(stack_index);
        self
    }
}
