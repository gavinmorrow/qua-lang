pub mod env;
mod stdlib;

pub use stdlib::stub_stdlib;

use std::iter::zip;

use env::Environment;

use crate::ast::{
    BinaryExpr, BinaryOp, Binding, Block, Call, ElseBlock, Expr, Identifier, IfExpr, Literal,
    Pattern, Program, Stmt, UnaryExpr, UnaryOp,
};

pub fn interpert(program: Program, env: &mut Environment) -> Result<Value> {
    program.evaluate(env)
}

pub type Result<T> = std::result::Result<T, Error>;

trait Evaluate {
    fn evaluate(&self, env: &mut Environment) -> Result<Value>;
}

impl Evaluate for Program {
    fn evaluate(&self, env: &mut Environment) -> Result<Value> {
        for stmt in self {
            stmt.evaluate(env)?;
        }
        Ok(Value::Nil)
    }
}

impl Evaluate for Stmt {
    fn evaluate(&self, env: &mut Environment) -> Result<Value> {
        match self {
            Stmt::Let(binding) => binding.evaluate(env),
            Stmt::Expr(expr) => expr.evaluate(env),
        }
    }
}

impl Evaluate for Binding {
    fn evaluate(&self, env: &mut Environment) -> Result<Value> {
        let identifier = self.pattern.0.clone();

        let value = match &self.arguments {
            // Don't `.evaluate()` anything for a function
            Some(arguments) => {
                let arguments = arguments.clone();
                let body = self.value.clone();
                Value::Func(Func::User(UserFunc { arguments, body }))
            }
            // But do for a variable
            None => self.value.evaluate(env)?,
        };
        env.define(identifier, value);
        Ok(Value::Nil)
    }
}

impl Evaluate for Expr {
    fn evaluate(&self, env: &mut Environment) -> Result<Value> {
        match self {
            Expr::Block(block) => block.evaluate(env),
            Expr::Call(call) => call.evaluate(env),
            Expr::If(if_expr) => if_expr.evaluate(env),
            Expr::Binary(binary_expr) => binary_expr.evaluate(env),
            Expr::Unary(unary_expr) => unary_expr.evaluate(env),
            Expr::Literal(literal) => literal.evaluate(env),
            Expr::Identifier(identifier) => identifier.evaluate(env),
        }
    }
}

impl Evaluate for Block {
    fn evaluate(&self, env: &mut Environment) -> Result<Value> {
        for stmt in &self.0 {
            stmt.evaluate(env)?;
        }

        self.1
            .as_ref()
            .map(|e| e.evaluate(env))
            .unwrap_or(Ok(Value::Nil))
    }
}

impl Evaluate for Call {
    fn evaluate(&self, env: &mut Environment) -> Result<Value> {
        let func = self.target.evaluate(env)?;
        let Value::Func(func) = func else {
            return Err(Error::new(ErrorKind::TypeError {
                expected: DiagnosticType::Func,
                actual: func.into(),
            }));
        };

        match func {
            Func::User(func) => {
                for (pattern, value) in zip(func.arguments, &self.arguments) {
                    let identifier = pattern.0.clone();

                    let value = value.evaluate(env)?;
                    env.define(identifier, value);
                }

                // Evaluate function body
                func.body.evaluate(env)
            }
            Func::Native(func) => {
                let mut arguments = vec![];
                for argument in &self.arguments {
                    let argument = argument.evaluate(env)?;
                    arguments.push(argument);
                }
                func.call(arguments)
            }
        }
    }
}

impl Evaluate for IfExpr {
    fn evaluate(&self, env: &mut Environment) -> Result<Value> {
        let condition = self.condition.evaluate(env)?;
        if condition.is_truthy() {
            self.then_block.evaluate(env)
        } else {
            match &self.else_block {
                Some(ElseBlock::Else(else_block)) => Ok(else_block.evaluate(env)?),
                Some(ElseBlock::ElseIf(if_expr)) => Ok(if_expr.evaluate(env)?),
                None => Ok(Value::Nil),
            }
        }
    }
}

