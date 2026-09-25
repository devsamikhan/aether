use super::ast::*;
use super::span::Span;
use super::token::{Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    None,
    Pipe,       // |>
    Or,         // or, ||
    And,        // and, &&
    Equality,   // ==, !=
    Comparison, // <, <=, >, >=
    Term,       // +, -
    Factor,     // *, /, %
    Power,      // ^
    Unary,      // -, not, !
    Call,       // (), [], .
}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program, (String, Span)> {
        let mut statements = Vec::new();
        self.skip_newlines();

        let start_span = self.current_span();

        while !self.is_at_end() {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }

        let end_span = if let Some(last) = statements.last() {
            last.span()
        } else {
            start_span
        };

        Ok(Program {
            statements,
            span: start_span.merge(end_span),
        })
    }

    fn parse_statement(&mut self) -> Result<Statement, (String, Span)> {
        self.skip_newlines();
        let token = self.peek();

        match &token.kind {
            TokenKind::Let => self.parse_let_statement(),
            TokenKind::Fn | TokenKind::Def => self.parse_fn_definition(),
            TokenKind::Global => self.parse_global_statement(),
            TokenKind::Intent => self.parse_intent_definition(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::Yield => self.parse_yield_statement(),
            TokenKind::Break => {
                let span = token.span;
                self.advance();
                self.consume_statement_terminator();
                Ok(Statement::Break(span))
            }
            TokenKind::Continue => {
                let span = token.span;
                self.advance();
                self.consume_statement_terminator();
                Ok(Statement::Continue(span))
            }
            TokenKind::Pass => {
                let span = token.span;
                self.advance();
                self.consume_statement_terminator();
                Ok(Statement::Pass(span))
            }
            TokenKind::Spawn => self.parse_spawn_statement(),
            TokenKind::Struct => self.parse_struct_definition(),
            TokenKind::Class => self.parse_class_statement(),
            TokenKind::Defer => self.parse_defer_statement(),
            TokenKind::Try => self.parse_try_statement(),
            TokenKind::Raise => self.parse_raise_statement(),
            TokenKind::Import => self.parse_import_statement(),
            TokenKind::From => self.parse_from_import_statement(),
            _ => self.parse_expr_or_assign_statement(),
        }
    }

    fn parse_let_statement(&mut self) -> Result<Statement, (String, Span)> {
        let let_token = self.advance(); // consume 'let'
        let is_mut = if self.match_token(&TokenKind::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let name = match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(("Expected variable name after 'let'".to_string(), self.current_span())),
        };
        self.advance();

        let mut type_annotation = None;
        if self.match_token(&TokenKind::Colon) {
            self.advance();
            if let TokenKind::Ident(t) = &self.peek().kind {
                type_annotation = Some(t.clone());
                self.advance();
            }
        }

        let mut initializer = None;
        if self.match_token(&TokenKind::Assign) {
            self.advance();
            initializer = Some(self.parse_expression(Precedence::None)?);
        }

        let end_span = initializer.as_ref().map(|e| e.span()).unwrap_or(let_token.span);
        self.consume_statement_terminator();

        Ok(Statement::Let {
            name,
            is_mut,
            type_annotation,
            initializer,
            span: let_token.span.merge(end_span),
        })
    }

    fn parse_global_statement(&mut self) -> Result<Statement, (String, Span)> {
        let global_tok = self.advance(); // consume 'global'
        let mut names = Vec::new();
        while !self.is_at_end() {
            let name = match &self.peek().kind {
                TokenKind::Ident(s) => s.clone(),
                _ => return Err(("Expected variable name after 'global'".to_string(), self.current_span())),
            };
            self.advance();
            names.push(name);
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }
        self.consume_statement_terminator();
        Ok(Statement::Global(names, global_tok.span))
    }

    fn parse_fn_definition(&mut self) -> Result<Statement, (String, Span)> {
        let fn_token = self.advance(); // consume 'fn' or 'def'

        let name = match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            other => {
                if let Some(s) = other.as_ident_str() {
                    s.to_string()
                } else {
                    return Err(("Expected function name after 'fn' or 'def'".to_string(), self.current_span()));
                }
            }
        };
        self.advance();

        self.consume(&TokenKind::LParen, "Expected '(' after function name")?;
        self.skip_braced_whitespace();
        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            self.skip_braced_whitespace();
            let mut is_vararg = false;
            let mut is_kwarg = false;
            if self.match_token(&TokenKind::Power) {
                self.advance();
                is_kwarg = true;
            } else if self.match_token(&TokenKind::Star) {
                self.advance();
                is_vararg = true;
            }

            let p_name = match &self.peek().kind {
                TokenKind::Ident(s) => s.clone(),
                other => {
                    if let Some(s) = other.as_ident_str() {
                        s.to_string()
                    } else {
                        return Err(("Expected parameter name".to_string(), self.current_span()));
                    }
                }
            };
            self.advance();

            let mut type_ann = None;
            if self.match_token(&TokenKind::Colon) {
                self.advance();
                if let TokenKind::Ident(t) = &self.peek().kind {
                    type_ann = Some(t.clone());
                    self.advance();
                }
            }

            let mut default_val = None;
            if self.match_token(&TokenKind::Assign) {
                self.advance();
                default_val = Some(self.parse_expression(Precedence::None)?);
            }

            params.push(Param {
                name: p_name,
                type_ann,
                default_val,
                is_vararg,
                is_kwarg,
            });

            self.skip_braced_whitespace();
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
            self.advance();
            self.skip_braced_whitespace();
        }
        self.consume(&TokenKind::RParen, "Expected ')' after parameter list")?;

        let mut return_type = None;
        if self.match_token(&TokenKind::Arrow) {
            self.advance();
            if let TokenKind::Ident(t) = &self.peek().kind {
                return_type = Some(t.clone());
                self.advance();
            }
        }

        let body = self.parse_block()?;
        let span = fn_token.span.merge(body.span);

        Ok(Statement::FnDef {
            name,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_intent_definition(&mut self) -> Result<Statement, (String, Span)> {
        let intent_token = self.advance(); // consume 'intent'

        let name = match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(("Expected intent name".to_string(), self.current_span())),
        };
        self.advance();

        let mut params = Vec::new();
        if self.match_token(&TokenKind::LParen) {
            self.advance();
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                if let TokenKind::Ident(p) = &self.peek().kind {
                    params.push(Param::new(p.clone()));
                    self.advance();
                }
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
            self.consume(&TokenKind::RParen, "Expected ')'")?;
        }

        if self.match_token(&TokenKind::Colon) {
            self.advance();
        }
        self.skip_newlines();

        let mut require = Vec::new();
        let mut ensure = Vec::new();
        let mut statements = Vec::new();

        if self.match_token(&TokenKind::Indent) {
            self.advance();
            while !self.check(&TokenKind::Dedent) && !self.is_at_end() {
                self.skip_newlines();
                if self.check(&TokenKind::Dedent) {
                    break;
                }

                if self.match_token(&TokenKind::Require) {
                    self.advance();
                    if self.match_token(&TokenKind::Colon) { self.advance(); }
                    require.push(self.parse_expression(Precedence::None)?);
                    self.consume_statement_terminator();
                } else if self.match_token(&TokenKind::Ensure) {
                    self.advance();
                    if self.match_token(&TokenKind::Colon) { self.advance(); }
                    ensure.push(self.parse_expression(Precedence::None)?);
                    self.consume_statement_terminator();
                } else if matches!(&self.peek().kind, TokenKind::Ident(s) if s == "body") {
                    self.advance();
                    if self.match_token(&TokenKind::Colon) { self.advance(); }
                    self.skip_newlines();
                    if self.match_token(&TokenKind::Indent) {
                        self.advance();
                        while !self.check(&TokenKind::Dedent) && !self.is_at_end() {
                            self.skip_newlines();
                            if self.check(&TokenKind::Dedent) { break; }
                            statements.push(self.parse_statement()?);
                            self.skip_newlines();
                        }
                        self.consume(&TokenKind::Dedent, "Expected dedent after body block")?;
                    }
                } else {
                    statements.push(self.parse_statement()?);
                }
                self.skip_newlines();
            }
            self.consume(&TokenKind::Dedent, "Expected dedent after intent body")?;
        }

        let span = intent_token.span;
        let body = Block {
            statements,
            result: None,
            span,
        };

        Ok(Statement::IntentDef {
            name,
            params,
            require,
            ensure,
            body,
            span,
        })
    }

    fn parse_while_statement(&mut self) -> Result<Statement, (String, Span)> {
        let while_tok = self.advance();
        let condition = self.parse_expression(Precedence::None)?;
        let body = self.parse_block()?;
        let span = while_tok.span.merge(body.span);
        Ok(Statement::While {
            condition,
            body,
            span,
        })
    }

    fn parse_for_statement(&mut self) -> Result<Statement, (String, Span)> {
        let for_tok = self.advance();
        let item = match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(("Expected identifier after 'for'".to_string(), self.current_span())),
        };
        self.advance();

        self.consume(&TokenKind::In, "Expected 'in' after for variable")?;
        let iter = self.parse_expression(Precedence::None)?;
        let body = self.parse_block()?;
        let span = for_tok.span.merge(body.span);
        Ok(Statement::For {
            item,
            iter,
            body,
            span,
        })
    }

    fn parse_return_statement(&mut self) -> Result<Statement, (String, Span)> {
        let ret_tok = self.advance();
        let mut value = None;
        if !self.check(&TokenKind::Newline) && !self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::Dedent) && !self.is_at_end() {
            let first = self.parse_expression(Precedence::None)?;
            if self.match_token(&TokenKind::Comma) {
                let mut items = vec![first];
                while self.match_token(&TokenKind::Comma) {
                    self.advance();
                    if self.check(&TokenKind::Newline) || self.check(&TokenKind::Semicolon) || self.check(&TokenKind::Dedent) || self.is_at_end() {
                        break;
                    }
                    items.push(self.parse_expression(Precedence::None)?);
                }
                let span = items.first().unwrap().span().merge(items.last().unwrap().span());
                value = Some(Expr::Tuple(items, span));
            } else {
                value = Some(first);
            }
        }
        let end_span = value.as_ref().map(|v| v.span()).unwrap_or(ret_tok.span);
        self.consume_statement_terminator();
        Ok(Statement::Return {
            value,
            span: ret_tok.span.merge(end_span),
        })
    }

    fn parse_yield_statement(&mut self) -> Result<Statement, (String, Span)> {
        let yld_tok = self.advance();
        let mut value = None;
        if !self.check(&TokenKind::Newline) && !self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::Dedent) && !self.is_at_end() {
            value = Some(self.parse_expression(Precedence::None)?);
        }
        let end_span = value.as_ref().map(|v| v.span()).unwrap_or(yld_tok.span);
        self.consume_statement_terminator();
        Ok(Statement::Yield {
            value,
            span: yld_tok.span.merge(end_span),
        })
    }

    fn parse_spawn_statement(&mut self) -> Result<Statement, (String, Span)> {
        let spawn_tok = self.advance();
        let body = if self.check(&TokenKind::Colon)
            || self.check(&TokenKind::LBrace)
            || self.check(&TokenKind::Indent)
        {
            self.parse_block()?
        } else {
            let expr = self.parse_expression(Precedence::None)?;
            let span = expr.span();
            self.consume_statement_terminator();
            Block {
                statements: vec![Statement::Expr(expr)],
                result: None,
                span,
            }
        };
        let span = spawn_tok.span.merge(body.span);
        Ok(Statement::Spawn { body, span })
    }

    fn parse_struct_definition(&mut self) -> Result<Statement, (String, Span)> {
        let struct_token = self.advance(); // consume 'struct'
        let name = match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(("Expected struct name".to_string(), self.current_span())),
        };
        self.advance();

        let mut fields = Vec::new();

        if self.match_token(&TokenKind::LParen) {
            self.advance();
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                if let TokenKind::Ident(f) = &self.peek().kind {
                    fields.push(f.clone());
                    self.advance();
                }
                if !self.match_token(&TokenKind::Comma) { break; }
                self.advance();
            }
            self.consume(&TokenKind::RParen, "Expected ')' after struct fields")?;
        } else if self.match_token(&TokenKind::Colon) {
            self.advance();
            self.skip_newlines();
            if self.match_token(&TokenKind::Indent) {
                self.advance();
                while !self.check(&TokenKind::Dedent) && !self.is_at_end() {
                    self.skip_newlines();
                    if self.check(&TokenKind::Dedent) { break; }
                    if let TokenKind::Ident(f) = &self.peek().kind {
                        fields.push(f.clone());
                        self.advance();
                        if self.match_token(&TokenKind::Colon) {
                            self.advance();
                            if let TokenKind::Ident(_) = &self.peek().kind {
                                self.advance();
                            }
                        }
                    }
                    self.consume_statement_terminator();
                }
                self.consume(&TokenKind::Dedent, "Expected dedent after struct fields")?;
            } else {
                while !self.check(&TokenKind::Newline) && !self.check(&TokenKind::Semicolon) && !self.is_at_end() {
                    if let TokenKind::Ident(f) = &self.peek().kind {
                        fields.push(f.clone());
                        self.advance();
                    }
                    if !self.match_token(&TokenKind::Comma) { break; }
                    self.advance();
                }
            }
        }

        self.consume_statement_terminator();
        let span = struct_token.span;
        Ok(Statement::StructDef { name, fields, span })
    }

    fn parse_class_statement(&mut self) -> Result<Statement, (String, Span)> {
        let class_tok = self.advance(); // consume 'class'
        let name = match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(("Expected class name after 'class'".to_string(), self.current_span())),
        };
        self.advance();

        let mut bases = Vec::new();
        if self.match_token(&TokenKind::LParen) {
            self.advance();
            self.skip_braced_whitespace();
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                self.skip_braced_whitespace();
                if let TokenKind::Ident(b) = &self.peek().kind {
                    bases.push(b.clone());
                    self.advance();
                } else {
                    return Err(("Expected base class name in inheritance list".to_string(), self.current_span()));
                }
                self.skip_braced_whitespace();
                if !self.match_token(&TokenKind::Comma) { break; }
                self.advance();
                self.skip_braced_whitespace();
            }
            self.consume(&TokenKind::RParen, "Expected ')' after base class list")?;
        }

        let mut body = Vec::new();
        let end_span;

        if self.match_token(&TokenKind::Colon) {
            self.advance(); // consume ':'
            self.skip_newlines();
            if self.match_token(&TokenKind::Indent) {
                self.advance(); // consume Indent
                while !self.check(&TokenKind::Dedent) && !self.is_at_end() {
                    self.skip_newlines();
                    if self.check(&TokenKind::Dedent) { break; }
                    let stmt = self.parse_statement()?;
                    body.push(stmt);
                    self.skip_newlines();
                }
                let dedent = self.consume(&TokenKind::Dedent, "Expected dedent after class body")?;
                end_span = dedent.span;
            } else {
                // Single line body (e.g. class Empty: pass)
                let stmt = self.parse_statement()?;
                end_span = stmt.span();
                body.push(stmt);
            }
        } else if self.match_token(&TokenKind::LBrace) {
            self.advance(); // consume '{'
            self.skip_newlines();
            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                self.skip_newlines();
                if self.check(&TokenKind::RBrace) { break; }
                let stmt = self.parse_statement()?;
                body.push(stmt);
                self.skip_newlines();
            }
            let rbrace = self.consume(&TokenKind::RBrace, "Expected '}' after class body")?;
            end_span = rbrace.span;
        } else {
            return Err(("Expected ':' or '{' to start class body".to_string(), self.current_span()));
        }

        self.consume_statement_terminator();
        let span = class_tok.span.merge(end_span);
        Ok(Statement::ClassDef { name, bases, body, span })
    }

    fn parse_defer_statement(&mut self) -> Result<Statement, (String, Span)> {
        let defer_tok = self.advance();
        let expr = self.parse_expression(Precedence::None)?;
        let span = defer_tok.span.merge(expr.span());
        self.consume_statement_terminator();
        Ok(Statement::Defer { expr, span })
    }

    fn parse_try_statement(&mut self) -> Result<Statement, (String, Span)> {
        let try_tok = self.advance(); // consume 'try'
        let try_block = self.parse_block()?;
        self.skip_newlines();

        let mut handlers = Vec::new();
        while self.match_token(&TokenKind::Except) {
            let except_tok = self.advance(); // consume 'except'
            let mut exception_type = None;
            let mut as_name = None;

            // Optional exception type if not followed immediately by colon, newline, or brace
            if !self.check(&TokenKind::Colon) && !self.check(&TokenKind::Newline) && !self.check(&TokenKind::LBrace) && !self.check(&TokenKind::As) {
                if let TokenKind::Ident(t) = &self.peek().kind {
                    exception_type = Some(t.clone());
                    self.advance();
                }
            }

            // Optional 'as name'
            if self.match_token(&TokenKind::As) {
                self.advance(); // consume 'as'
                if let TokenKind::Ident(n) = &self.peek().kind {
                    as_name = Some(n.clone());
                    self.advance();
                } else {
                    return Err(("Expected identifier after 'as'".to_string(), self.current_span()));
                }
            }

            let body = self.parse_block()?;
            let span = except_tok.span.merge(body.span);
            handlers.push(ExceptHandler {
                exception_type,
                as_name,
                body,
                span,
            });
            self.skip_newlines();
        }

        let mut finally_block = None;
        if self.match_token(&TokenKind::Finally) {
            self.advance(); // consume 'finally'
            finally_block = Some(self.parse_block()?);
        }

        if handlers.is_empty() && finally_block.is_none() {
            return Err(("Expected at least one 'except' or 'finally' block after 'try'".to_string(), try_tok.span));
        }

        let end_span = if let Some(fb) = &finally_block {
            fb.span
        } else if let Some(lh) = handlers.last() {
            lh.span
        } else {
            try_block.span
        };

        Ok(Statement::TryCatch {
            try_block,
            handlers,
            finally_block,
            span: try_tok.span.merge(end_span),
        })
    }

    fn parse_raise_statement(&mut self) -> Result<Statement, (String, Span)> {
        let raise_tok = self.advance(); // consume 'raise'
        let mut expr = None;
        if !self.check(&TokenKind::Newline) && !self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::Dedent) && !self.is_at_end() {
            expr = Some(self.parse_expression(Precedence::None)?);
        }
        let end_span = expr.as_ref().map(|e| e.span()).unwrap_or(raise_tok.span);
        self.consume_statement_terminator();
        Ok(Statement::Raise {
            expr,
            span: raise_tok.span.merge(end_span),
        })
    }

    fn parse_import_statement(&mut self) -> Result<Statement, (String, Span)> {
        let import_tok = self.advance(); // consume 'import'
        let mut module = match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(("Expected module name after 'import'".to_string(), self.current_span())),
        };
        self.advance();

        while self.match_token(&TokenKind::Dot) {
            self.advance();
            if let TokenKind::Ident(sub) = &self.peek().kind {
                module.push('.');
                module.push_str(sub);
                self.advance();
            } else {
                return Err(("Expected identifier after '.' in module import".to_string(), self.current_span()));
            }
        }

        let mut alias = None;
        if self.match_token(&TokenKind::As) {
            self.advance(); // consume 'as'
            if let TokenKind::Ident(a) = &self.peek().kind {
                alias = Some(a.clone());
                self.advance();
            } else {
                return Err(("Expected alias name after 'as'".to_string(), self.current_span()));
            }
        }

        let span = import_tok.span.merge(self.current_span());
        self.consume_statement_terminator();
        Ok(Statement::Import {
            module,
            alias,
            span,
        })
    }

    fn parse_from_import_statement(&mut self) -> Result<Statement, (String, Span)> {
        let from_tok = self.advance(); // consume 'from'
        let mut module = match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(("Expected module name after 'from'".to_string(), self.current_span())),
        };
        self.advance();

        while self.match_token(&TokenKind::Dot) {
            self.advance();
            if let TokenKind::Ident(sub) = &self.peek().kind {
                module.push('.');
                module.push_str(sub);
                self.advance();
            } else {
                return Err(("Expected identifier after '.' in from-import".to_string(), self.current_span()));
            }
        }

        self.consume(&TokenKind::Import, "Expected 'import' after module name in from-import")?;

        let mut symbols = Vec::new();
        while !self.is_at_end() {
            let sym = match &self.peek().kind {
                TokenKind::Ident(s) => s.clone(),
                TokenKind::Star => "*".to_string(),
                _ => return Err(("Expected symbol name after 'import'".to_string(), self.current_span())),
            };
            self.advance();

            let mut alias = None;
            if self.match_token(&TokenKind::As) {
                self.advance();
                if let TokenKind::Ident(a) = &self.peek().kind {
                    alias = Some(a.clone());
                    self.advance();
                } else {
                    return Err(("Expected alias name after 'as'".to_string(), self.current_span()));
                }
            }

            symbols.push((sym, alias));
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
            self.advance(); // consume ','
        }

        let span = from_tok.span.merge(self.current_span());
        self.consume_statement_terminator();
        Ok(Statement::FromImport {
            module,
            symbols,
            span,
        })
    }

    fn parse_expr_or_assign_statement(&mut self) -> Result<Statement, (String, Span)> {
        let mut expr = self.parse_expression(Precedence::None)?;

        // If followed by comma before an assignment operator, it's multiple target unpacking: a, b = ...
        if self.match_token(&TokenKind::Comma) {
            let mut targets = vec![expr];
            while self.match_token(&TokenKind::Comma) {
                self.advance();
                if self.check(&TokenKind::Assign) {
                    break;
                }
                targets.push(self.parse_expression(Precedence::None)?);
            }
            let span = targets.first().unwrap().span().merge(targets.last().unwrap().span());
            expr = Expr::Tuple(targets, span);
        }

        // Check for assignment operator (=, +=, -=, *=, /=, %=)
        let op = match &self.peek().kind {
            TokenKind::Assign => Some(AssignOp::Assign),
            TokenKind::PlusAssign => Some(AssignOp::AddAssign),
            TokenKind::MinusAssign => Some(AssignOp::SubAssign),
            TokenKind::StarAssign => Some(AssignOp::MulAssign),
            TokenKind::SlashAssign => Some(AssignOp::DivAssign),
            TokenKind::PercentAssign => Some(AssignOp::ModAssign),
            _ => None,
        };

        if let Some(assign_op) = op {
            self.advance();
            let mut value = self.parse_expression(Precedence::None)?;
            if self.match_token(&TokenKind::Comma) {
                let mut items = vec![value];
                while self.match_token(&TokenKind::Comma) {
                    self.advance();
                    if self.check(&TokenKind::Newline) || self.check(&TokenKind::Semicolon) || self.check(&TokenKind::Dedent) || self.is_at_end() {
                        break;
                    }
                    items.push(self.parse_expression(Precedence::None)?);
                }
                let span = items.first().unwrap().span().merge(items.last().unwrap().span());
                value = Expr::Tuple(items, span);
            }
            let span = expr.span().merge(value.span());
            self.consume_statement_terminator();
            Ok(Statement::Assign {
                target: expr,
                op: assign_op,
                value,
                span,
            })
        } else {
            self.consume_statement_terminator();
            Ok(Statement::Expr(expr))
        }
    }

    // =========================================================================
    // Block Parsing (Indentation OR Braced)
    // =========================================================================

    fn parse_block(&mut self) -> Result<Block, (String, Span)> {
        if self.match_token(&TokenKind::Colon) {
            self.advance();
        }

        self.skip_newlines();
        let start_span = self.current_span();

        if self.match_token(&TokenKind::LBrace) {
            // Braced block: { ... }
            self.advance();
            let mut statements = Vec::new();
            self.skip_braced_whitespace();

            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                statements.push(self.parse_statement()?);
                self.skip_braced_whitespace();
            }

            let end_tok = self.consume(&TokenKind::RBrace, "Expected '}' after block")?;
            return Ok(Block {
                statements,
                result: None,
                span: start_span.merge(end_tok.span),
            });
        }

        if self.match_token(&TokenKind::Indent) {
            // Indented block
            self.advance();
            let mut statements = Vec::new();
            self.skip_newlines();

            while !self.check(&TokenKind::Dedent) && !self.is_at_end() {
                statements.push(self.parse_statement()?);
                self.skip_newlines();
            }

            let end_span = self.current_span();
            self.consume(&TokenKind::Dedent, "Expected dedent after indented block")?;
            return Ok(Block {
                statements,
                result: None,
                span: start_span.merge(end_span),
            });
        }

        // Single-line block (e.g. fn add(a, b): a + b)
        let expr = self.parse_expression(Precedence::None)?;
        let span = expr.span();
        self.consume_statement_terminator();

        Ok(Block {
            statements: vec![Statement::Expr(expr.clone())],
            result: Some(Box::new(expr)),
            span,
        })
    }

    // =========================================================================
    // Pratt Expression Parsing
    // =========================================================================

    pub fn parse_expression(&mut self, precedence: Precedence) -> Result<Expr, (String, Span)> {
        let mut left = self.parse_prefix()?;

        while !self.is_at_end() && precedence < self.current_precedence() {
            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, (String, Span)> {
        let token = self.advance();

        match &token.kind {
            TokenKind::Int(v) => Ok(Expr::Literal(Literal::Int(*v), token.span)),
            TokenKind::Float(v) => Ok(Expr::Literal(Literal::Float(*v), token.span)),
            TokenKind::String(s) => Ok(Expr::Literal(Literal::String(s.clone()), token.span)),
            TokenKind::True => Ok(Expr::Literal(Literal::Bool(true), token.span)),
            TokenKind::False => Ok(Expr::Literal(Literal::Bool(false), token.span)),
            TokenKind::Nil => Ok(Expr::Literal(Literal::Nil, token.span)),
            TokenKind::Ident(name) => Ok(Expr::Ident(name.clone(), token.span)),

            // Unary operators
            TokenKind::Minus => {
                let right = self.parse_expression(Precedence::Unary)?;
                let span = token.span.merge(right.span());
                Ok(Expr::Unary(UnaryOp::Negate, Box::new(right), span))
            }
            TokenKind::Not => {
                let right = self.parse_expression(Precedence::Unary)?;
                let span = token.span.merge(right.span());
                Ok(Expr::Unary(UnaryOp::Not, Box::new(right), span))
            }

            // Grouping (expr) or Tuple: (), (expr,), (expr1, expr2)
            TokenKind::LParen => {
                self.skip_braced_whitespace();
                if self.check(&TokenKind::RParen) {
                    let end_tok = self.advance();
                    return Ok(Expr::Tuple(Vec::new(), token.span.merge(end_tok.span)));
                }
                let first = self.parse_expression(Precedence::None)?;
                self.skip_braced_whitespace();
                if self.match_token(&TokenKind::Comma) {
                    self.advance(); // consume comma
                    self.skip_braced_whitespace();
                    let mut elements = vec![first];
                    while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                        elements.push(self.parse_expression(Precedence::None)?);
                        self.skip_braced_whitespace();
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                        self.skip_braced_whitespace();
                    }
                    let end_tok = self.consume(&TokenKind::RParen, "Expected ')' after tuple")?;
                    Ok(Expr::Tuple(elements, token.span.merge(end_tok.span)))
                } else {
                    let _ = self.consume(&TokenKind::RParen, "Expected ')' after expression")?;
                    Ok(first)
                }
            }

            // Array Literal: [1, 2, 3] or List Comprehension: [x * 2 for x in items if x > 1]
            TokenKind::LBracket => {
                self.skip_braced_whitespace();
                if self.check(&TokenKind::RBracket) {
                    let end_tok = self.advance();
                    return Ok(Expr::Array(Vec::new(), token.span.merge(end_tok.span)));
                }

                let first = self.parse_expression(Precedence::None)?;
                self.skip_braced_whitespace();

                if self.match_token(&TokenKind::For) {
                    // List comprehension!
                    self.advance(); // consume 'for'
                    self.skip_braced_whitespace();
                    let item = match &self.peek().kind {
                        TokenKind::Ident(s) => s.clone(),
                        _ => return Err(("Expected variable identifier after 'for' in list comprehension".to_string(), self.current_span())),
                    };
                    self.advance();
                    self.skip_braced_whitespace();
                    self.consume(&TokenKind::In, "Expected 'in' in list comprehension")?;
                    self.skip_braced_whitespace();
                    let iter = self.parse_expression(Precedence::None)?;
                    self.skip_braced_whitespace();

                    let mut condition = None;
                    if self.match_token(&TokenKind::If) {
                        self.advance(); // consume 'if'
                        self.skip_braced_whitespace();
                        condition = Some(Box::new(self.parse_expression(Precedence::None)?));
                        self.skip_braced_whitespace();
                    }

                    let end_tok = self.consume(&TokenKind::RBracket, "Expected ']' after list comprehension")?;
                    let span = token.span.merge(end_tok.span);
                    return Ok(Expr::ListComp {
                        element: Box::new(first),
                        item,
                        iter: Box::new(iter),
                        condition,
                        span,
                    });
                }

                // Regular array
                let mut elements = vec![first];
                while self.match_token(&TokenKind::Comma) {
                    self.advance();
                    self.skip_braced_whitespace();
                    if self.check(&TokenKind::RBracket) {
                        break;
                    }
                    elements.push(self.parse_expression(Precedence::None)?);
                    self.skip_braced_whitespace();
                }
                let end_tok = self.consume(&TokenKind::RBracket, "Expected ']' after array")?;
                let span = token.span.merge(end_tok.span);
                Ok(Expr::Array(elements, span))
            }

            // Map Literal: {}, {"k": v}, Set Literal: {1, 2, 3}, or Dict Comprehension: {k: v for k in items}
            TokenKind::LBrace => {
                self.skip_braced_whitespace();
                if self.check(&TokenKind::RBrace) {
                    let end_tok = self.advance();
                    return Ok(Expr::Map(Vec::new(), token.span.merge(end_tok.span)));
                }

                let first = self.parse_expression(Precedence::None)?;
                self.skip_braced_whitespace();
                if self.match_token(&TokenKind::Colon) {
                    // Dictionary / Map or Dict Comprehension
                    self.advance(); // consume ':'
                    self.skip_braced_whitespace();
                    let val = self.parse_expression(Precedence::None)?;
                    self.skip_braced_whitespace();

                    if self.match_token(&TokenKind::For) {
                        // Dict comprehension!
                        self.advance(); // consume 'for'
                        self.skip_braced_whitespace();
                        let item = match &self.peek().kind {
                            TokenKind::Ident(s) => s.clone(),
                            _ => return Err(("Expected identifier after 'for' in dict comprehension".to_string(), self.current_span())),
                        };
                        self.advance();
                        self.skip_braced_whitespace();
                        self.consume(&TokenKind::In, "Expected 'in' in dict comprehension")?;
                        self.skip_braced_whitespace();
                        let iter = self.parse_expression(Precedence::None)?;
                        self.skip_braced_whitespace();

                        let mut condition = None;
                        if self.match_token(&TokenKind::If) {
                            self.advance(); // consume 'if'
                            self.skip_braced_whitespace();
                            condition = Some(Box::new(self.parse_expression(Precedence::None)?));
                            self.skip_braced_whitespace();
                        }

                        let end_tok = self.consume(&TokenKind::RBrace, "Expected '}' after dict comprehension")?;
                        let span = token.span.merge(end_tok.span);
                        return Ok(Expr::DictComp {
                            key: Box::new(first),
                            value: Box::new(val),
                            item,
                            iter: Box::new(iter),
                            condition,
                            span,
                        });
                    }

                    let mut entries = vec![(first, val)];
                    while self.match_token(&TokenKind::Comma) {
                        self.advance();
                        self.skip_braced_whitespace();
                        if self.check(&TokenKind::RBrace) {
                            break;
                        }
                        let k = self.parse_expression(Precedence::None)?;
                        self.skip_braced_whitespace();
                        self.consume(&TokenKind::Colon, "Expected ':' after map key")?;
                        self.skip_braced_whitespace();
                        let v = self.parse_expression(Precedence::None)?;
                        entries.push((k, v));
                        self.skip_braced_whitespace();
                    }
                    let end_tok = self.consume(&TokenKind::RBrace, "Expected '}' after map")?;
                    Ok(Expr::Map(entries, token.span.merge(end_tok.span)))
                } else {
                    // Set
                    let mut elements = vec![first];
                    self.skip_braced_whitespace();
                    while self.match_token(&TokenKind::Comma) {
                        self.advance();
                        self.skip_braced_whitespace();
                        if self.check(&TokenKind::RBrace) {
                            break;
                        }
                        elements.push(self.parse_expression(Precedence::None)?);
                        self.skip_braced_whitespace();
                    }
                    let end_tok = self.consume(&TokenKind::RBrace, "Expected '}' after set")?;
                    Ok(Expr::Set(elements, token.span.merge(end_tok.span)))
                }
            }

            // If expression
            TokenKind::If => self.parse_if_expression(token),

            // Anonymous Lambda: fn(x): x * 2
            TokenKind::Fn => {
                self.consume(&TokenKind::LParen, "Expected '(' in lambda parameter list")?;
                let mut params = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                    let mut is_vararg = false;
                    let mut is_kwarg = false;
                    if self.match_token(&TokenKind::Power) {
                        self.advance();
                        is_kwarg = true;
                    } else if self.match_token(&TokenKind::Star) {
                        self.advance();
                        is_vararg = true;
                    }
                    if let TokenKind::Ident(p) = &self.peek().kind {
                        let p_name = p.clone();
                        self.advance();
                        let mut default_val = None;
                        if self.match_token(&TokenKind::Assign) {
                            self.advance();
                            default_val = Some(self.parse_expression(Precedence::None)?);
                        }
                        params.push(Param {
                            name: p_name,
                            type_ann: None,
                            default_val,
                            is_vararg,
                            is_kwarg,
                        });
                    }
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                }
                self.consume(&TokenKind::RParen, "Expected ')' in lambda")?;
                let body = self.parse_block()?;
                let span = token.span.merge(body.span);
                Ok(Expr::Lambda(params, Box::new(body), span))
            }

            TokenKind::Lambda => self.parse_lambda_expression(token),
            TokenKind::Super => {
                let mut span = token.span;
                if self.match_token(&TokenKind::LParen) {
                    self.advance();
                    let rparen = self.consume(&TokenKind::RParen, "Expected ')' after 'super'")?;
                    span = span.merge(rparen.span);
                }
                Ok(Expr::Super(span))
            },

            TokenKind::Channel => Ok(Expr::Ident("channel".to_string(), token.span)),
            TokenKind::Spawn => Ok(Expr::Ident("spawn".to_string(), token.span)),
            TokenKind::Type => Ok(Expr::Ident("type".to_string(), token.span)),
            TokenKind::Match => self.parse_match_expression(token),

            _ => Err((format!("Unexpected token in expression: '{:?}'", token.kind), token.span)),
        }
    }

    fn parse_if_expression(&mut self, token: Token) -> Result<Expr, (String, Span)> {
        let cond = self.parse_expression(Precedence::None)?;
        let then_branch = self.parse_block()?;
        let mut else_branch = None;

        self.skip_newlines();
        if self.match_token(&TokenKind::Elif) {
            let elif_tok = self.advance();
            let inner_if = self.parse_if_expression(elif_tok)?;
            let inner_span = inner_if.span();
            else_branch = Some(Block {
                statements: vec![],
                result: Some(Box::new(inner_if)),
                span: inner_span,
            });
        } else if self.match_token(&TokenKind::Else) {
            self.advance();
            else_branch = Some(self.parse_block()?);
        }

        let end_span = else_branch.as_ref().map(|b| b.span).unwrap_or(then_branch.span);
        let span = token.span.merge(end_span);

        Ok(Expr::If {
            condition: Box::new(cond),
            then_branch,
            else_branch,
            span,
        })
    }

    fn parse_lambda_expression(&mut self, token: Token) -> Result<Expr, (String, Span)> {
        let mut params = Vec::new();
        while !self.check(&TokenKind::Colon) && !self.is_at_end() {
            let mut is_vararg = false;
            let mut is_kwarg = false;
            if self.match_token(&TokenKind::Power) {
                self.advance();
                is_kwarg = true;
            } else if self.match_token(&TokenKind::Star) {
                self.advance();
                is_vararg = true;
            }

            let p_name = match &self.peek().kind {
                TokenKind::Ident(s) => s.clone(),
                _ => return Err(("Expected parameter name in lambda".to_string(), self.current_span())),
            };
            self.advance();

            let mut default_val = None;
            if self.match_token(&TokenKind::Assign) {
                self.advance();
                default_val = Some(self.parse_expression(Precedence::None)?);
            }

            params.push(Param {
                name: p_name,
                type_ann: None,
                default_val,
                is_vararg,
                is_kwarg,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }

        self.consume(&TokenKind::Colon, "Expected ':' in lambda expression")?;
        let body_expr = self.parse_expression(Precedence::None)?;
        let span = token.span.merge(body_expr.span());
        let body = Block {
            statements: Vec::new(),
            result: Some(Box::new(body_expr)),
            span,
        };
        Ok(Expr::Lambda(params, Box::new(body), span))
    }

    fn parse_match_expression(&mut self, match_token: Token) -> Result<Expr, (String, Span)> {
        let target = self.parse_expression(Precedence::None)?;
        if self.match_token(&TokenKind::Colon) { self.advance(); }
        self.skip_newlines();

        let mut arms = Vec::new();
        if self.match_token(&TokenKind::Indent) {
            self.advance();
            while !self.check(&TokenKind::Dedent) && !self.is_at_end() {
                self.skip_newlines();
                if self.check(&TokenKind::Dedent) { break; }

                let arm_start = self.current_span();
                // Optional Pythonic 'case' keyword:
                if self.match_token(&TokenKind::Case) {
                    self.advance();
                }

                let pattern = self.parse_match_pattern()?;
                
                // Separator: '=>' (Rust/Aether) or ':' (Python case 1:)
                if self.match_token(&TokenKind::FatArrow) {
                    self.advance();
                } else if self.match_token(&TokenKind::Colon) {
                    self.advance();
                } else {
                    return Err(("Expected '=>' or ':' after match pattern".to_string(), self.current_span()));
                }

                self.skip_newlines();
                // Support both indented block body or single expression:
                let body = if self.match_token(&TokenKind::Indent) {
                    let block = self.parse_block()?;
                    let span = block.span;
                    Expr::Block(block, span)
                } else {
                    self.parse_expression(Precedence::None)?
                };

                let arm_span = arm_start.merge(body.span());
                arms.push(MatchArm {
                    pattern,
                    body,
                    span: arm_span,
                });
                self.consume_statement_terminator();
            }
            self.consume(&TokenKind::Dedent, "Expected dedent after match arms")?;
        }

        let span = match_token.span;
        Ok(Expr::Match {
            target: Box::new(target),
            arms,
            span,
        })
    }

    fn parse_match_pattern(&mut self) -> Result<MatchPattern, (String, Span)> {
        let tok = self.advance();
        match &tok.kind {
            TokenKind::Int(i) => {
                if self.match_token(&TokenKind::DotDot) {
                    self.advance();
                    if let TokenKind::Int(end_i) = &self.peek().kind {
                        let end_val = *end_i;
                        self.advance();
                        Ok(MatchPattern::Range(Literal::Int(*i), Literal::Int(end_val)))
                    } else {
                        Err(("Expected integer after '..'".to_string(), self.current_span()))
                    }
                } else {
                    Ok(MatchPattern::Literal(Literal::Int(*i)))
                }
            }
            TokenKind::Float(f) => Ok(MatchPattern::Literal(Literal::Float(*f))),
            TokenKind::String(s) => Ok(MatchPattern::Literal(Literal::String(s.clone()))),
            TokenKind::True => Ok(MatchPattern::Literal(Literal::Bool(true))),
            TokenKind::False => Ok(MatchPattern::Literal(Literal::Bool(false))),
            TokenKind::Nil => Ok(MatchPattern::Literal(Literal::Nil)),
            TokenKind::Ident(name) => {
                if name == "_" {
                    Ok(MatchPattern::Wildcard)
                } else {
                    Ok(MatchPattern::Ident(name.clone()))
                }
            }
            _ => Err((format!("Invalid match pattern token '{:?}'", tok.kind), tok.span)),
        }
    }

    fn parse_infix(&mut self, left: Expr) -> Result<Expr, (String, Span)> {
        let op_tok = self.advance();

        match &op_tok.kind {
            // Pipeline Operator: data |> func(args)
            TokenKind::PipeRight => {
                let right = self.parse_expression(Precedence::Pipe)?;
                let span = left.span().merge(right.span());
                Ok(Expr::Pipe(Box::new(left), Box::new(right), span))
            }

            // Binary Operators
            TokenKind::Plus => self.parse_binary(left, BinaryOp::Add, Precedence::Term),
            TokenKind::Minus => self.parse_binary(left, BinaryOp::Sub, Precedence::Term),
            TokenKind::Star => self.parse_binary(left, BinaryOp::Mul, Precedence::Factor),
            TokenKind::Slash => self.parse_binary(left, BinaryOp::Div, Precedence::Factor),
            TokenKind::Percent => self.parse_binary(left, BinaryOp::Mod, Precedence::Factor),
            TokenKind::Power => self.parse_binary(left, BinaryOp::Pow, Precedence::Power),
            TokenKind::EqualEqual => self.parse_binary(left, BinaryOp::Equal, Precedence::Equality),
            TokenKind::NotEqual => self.parse_binary(left, BinaryOp::NotEqual, Precedence::Equality),
            TokenKind::Less => self.parse_binary(left, BinaryOp::Less, Precedence::Comparison),
            TokenKind::LessEqual => self.parse_binary(left, BinaryOp::LessEqual, Precedence::Comparison),
            TokenKind::Greater => self.parse_binary(left, BinaryOp::Greater, Precedence::Comparison),
            TokenKind::GreaterEqual => self.parse_binary(left, BinaryOp::GreaterEqual, Precedence::Comparison),
            TokenKind::In => self.parse_binary(left, BinaryOp::In, Precedence::Comparison),
            TokenKind::And => self.parse_binary(left, BinaryOp::And, Precedence::And),
            TokenKind::Or => self.parse_binary(left, BinaryOp::Or, Precedence::Or),

            // Function Call: f(a, b)
            TokenKind::LParen => {
                let mut args = Vec::new();
                self.skip_braced_whitespace();
                while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                    self.skip_braced_whitespace();
                    if self.match_token(&TokenKind::Power) {
                        let star_tok = self.advance();
                        let inner = self.parse_expression(Precedence::None)?;
                        let span = star_tok.span.merge(inner.span());
                        args.push(Expr::DoubleStarArg(Box::new(inner), span));
                    } else if self.match_token(&TokenKind::Star) {
                        let star_tok = self.advance();
                        let inner = self.parse_expression(Precedence::None)?;
                        let span = star_tok.span.merge(inner.span());
                        args.push(Expr::StarArg(Box::new(inner), span));
                    } else if matches!(&self.peek().kind, TokenKind::Ident(_)) && self.peek_next().map(|t| &t.kind) == Some(&TokenKind::Assign) {
                        let name = if let TokenKind::Ident(s) = &self.peek().kind { s.clone() } else { unreachable!() };
                        let id_tok = self.advance();
                        self.advance(); // consume '='
                        let val = self.parse_expression(Precedence::None)?;
                        let span = id_tok.span.merge(val.span());
                        args.push(Expr::NamedArg(name, Box::new(val), span));
                    } else {
                        args.push(self.parse_expression(Precedence::None)?);
                    }
                    self.skip_braced_whitespace();
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                    self.skip_braced_whitespace();
                }
                let end_tok = self.consume(&TokenKind::RParen, "Expected ')' after call arguments")?;
                let span = left.span().merge(end_tok.span);
                Ok(Expr::Call(Box::new(left), args, span))
            }

            // Index Access: arr[i]
            TokenKind::LBracket => {
                let index = self.parse_expression(Precedence::None)?;
                let end_tok = self.consume(&TokenKind::RBracket, "Expected ']' after index")?;
                let span = left.span().merge(end_tok.span);
                Ok(Expr::Index(Box::new(left), Box::new(index), span))
            }

            // Member Access: obj.field
            TokenKind::Dot => {
                let member_name = match &self.peek().kind {
                    TokenKind::Ident(s) => s.clone(),
                    other => {
                        if let Some(s) = other.as_ident_str() {
                            s.to_string()
                        } else {
                            return Err(("Expected identifier after '.'".to_string(), self.current_span()));
                        }
                    }
                };
                let member_tok = self.advance();
                let span = left.span().merge(member_tok.span);
                Ok(Expr::Member(Box::new(left), member_name, span))
            }

            _ => Err((format!("Unexpected infix operator '{:?}'", op_tok.kind), op_tok.span)),
        }
    }

    fn parse_binary(&mut self, left: Expr, op: BinaryOp, prec: Precedence) -> Result<Expr, (String, Span)> {
        let right = self.parse_expression(prec)?;
        let span = left.span().merge(right.span());
        Ok(Expr::Binary(Box::new(left), op, Box::new(right), span))
    }

    fn current_precedence(&self) -> Precedence {
        if self.is_at_end() {
            return Precedence::None;
        }

        match &self.peek().kind {
            TokenKind::PipeRight => Precedence::Pipe,
            TokenKind::Or => Precedence::Or,
            TokenKind::And => Precedence::And,
            TokenKind::EqualEqual | TokenKind::NotEqual => Precedence::Equality,
            TokenKind::Less | TokenKind::LessEqual | TokenKind::Greater | TokenKind::GreaterEqual | TokenKind::In => Precedence::Comparison,
            TokenKind::Plus | TokenKind::Minus => Precedence::Term,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Factor,
            TokenKind::Power => Precedence::Power,
            TokenKind::LParen | TokenKind::LBracket | TokenKind::Dot => Precedence::Call,
            _ => Precedence::None,
        }
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len() || self.peek().kind == TokenKind::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn peek_next(&self) -> Option<&Token> {
        if self.cursor + 1 < self.tokens.len() {
            Some(&self.tokens[self.cursor + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.cursor].clone();
        if self.cursor < self.tokens.len() - 1 {
            self.cursor += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            &self.peek().kind == kind
        }
    }

    fn match_token(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            &self.peek().kind == kind
        }
    }

    fn consume(&mut self, kind: &TokenKind, err_msg: &str) -> Result<Token, (String, Span)> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err((format!("{}: found {:?}", err_msg, self.peek().kind), self.current_span()))
        }
    }

    fn current_span(&self) -> Span {
        if self.cursor < self.tokens.len() {
            self.tokens[self.cursor].span
        } else if let Some(last) = self.tokens.last() {
            last.span
        } else {
            Span::default()
        }
    }

    fn skip_newlines(&mut self) {
        while !self.is_at_end() && (self.peek().kind == TokenKind::Newline || self.peek().kind == TokenKind::Semicolon) {
            self.advance();
        }
    }

    fn consume_statement_terminator(&mut self) {
        while !self.is_at_end() && (self.peek().kind == TokenKind::Newline || self.peek().kind == TokenKind::Semicolon) {
            self.advance();
        }
    }

    fn skip_braced_whitespace(&mut self) {
        while !self.is_at_end() && (
            self.peek().kind == TokenKind::Newline ||
            self.peek().kind == TokenKind::Semicolon ||
            self.peek().kind == TokenKind::Indent ||
            self.peek().kind == TokenKind::Dedent
        ) {
            self.advance();
        }
    }
}
