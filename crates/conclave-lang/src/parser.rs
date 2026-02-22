#![allow(dead_code, unused_variables)]
use crate::ast::*;
use crate::error::LangError;
use crate::lexer::{tokenize, Token};

// ---------------------------------------------------------------------------
// Token stream
// ---------------------------------------------------------------------------

struct Tokens {
    items: Vec<(Token, usize)>, // (token, line)
    pos: usize,
}

impl Tokens {
    fn new(spanned: Vec<crate::lexer::Spanned>) -> Self {
        let items = spanned.into_iter().map(|s| (s.token, s.line)).collect();
        Tokens { items, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.items.get(self.pos).map(|(t, _)| t)
    }

    fn peek_line(&self) -> usize {
        self.items.get(self.pos).map(|(_, l)| *l).unwrap_or(0)
    }

    fn advance(&mut self) -> Option<(Token, usize)> {
        if self.pos < self.items.len() {
            let item = self.items[self.pos].clone();
            self.pos += 1;
            Some(item)
        } else {
            None
        }
    }

    fn expect(&mut self, expected_desc: &str) -> Result<(Token, usize), LangError> {
        self.advance().ok_or_else(|| LangError::UnexpectedEof {
            expected: expected_desc.to_string(),
        })
    }

    fn expect_token(&mut self, expected: &Token, desc: &str) -> Result<usize, LangError> {
        match self.advance() {
            Some((tok, line)) if token_eq(&tok, expected) => Ok(line),
            Some((tok, line)) => Err(LangError::UnexpectedToken {
                expected: desc.to_string(),
                got: token_display(&tok),
                line,
            }),
            None => Err(LangError::UnexpectedEof {
                expected: desc.to_string(),
            }),
        }
    }

    fn expect_ident(&mut self) -> Result<String, LangError> {
        match self.advance() {
            Some((Token::Ident(s), _)) => Ok(s),
            Some((tok, line)) => Err(LangError::UnexpectedToken {
                expected: "identifier".to_string(),
                got: token_display(&tok),
                line,
            }),
            None => Err(LangError::UnexpectedEof {
                expected: "identifier".to_string(),
            }),
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.items.len()
    }
}

fn token_eq(a: &Token, b: &Token) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

fn token_display(tok: &Token) -> String {
    match tok {
        Token::Version => "version".into(),
        Token::Type => "type".into(),
        Token::Capability => "capability".into(),
        Token::Intrinsic => "intrinsic".into(),
        Token::Goal => "goal".into(),
        Token::Want => "want".into(),
        Token::Map => "map".into(),
        Token::As => "as".into(),
        Token::Let => "let".into(),
        Token::Emit => "emit".into(),
        Token::Return => "return".into(),
        Token::Constraints => "constraints".into(),
        Token::Where => "where".into(),
        Token::LBrace => "{".into(),
        Token::RBrace => "}".into(),
        Token::LParen => "(".into(),
        Token::RParen => ")".into(),
        Token::LAngle => "<".into(),
        Token::RAngle => ">".into(),
        Token::Semicolon => ";".into(),
        Token::Comma => ",".into(),
        Token::Colon => ":".into(),
        Token::Equals => "=".into(),
        Token::Arrow => "->".into(),
        Token::LtEq => "<=".into(),
        Token::Dot => ".".into(),
        Token::Ident(s) => s.clone(),
        Token::StringLit(s) => format!("\"{}\"", s),
        Token::Number(n) => n.to_string(),
        Token::ReqPerSec => "req/s".into(),
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse a Conclave v0.1 source string into an AST `Module`.
///
/// This does NOT normalize — call `normalize::normalize(module)` afterwards.
pub fn parse(source: &str) -> Result<Module, LangError> {
    // Normalize line endings before lexing.
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let spanned = tokenize(&normalized)?;
    let mut ts = Tokens::new(spanned);
    parse_module(&mut ts)
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

fn parse_module(ts: &mut Tokens) -> Result<Module, LangError> {
    // `version <number> ;`
    let line = ts.peek_line();
    match ts.advance() {
        Some((Token::Version, _)) => {}
        Some((tok, l)) => {
            return Err(LangError::UnexpectedToken {
                expected: "version".into(),
                got: token_display(&tok),
                line: l,
            })
        }
        None => {
            return Err(LangError::UnexpectedEof {
                expected: "version".into(),
            })
        }
    }
    // Accept both integer and float-formatted version numbers.
    let version = parse_version_number(ts, line)?;
    ts.expect_token(&Token::Semicolon, "';' after version")?;

    let mut types: Vec<TypeDecl> = Vec::new();
    let mut capabilities: Vec<CapDecl> = Vec::new();
    let mut intrinsics: Vec<IntrinsicDecl> = Vec::new();
    let mut goals: Vec<GoalDecl> = Vec::new();

    while !ts.at_end() {
        match ts.peek() {
            Some(Token::Type) => types.push(parse_type_decl(ts)?),
            Some(Token::Capability) => capabilities.push(parse_cap_decl(ts)?),
            Some(Token::Intrinsic) => intrinsics.push(parse_intrinsic_decl(ts)?),
            Some(Token::Goal) => goals.push(parse_goal_decl(ts)?),
            Some(_) => {
                let (tok, l) = ts.advance().unwrap();
                return Err(LangError::UnexpectedToken {
                    expected: "type, capability, intrinsic, or goal declaration".into(),
                    got: token_display(&tok),
                    line: l,
                });
            }
            None => break,
        }
    }

    Ok(Module {
        version,
        types,
        capabilities,
        intrinsics,
        goals,
    })
}

/// Parse a version number token. Accepts `0` (Number) or looks for `0.1`
/// as Number Dot Number sequence.
fn parse_version_number(ts: &mut Tokens, _hint_line: usize) -> Result<String, LangError> {
    match ts.advance() {
        Some((Token::Number(major), _)) => {
            // Check for .minor
            if matches!(ts.peek(), Some(Token::Dot)) {
                ts.advance(); // consume dot
                match ts.advance() {
                    Some((Token::Number(minor), _)) => Ok(format!("{}.{}", major, minor)),
                    Some((tok, line)) => Err(LangError::UnexpectedToken {
                        expected: "minor version number".into(),
                        got: token_display(&tok),
                        line,
                    }),
                    None => Err(LangError::UnexpectedEof {
                        expected: "minor version number".into(),
                    }),
                }
            } else {
                Ok(major.to_string())
            }
        }
        Some((tok, line)) => Err(LangError::UnexpectedToken {
            expected: "version number".into(),
            got: token_display(&tok),
            line,
        }),
        None => Err(LangError::UnexpectedEof {
            expected: "version number".into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Type declaration
// ---------------------------------------------------------------------------

/// `type IDENT = TYPE_EXPR ( where IDENT ( STRING ) )? ;`
fn parse_type_decl(ts: &mut Tokens) -> Result<TypeDecl, LangError> {
    ts.expect_token(&Token::Type, "type")?;
    let name = ts.expect_ident()?;
    ts.expect_token(&Token::Equals, "'='")?;
    let base = parse_type_expr(ts)?;

    let constraint = if matches!(ts.peek(), Some(Token::Where)) {
        ts.advance();
        let validator = ts.expect_ident()?;
        ts.expect_token(&Token::LParen, "'('")?;
        let pattern = match ts.advance() {
            Some((Token::StringLit(s), _)) => s,
            Some((tok, line)) => {
                return Err(LangError::UnexpectedToken {
                    expected: "string pattern".into(),
                    got: token_display(&tok),
                    line,
                })
            }
            None => {
                return Err(LangError::UnexpectedEof {
                    expected: "string pattern".into(),
                })
            }
        };
        ts.expect_token(&Token::RParen, "')'")?;
        Some(TypeConstraint { validator, pattern })
    } else {
        None
    };

    ts.expect_token(&Token::Semicolon, "';'")?;
    Ok(TypeDecl {
        name,
        base,
        constraint,
    })
}

/// A type expression: `IDENT` or `IDENT < IDENT >`.
fn parse_type_expr(ts: &mut Tokens) -> Result<String, LangError> {
    let name = ts.expect_ident()?;
    if matches!(ts.peek(), Some(Token::LAngle)) {
        ts.advance();
        let inner = parse_type_expr(ts)?;
        ts.expect_token(&Token::RAngle, "'>'")?;
        Ok(format!("{}<{}>", name, inner))
    } else {
        Ok(name)
    }
}

// ---------------------------------------------------------------------------
// Capability declaration
// ---------------------------------------------------------------------------

/// `capability IDENT : SIGNATURE ;`
fn parse_cap_decl(ts: &mut Tokens) -> Result<CapDecl, LangError> {
    ts.expect_token(&Token::Capability, "capability")?;
    let alias = ts.expect_ident()?;
    ts.expect_token(&Token::Colon, "':'")?;
    let signature = parse_signature(ts)?;
    ts.expect_token(&Token::Semicolon, "';'")?;
    Ok(CapDecl { alias, signature })
}

/// `intrinsic IDENT : SIGNATURE ;`
fn parse_intrinsic_decl(ts: &mut Tokens) -> Result<IntrinsicDecl, LangError> {
    ts.expect_token(&Token::Intrinsic, "intrinsic")?;
    let alias = ts.expect_ident()?;
    ts.expect_token(&Token::Colon, "':'")?;
    let signature = parse_signature(ts)?;
    ts.expect_token(&Token::Semicolon, "';'")?;
    Ok(IntrinsicDecl { alias, signature })
}

/// A signature: `NAME ( TYPE_LIST ) -> TYPE`
/// Returns the raw signature string (normalization happens in normalize.rs).
fn parse_signature(ts: &mut Tokens) -> Result<String, LangError> {
    let name = ts.expect_ident()?;
    ts.expect_token(&Token::LParen, "'('")?;
    let mut args: Vec<String> = Vec::new();
    while !matches!(ts.peek(), Some(Token::RParen) | None) {
        if !args.is_empty() {
            ts.expect_token(&Token::Comma, "','")?;
        }
        args.push(parse_type_expr(ts)?);
    }
    ts.expect_token(&Token::RParen, "')'")?;
    ts.expect_token(&Token::Arrow, "'->'")?;
    let ret = parse_type_expr(ts)?;
    Ok(format!("{}({})->{}", name, args.join(", "), ret))
}

// ---------------------------------------------------------------------------
// Goal declaration
// ---------------------------------------------------------------------------

/// `goal IDENT ( PARAMS ) -> TYPE { want { ... } constraints { ... }? }`
fn parse_goal_decl(ts: &mut Tokens) -> Result<GoalDecl, LangError> {
    ts.expect_token(&Token::Goal, "goal")?;
    let name = ts.expect_ident()?;
    ts.expect_token(&Token::LParen, "'('")?;
    let params = parse_param_list(ts)?;
    ts.expect_token(&Token::RParen, "')'")?;
    ts.expect_token(&Token::Arrow, "'->'")?;
    let returns = parse_type_expr(ts)?;
    ts.expect_token(&Token::LBrace, "'{'")?;

    let want = parse_want_block(ts)?;
    let constraints = if matches!(ts.peek(), Some(Token::Constraints)) {
        parse_constraints_block(ts)?
    } else {
        Vec::new()
    };

    ts.expect_token(&Token::RBrace, "'}' closing goal")?;
    Ok(GoalDecl {
        name,
        params,
        returns,
        want,
        constraints,
    })
}

fn parse_param_list(ts: &mut Tokens) -> Result<Vec<Param>, LangError> {
    let mut params = Vec::new();
    while !matches!(ts.peek(), Some(Token::RParen) | None) {
        if !params.is_empty() {
            ts.expect_token(&Token::Comma, "','")?;
        }
        let param_name = ts.expect_ident()?;
        ts.expect_token(&Token::Colon, "':'")?;
        let type_name = parse_type_expr(ts)?;
        params.push(Param {
            name: param_name,
            type_name,
        });
    }
    Ok(params)
}

// ---------------------------------------------------------------------------
// Want block
// ---------------------------------------------------------------------------

fn parse_want_block(ts: &mut Tokens) -> Result<WantBlock, LangError> {
    ts.expect_token(&Token::Want, "want")?;
    ts.expect_token(&Token::LBrace, "'{' after want")?;
    let stmts = parse_stmt_list(ts, false)?;
    ts.expect_token(&Token::RBrace, "'}' closing want")?;
    Ok(WantBlock { stmts })
}

/// Parse a list of statements until `}`. `in_map` = true means we're inside
/// a map body (Return is not allowed there).
fn parse_stmt_list(ts: &mut Tokens, in_map: bool) -> Result<Vec<Stmt>, LangError> {
    let mut stmts = Vec::new();
    loop {
        match ts.peek() {
            Some(Token::RBrace) | None => break,
            Some(Token::Let) => stmts.push(parse_let_stmt(ts)?),
            Some(Token::Map) => stmts.push(parse_map_stmt(ts)?),
            Some(Token::Emit) => stmts.push(parse_emit_stmt(ts)?),
            Some(Token::Return) => {
                stmts.push(parse_return_stmt(ts)?);
                break; // return is always last
            }
            Some(tok) => {
                let (tok, line) = ts.advance().unwrap();
                return Err(LangError::UnexpectedToken {
                    expected: "let, map, emit, or return".into(),
                    got: token_display(&tok),
                    line,
                });
            }
        }
    }
    Ok(stmts)
}

fn parse_let_stmt(ts: &mut Tokens) -> Result<Stmt, LangError> {
    ts.expect_token(&Token::Let, "let")?;
    let name = ts.expect_ident()?;
    ts.expect_token(&Token::Equals, "'='")?;
    let expr = parse_call_expr(ts)?;
    ts.expect_token(&Token::Semicolon, "';'")?;
    Ok(Stmt::Let { name, expr })
}

fn parse_map_stmt(ts: &mut Tokens) -> Result<Stmt, LangError> {
    ts.expect_token(&Token::Map, "map")?;
    let list = ts.expect_ident()?;
    ts.expect_token(&Token::As, "as")?;
    let binder = ts.expect_ident()?;
    ts.expect_token(&Token::LBrace, "'{' after map binder")?;
    let body = parse_stmt_list(ts, true)?;
    ts.expect_token(&Token::RBrace, "'}' closing map")?;
    Ok(Stmt::Map { list, binder, body })
}

fn parse_emit_stmt(ts: &mut Tokens) -> Result<Stmt, LangError> {
    ts.expect_token(&Token::Emit, "emit")?;
    let expr = parse_expr(ts)?;
    ts.expect_token(&Token::Semicolon, "';'")?;
    Ok(Stmt::Emit { expr })
}

fn parse_return_stmt(ts: &mut Tokens) -> Result<Stmt, LangError> {
    ts.expect_token(&Token::Return, "return")?;
    let expr = parse_expr(ts)?;
    ts.expect_token(&Token::Semicolon, "';'")?;
    Ok(Stmt::Return { expr })
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

/// A call expression (used in `let`): must be a Call.
fn parse_call_expr(ts: &mut Tokens) -> Result<Expr, LangError> {
    let expr = parse_expr(ts)?;
    match &expr {
        Expr::Call { .. } => Ok(expr),
        _ => {
            let line = ts.peek_line();
            Err(LangError::UnexpectedToken {
                expected: "function call".into(),
                got: "non-call expression".into(),
                line,
            })
        }
    }
}

/// Any expression: ident, string literal, or call.
fn parse_expr(ts: &mut Tokens) -> Result<Expr, LangError> {
    match ts.peek() {
        Some(Token::Ident(_)) => {
            let (tok, _) = ts.advance().unwrap();
            let Token::Ident(name) = tok else {
                unreachable!()
            };
            if matches!(ts.peek(), Some(Token::LParen)) {
                // Call expression
                ts.advance(); // consume '('
                let mut args = Vec::new();
                while !matches!(ts.peek(), Some(Token::RParen) | None) {
                    if !args.is_empty() {
                        ts.expect_token(&Token::Comma, "','")?;
                    }
                    args.push(parse_expr(ts)?);
                }
                ts.expect_token(&Token::RParen, "')'")?;
                Ok(Expr::Call { name, args })
            } else {
                Ok(Expr::Ident { name })
            }
        }
        Some(Token::StringLit(_)) => {
            let (tok, _) = ts.advance().unwrap();
            let Token::StringLit(value) = tok else {
                unreachable!()
            };
            Ok(Expr::StringLit { value })
        }
        Some(tok) => {
            let (tok, line) = ts.advance().unwrap();
            Err(LangError::UnexpectedToken {
                expected: "expression".into(),
                got: token_display(&tok),
                line,
            })
        }
        None => Err(LangError::UnexpectedEof {
            expected: "expression".into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Constraints block
// ---------------------------------------------------------------------------

fn parse_constraints_block(ts: &mut Tokens) -> Result<Vec<ConstraintExpr>, LangError> {
    ts.expect_token(&Token::Constraints, "constraints")?;
    ts.expect_token(&Token::LBrace, "'{' after constraints")?;
    let mut constraints = Vec::new();
    while !matches!(ts.peek(), Some(Token::RBrace) | None) {
        constraints.push(parse_constraint_expr(ts)?);
    }
    ts.expect_token(&Token::RBrace, "'}' closing constraints")?;
    Ok(constraints)
}

/// Parse one constraint expression:
///
/// - `IDENT . IDENT == STRING ;`   (e.g. `determinism.mode == "sealed_replay"`)
/// - `IDENT . IDENT <= NUMBER ;`   (e.g. `scheduler.max_inflight <= 2`)
/// - `IDENT ( IDENT ) <= NUMBER UNIT ;`  (e.g. `rate_limit(fetch) <= 2 req/s`)
fn parse_constraint_expr(ts: &mut Tokens) -> Result<ConstraintExpr, LangError> {
    let first_ident = ts.expect_ident()?;

    let left = if matches!(ts.peek(), Some(Token::LParen)) {
        // FnCall: `rate_limit(fetch)`
        ts.advance(); // consume '('
        let mut args = Vec::new();
        while !matches!(ts.peek(), Some(Token::RParen) | None) {
            if !args.is_empty() {
                ts.expect_token(&Token::Comma, "','")?;
            }
            args.push(ts.expect_ident()?);
        }
        ts.expect_token(&Token::RParen, "')'")?;
        ConstraintLeft::FnCall {
            name: first_ident,
            args,
        }
    } else {
        // Path: `determinism.mode` or `scheduler.max_inflight`
        let mut segments = vec![first_ident];
        while matches!(ts.peek(), Some(Token::Dot)) {
            ts.advance(); // consume '.'
            segments.push(ts.expect_ident()?);
        }
        ConstraintLeft::Path { segments }
    };

    // Operator
    let op = match ts.advance() {
        Some((Token::Equals, _)) => {
            // Expect a second `=` for `==`
            ts.expect_token(&Token::Equals, "'=' (second = of ==)")?;
            CmpOp::Eq
        }
        Some((Token::LtEq, _)) => CmpOp::LtEq,
        Some((tok, line)) => {
            return Err(LangError::UnexpectedToken {
                expected: "'==' or '<='".into(),
                got: token_display(&tok),
                line,
            })
        }
        None => {
            return Err(LangError::UnexpectedEof {
                expected: "'==' or '<='".into(),
            })
        }
    };

    // Right-hand side
    let right = match ts.advance() {
        Some((Token::StringLit(s), _)) => ConstraintValue::StringLit { value: s },
        Some((Token::Number(n), _)) => {
            // Check for `req/s`
            if matches!(ts.peek(), Some(Token::ReqPerSec)) {
                ts.advance();
                ConstraintValue::Rate {
                    value: n,
                    unit: "req/s".into(),
                }
            } else {
                ConstraintValue::Number { value: n }
            }
        }
        Some((tok, line)) => {
            return Err(LangError::UnexpectedToken {
                expected: "constraint value".into(),
                got: token_display(&tok),
                line,
            })
        }
        None => {
            return Err(LangError::UnexpectedEof {
                expected: "constraint value".into(),
            })
        }
    };

    ts.expect_token(&Token::Semicolon, "';'")?;
    Ok(ConstraintExpr { op, left, right })
}