impl Evaluate for BinaryExpr {
    fn evaluate(&self, env: &mut Environment) -> Result<Value> {
        use Value::{Bool, Num, Str};

        let lhs = self.lhs.evaluate(env)?;
        // A closure so that it is lazy, for short-circuiting
        let mut rhs = || self.rhs.evaluate(env);
        Ok(match &self.op {
            BinaryOp::Or => {
                if lhs.is_truthy() {
                    lhs
                } else {
                    rhs()?
                }
            }
            BinaryOp::And => {
                if lhs.is_falsy() {
                    lhs
                } else {
                    rhs()?
                }
            }
            BinaryOp::NotEq => Bool(lhs != rhs()?),
            BinaryOp::Eq => Bool(lhs == rhs()?),
            BinaryOp::Greater => Bool(lhs.as_num()? > rhs()?.as_num()?),
            BinaryOp::GreaterEq => Bool(lhs.as_num()? >= rhs()?.as_num()?),
            BinaryOp::Less => Bool(lhs.as_num()? < rhs()?.as_num()?),
            BinaryOp::LessEq => Bool(lhs.as_num()? <= rhs()?.as_num()?),
            BinaryOp::Subtract => Num(lhs.as_num()? - rhs()?.as_num()?),
            BinaryOp::Add => match (lhs, rhs()?) {
                (Num(a), Num(b)) => Num(a + b),
                (Str(a), Num(b)) => Str(a + &b.to_string()),
                (Num(a), Str(b)) => Str(a.to_string() + &b),
                (Str(a), Str(b)) => Str(a + &b),

                // Errors:
                (Num(_), b) | (Str(_), b) => {
                    return Err(Error::new(ErrorKind::TypeError {
                        expected: DiagnosticType::Num,
                        actual: b.into(),
                    }))
                }
                (a, _) => {
                    return Err(Error::new(ErrorKind::TypeError {
                        expected: DiagnosticType::Num,
                        actual: a.into(),
                    }))
                }
            },
            BinaryOp::Divide => Num(lhs.as_num()? / rhs()?.as_num()?),
            BinaryOp::Multiply => Num(lhs.as_num()? * rhs()?.as_num()?),
        })
    }
}

impl Evaluate for UnaryExpr {
    fn evaluate(&self, env: &mut Environment) -> Result<Value> {
        let rhs = self.rhs.evaluate(env)?;
        match self.op {
            UnaryOp::Not => Ok(Value::Bool(!rhs.is_truthy())),
            UnaryOp::Negate => Ok(Value::Num(-rhs.as_num()?)),
        }
    }
}

impl Evaluate for Literal {
    fn evaluate(&self, _env: &mut Environment) -> Result<Value> {
        Ok(match self {
            Literal::Bool(b) => Value::Bool(*b),
            Literal::Number(n) => Value::Num(*n),
            Literal::Str(s) => Value::Str(s.clone()),
            Literal::Nil => Value::Nil,
        })
    }
}

impl Evaluate for Identifier {
    fn evaluate(&self, env: &mut Environment) -> Result<Value> {
        Ok(env
            .get(self)
            .unwrap_or_else(|| panic!("Variable not defined!! {self:#?}"))
            .clone())
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Bool(bool),
    Num(f64),
    Str(String),
    Func(Func),
    List(Vec<Value>),
    Nil,
}

impl Value {
    fn is_truthy(&self) -> bool {
        !self.is_falsy()
    }
    fn is_falsy(&self) -> bool {
        matches!(self, Value::Bool(false) | Value::Nil)
    }

    fn as_num(&self) -> Result<f64> {
        match self {
            Self::Num(n) => Ok(*n),
            _ => Err(Error::new(ErrorKind::TypeError {
                expected: DiagnosticType::Num,
                actual: DiagnosticType::from(self),
            })),
        }
    }

    fn as_str(&self) -> Result<String> {
        match self {
            Self::Str(s) => Ok(s.clone()),
            _ => Err(Error::new(ErrorKind::TypeError {
                expected: DiagnosticType::Str,
                actual: DiagnosticType::from(self),
            })),
        }
    }

    fn as_list(&self) -> Result<Vec<Value>> {
        match self {
            Self::List(l) => Ok(l.clone()),
            _ => Err(Error::new(ErrorKind::TypeError {
                expected: DiagnosticType::List,
                actual: DiagnosticType::from(self),
            })),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => l == r,
            (Self::Num(l), Self::Num(r)) => l == r,
            (Self::Str(l), Self::Str(r)) => l == r,
            (Self::Func(_), Self::Func(_)) => false,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Func {
    User(UserFunc),
    Native(&'static dyn NativeFunc),
}

#[derive(Clone, Debug)]
pub struct UserFunc {
    arguments: Vec<Pattern>,
    body: Expr,
}

pub trait NativeFunc: std::fmt::Debug {
    fn call(&self, arguments: Vec<Value>) -> Result<Value>;
}

#[expect(dead_code, reason = "Pretty error printing not implemented yet")]
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }
}

#[expect(dead_code, reason = "Pretty error printing not implemented yet")]
#[derive(Debug)]
pub enum ErrorKind {
    TypeError {
        expected: DiagnosticType,
        actual: DiagnosticType,
    },
    IOError(std::io::Error),
    IncorrectArity {
        given: usize,
        correct: usize,
    },
}

#[derive(Debug)]
pub enum DiagnosticType {
    Bool,
    Num,
    Str,
    Func,
    List,
    Nil,
}
impl From<&Value> for DiagnosticType {
    fn from(value: &Value) -> Self {
        match value {
            Value::Bool(_) => Self::Bool,
            Value::Num(_) => Self::Num,
            Value::Str(_) => Self::Str,
            Value::Func(_) => Self::Func,
            Value::List(_) => Self::List,
            Value::Nil => Self::Nil,
        }
    }
}
impl From<Value> for DiagnosticType {
    fn from(value: Value) -> Self {
        Self::from(&value)
    }
}
