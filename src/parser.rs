use crate::{
    ast::{
        BinaryExpr, BinaryOp, Binding, Block, Call, ElseBlock, Expr, Identifier, IfExpr, Pattern,
        Program, Stmt, UnaryExpr, UnaryOp,
    },
    lexer::{Pos, Token, TokenData},
    stream::Stream,
};

pub fn parse(tokens: Vec<Token>) -> Parse<Program> {
    Parser::new(tokens).parse_program()
}

pub type Parse<T> = Result<T, Error>;

struct Parser {
    tokens: Stream<Token>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens: Stream::new(tokens),
        }
    }
}

impl Parser {
    fn parse_program(&mut self) -> Parse<Program> {
        let mut stmts = vec![];
        while self.tokens.peek().is_some() {
            let stmt = self.parse_stmt()?;
            stmts.push(stmt);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Parse<Stmt> {
        if self.matches(&TokenData::Let) {
            let binding = self.parse_binding()?;
            self.expect(TokenData::Semicolon)?;
            Ok(Stmt::Let(binding))
        } else {
            let expr = self.parse_expr()?;
            self.expect(TokenData::Semicolon)?;
            Ok(Stmt::Expr(expr))
        }
    }

    fn parse_binding(&mut self) -> Parse<Binding> {
        let pattern = self.parse_pattern()?;
        self.expect(TokenData::Equals)?;
        let value = self.parse_expr()?;

        Ok(Binding { pattern, value })
    }

    fn parse_pattern(&mut self) -> Parse<Pattern> {
        let identifier = self.parse_identifier()?;
        Ok(Pattern(identifier))
    }

    fn parse_expr(&mut self) -> Parse<Expr> {
        let binary_expr = self.parse_logic_or()?;
        Ok(binary_expr)
    }

    fn parse_binary_expr(
        &mut self,
        mut parse_operand: impl FnMut(&mut Parser) -> Parse<Expr>,
        parse_operator: impl Fn(&Token) -> Option<BinaryOp>,
    ) -> Parse<Expr> {
        let mut lhs = parse_operand(self)?;

        while let Some(op) = self.tokens.next_if_map(&parse_operator) {
            let rhs = parse_operand(self)?;
            lhs = Expr::Binary(Box::new(BinaryExpr { lhs, op, rhs }));
        }

        Ok(lhs)
    }

    fn parse_logic_or(&mut self) -> Parse<Expr> {
        self.parse_binary_expr(Self::parse_logic_and, |t| match t.data {
            TokenData::Or => Some(BinaryOp::Or),
            _ => None,
        })
    }

    fn parse_logic_and(&mut self) -> Parse<Expr> {
        self.parse_binary_expr(Self::parse_equality, |t| match t.data {
            TokenData::And => Some(BinaryOp::And),
            _ => None,
        })
    }

    fn parse_equality(&mut self) -> Parse<Expr> {
        self.parse_binary_expr(Self::parse_term, |t| match t.data {
            TokenData::Less => Some(BinaryOp::Less),
            TokenData::LessEquals => Some(BinaryOp::LessEq),
            TokenData::Greater => Some(BinaryOp::Greater),
            TokenData::GreaterEquals => Some(BinaryOp::GreaterEq),
            _ => None,
        })
    }

    fn parse_term(&mut self) -> Parse<Expr> {
        self.parse_binary_expr(Self::parse_factor, |t| match t.data {
            TokenData::Minus => Some(BinaryOp::Subtract),
            TokenData::Plus => Some(BinaryOp::Add),
            _ => None,
        })
    }

    fn parse_factor(&mut self) -> Parse<Expr> {
        self.parse_binary_expr(Self::parse_unary, |t| match t.data {
            TokenData::Slash => Some(BinaryOp::Divide),
            TokenData::Star => Some(BinaryOp::Multiply),
            _ => None,
        })
    }

    fn parse_unary(&mut self) -> Parse<Expr> {
        let op = self.tokens.next_if_map(|t| match t.data {
            TokenData::Bang => Some(UnaryOp::Not),
            TokenData::Minus => Some(UnaryOp::Negate),
            _ => None,
        });

        if let Some(op) = op {
            let rhs = self.parse_unary()?;
            Ok(Expr::Unary(Box::new(UnaryExpr { op, rhs })))
        } else {
            self.parse_call()
        }
    }

    fn parse_call(&mut self) -> Parse<Expr> {
        let target = self.parse_primary()?;

        let mut arguments = vec![];
        while let Ok(arg) = self.parse_expr() {
            arguments.push(arg);
        }

        if arguments.is_empty() {
            Ok(target)
        } else {
            let target = Box::new(target);
            Ok(Expr::Call(Call { target, arguments }))
        }
    }

    fn parse_primary(&mut self) -> Parse<Expr> {
        use crate::ast::Literal::{Bool, Number, Str};
        use Expr::Literal;
        if self.matches(&TokenData::OpenBrace) {
            let block = self.parse_block()?;
            Ok(Expr::Block(block))
        } else if self.matches(&TokenData::If) {
            let if_expr = self.parse_if_expr()?;
            Ok(Expr::If(Box::new(if_expr)))
        } else {
            self.consume_map(
                |t| match &t.data {
                    TokenData::True => Some(Literal(Bool(true))),
                    TokenData::False => Some(Literal(Bool(false))),
                    TokenData::Number(n) => Some(Literal(Number(*n))),
                    TokenData::Str(s) => Some(Literal(Str(s.clone()))),
                    TokenData::Identifier(name) => {
                        Some(Expr::Identifier(Identifier { name: name.clone() }))
                    }
                    _ => None,
                },
                ErrorKind::ExpectedPrimary,
            )
        }
    }

    fn parse_block(&mut self) -> Parse<Block> {
        let mut stmts = vec![];
        while self.tokens.peek().is_some() {
            let stmt = self.parse_stmt()?;
            stmts.push(stmt);
        }

        self.expect(TokenData::CloseBrace)?;

        Ok(Block(stmts))
    }

    fn parse_if_expr(&mut self) -> Parse<IfExpr> {
        let condition = self.parse_expr()?;
        self.expect(TokenData::OpenBrace)?;
        let then_block = self.parse_block()?;

        let else_block = if self.matches(&TokenData::Else) {
            Some(if self.matches(&TokenData::If) {
                ElseBlock::ElseIf(Box::new(self.parse_if_expr()?))
            } else {
                ElseBlock::Else(self.parse_block()?)
            })
        } else {
            None
        };

        Ok(IfExpr {
            condition,
            then_block,
            else_block,
        })
    }

    fn parse_identifier(&mut self) -> Parse<Identifier> {
        let Some(next) = self.tokens.peek() else {
            return Err(Error {
                pos: None,
                kind: ErrorKind::ExpectedIdentifier,
            });
        };
        let TokenData::Identifier(name) = &next.data else {
            return Err(Error {
                pos: Some(next.pos),
                kind: ErrorKind::ExpectedIdentifier,
            });
        };
        let name = name.clone();
        Ok(Identifier { name })
    }
}

impl Parser {
    fn matches(&mut self, expected_type: &TokenData) -> bool {
        self.tokens.advance_if(|t| &t.data == expected_type)
    }

    fn consume_map<U>(&mut self, f: impl Fn(&Token) -> Option<U>, err: ErrorKind) -> Parse<U> {
        self.tokens.next_if_map(f).ok_or_else(|| Error {
            pos: self.tokens.peek().map(|t| t.pos),
            kind: err,
        })
    }

    fn expect(&mut self, expected_type: TokenData) -> Parse<()> {
        if self.matches(&expected_type) {
            Ok(())
        } else {
            Err(Error {
                pos: self.tokens.peek().map(|t| t.pos),
                kind: ErrorKind::ExpectedToken(expected_type),
            })
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pos: Option<Pos>,
    kind: ErrorKind,
}
#[allow(
    clippy::enum_variant_names,
    reason = "Not repeating the enum name, and adds important context."
)]
#[derive(Debug)]
pub enum ErrorKind {
    ExpectedToken(TokenData),
    ExpectedIdentifier,
    ExpectedPrimary,
    ExpectedUnary,
}

impl Stream<Token> {
    fn advance_if_token(&mut self, expected_type: TokenData) -> bool {
        self.advance_if(|t| t.data == expected_type)
    }
}
