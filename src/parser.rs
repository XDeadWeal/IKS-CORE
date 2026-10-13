use crate::ast::*;
use crate::diag::{Diag, Diagnostics, Span};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<Diag>,
}

pub fn parse(tokens: Vec<Token>) -> Result<Program, Diagnostics> {
    let mut p = Parser { tokens, pos: 0, errors: Vec::new() };
    let prog = p.program();
    if !p.errors.is_empty() {
        return Err(Diagnostics { errors: p.errors, warnings: Vec::new() });
    }
    prog
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_at(&self, off: usize) -> &TokenKind {
        &self.tokens[(self.pos + off).min(self.tokens.len() - 1)].kind
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek_kind().same_variant(kind)
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.at(&kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, Diag> {
        if self.at(&kind) {
            Ok(self.advance())
        } else {
            Err(Diag::new(
                self.peek().span,
                format!("expected {}, found {}", kind.describe(), self.peek_kind().describe()),
            ))
        }
    }

    fn expect_ident(&mut self, what: &str) -> Result<(String, Span), Diag> {
        let tok = self.peek().clone();
        if let TokenKind::Ident(name) = &tok.kind {
            self.advance();
            Ok((name.clone(), tok.span))
        } else {
            Err(Diag::new(
                tok.span,
                format!("expected {}, found {}", what, tok.kind.describe()),
            ))
        }
    }

    fn program(&mut self) -> Result<Program, Diagnostics> {
        let mut funcs = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut imports = Vec::new();
        while !self.at(&TokenKind::Eof) {
            let start_pos = self.pos;
            match self.parse_top_item(&mut funcs, &mut structs, &mut enums, &mut imports) {
                Ok(()) => {}
                Err(e) => {
                    self.errors.push(e);
                    self.sync_top();
                    if self.pos == start_pos {
                        // Ensure forward progress to avoid an infinite loop.
                        self.advance();
                    }
                }
            }
        }
        if !self.errors.is_empty() {
            return Err(Diagnostics { errors: std::mem::take(&mut self.errors), warnings: Vec::new() });
        }
        Ok(Program { funcs, structs, enums, imports, warnings: Vec::new() })
    }

    /// Parse one top-level item, pushing it into the given vectors.
    fn parse_top_item(
        &mut self,
        funcs: &mut Vec<FnDecl>,
        structs: &mut Vec<StructDecl>,
        enums: &mut Vec<EnumDecl>,
        imports: &mut Vec<ImportDecl>,
    ) -> Result<(), Diag> {
        match self.peek_kind().clone() {
            TokenKind::Import => {
                let span = self.advance().span;
                let tok = self.peek().clone();
                let TokenKind::Str(path) = &tok.kind else {
                    return Err(Diag::new(
                        tok.span,
                        format!("expected a quoted file path after 'import', found {}", tok.kind.describe()),
                    ));
                };
                let path = path.clone();
                self.advance();
                self.expect(TokenKind::Semi)?;
                imports.push(ImportDecl { path, span });
                Ok(())
            }
            TokenKind::Struct => {
                structs.push(self.struct_decl()?);
                Ok(())
            }
            TokenKind::Enum => {
                enums.push(self.enum_decl()?);
                Ok(())
            }
            TokenKind::Extern => {
                funcs.push(self.extern_fn()?);
                Ok(())
            }
            TokenKind::Fn => {
                funcs.push(self.fn_decl(false)?);
                Ok(())
            }
            other => Err(Diag::new(
                self.peek().span,
                format!("expected a top-level item (fn, extern fn, struct, enum or import), found {}", other.describe()),
            )),
        }
    }

    /// Skip tokens until the start of the next plausible top-level item.
    fn sync_top(&mut self) {
        while !self.at(&TokenKind::Eof) {
            match self.peek_kind() {
                TokenKind::Fn
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Extern
                | TokenKind::Import => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn struct_decl(&mut self) -> Result<StructDecl, Diag> {
        let span = self.expect(TokenKind::Struct)?.span;
        let (name, _) = self.expect_ident("struct name")?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (fname, fspan) = self.expect_ident("field name")?;
            self.expect(TokenKind::Colon)?;
            let (fty, _) = self.parse_type()?;
            if fty == Ty::Void {
                return Err(Diag::new(fspan, "field cannot have type 'void'"));
            }
            fields.push(Field { name: fname, ty: fty, span: fspan });
            if !self.eat(TokenKind::Comma) && !self.eat(TokenKind::Semi) && !self.at(&TokenKind::RBrace) {
                return Err(Diag::new(
                    self.peek().span,
                    format!("expected ',' or '}}', found {}", self.peek_kind().describe()),
                ));
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(StructDecl { name, fields, span })
    }

    fn enum_decl(&mut self) -> Result<EnumDecl, Diag> {
        let span = self.expect(TokenKind::Enum)?.span;
        let (name, _) = self.expect_ident("enum name")?;
        if name == "nil" {
            return Err(Diag::new(span, "enum name cannot be 'nil'"));
        }
        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (vname, vspan) = self.expect_ident("variant name")?;
            if variants.iter().any(|v: &VariantDecl| v.name == vname) {
                return Err(Diag::new(vspan, format!("duplicate variant '{}' in enum '{}'", vname, name)));
            }
            let mut params = Vec::new();
            if self.eat(TokenKind::LParen) {
                if !self.at(&TokenKind::RParen) {
                    loop {
                        let (pty, _) = self.parse_type()?;
                        if pty == Ty::Void {
                            return Err(Diag::new(vspan, "variant payload cannot have type 'void'"));
                        }
                        params.push(pty);
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                        if self.at(&TokenKind::RParen) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen)?;
            }
            variants.push(VariantDecl { name: vname, params, span: vspan });
            if !self.eat(TokenKind::Comma) && !self.eat(TokenKind::Semi) && !self.at(&TokenKind::RBrace) {
                return Err(Diag::new(
                    self.peek().span,
                    format!("expected ',' or '}}', found {}", self.peek_kind().describe()),
                ));
            }
        }
        if variants.is_empty() {
            return Err(Diag::new(span, format!("enum '{}' must declare at least one variant", name)));
        }
        self.expect(TokenKind::RBrace)?;
        Ok(EnumDecl { name, variants, span })
    }

    /// Parse one `case` pattern: `Literal`, `Name` or `Name(a, b)`.
    ///
    /// `Name(...)` is only committed to as a variant pattern when it is
    /// followed by the tokens that legally end a pattern (`,` or `:`); on any
    /// other parse failure the position is rewound and the pattern is read as
    /// an ordinary expression, so existing value switches keep working.
    fn pattern(&mut self) -> Result<Pattern, Diag> {
        if let TokenKind::Ident(name) = self.peek_kind().clone() {
            if self.peek_at(1).same_variant(&TokenKind::LParen) {
                let save = self.pos;
                match self.variant_bindings() {
                    Ok((bindings, span)) => {
                        if self.at(&TokenKind::Comma) || self.at(&TokenKind::Colon) {
                            return Ok(Pattern::Variant { name, bindings, span });
                        }
                        self.pos = save;
                    }
                    Err(_) => self.pos = save,
                }
            } else if self.peek_at(1).same_variant(&TokenKind::Colon)
                || self.peek_at(1).same_variant(&TokenKind::Comma)
            {
                let span = self.advance().span;
                return Ok(Pattern::Bare { name, span });
            }
        }
        Ok(Pattern::Value(self.expr()?))
    }

    /// `Name(binding, ...)` after the variant name has been consumed by
    /// [`Parser::pattern`]'s lookahead. Returns the bindings and the span of
    /// the variant name.
    fn variant_bindings(&mut self) -> Result<(Vec<String>, Span), Diag> {
        let span = self.advance().span;
        self.expect(TokenKind::LParen)?;
        let mut bindings = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                let (b, _) = self.expect_ident("binding name")?;
                bindings.push(b);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
                if self.at(&TokenKind::RParen) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok((bindings, span))
    }

    fn extern_fn(&mut self) -> Result<FnDecl, Diag> {
        let span = self.expect(TokenKind::Extern)?.span;
        let tok = self.peek().clone();
        if !matches!(&tok.kind, TokenKind::Str(c) if c == "C") {
            return Err(Diag::new(
                tok.span,
                format!("expected \"C\" after 'extern', found {}", tok.kind.describe()),
            ));
        }
        self.advance();
        let f = self.fn_decl(true)?;
        let mut f = f;
        f.span = span;
        self.expect(TokenKind::Semi)?;
        Ok(f)
    }

    fn fn_decl(&mut self, is_extern: bool) -> Result<FnDecl, Diag> {
        let start = self.expect(TokenKind::Fn)?.span;
        let (name, _) = self.expect_ident("function name")?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                let (pname, pspan) = self.expect_ident("parameter name")?;
                self.expect(TokenKind::Colon)?;
                let (pty, pty_span) = self.parse_type()?;
                if pty == Ty::Void {
                    return Err(Diag::new(pty_span, "parameter cannot have type 'void'"));
                }
                params.push(Param { name: pname, ty: pty, span: pspan });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen)?;
        let ret = if self.eat(TokenKind::Arrow) {
            self.parse_type()?.0
        } else {
            Ty::Void
        };
        let body = if is_extern {
            Block { stmts: Vec::new(), span: start }
        } else {
            self.block()?
        };
        Ok(FnDecl { name, params, ret, body, span: start, is_extern })
    }

    fn parse_type(&mut self) -> Result<(Ty, Span), Diag> {
        let (name, span) = self.expect_ident("a type name")?;
        let base = match name.as_str() {
            "int" => Ty::Int,
            "float" => Ty::Float,
            "bool" => Ty::Bool,
            "string" => Ty::Str,
            "void" => Ty::Void,
            "ptr" => Ty::Ptr,
            "map" => Ty::HashMap,
            "vec2" => Ty::Vec2,
            "vec3" => Ty::Vec3,
            "vec4" => Ty::Vec4,
            _ => Ty::Named(name),
        };
        if self.eat(TokenKind::LBracket) {
            self.expect(TokenKind::RBracket)?;
            if base == Ty::Void {
                return Err(Diag::new(span, "type 'void' cannot be used as an array element"));
            }
            return Ok((Ty::Array(Box::new(base)), span));
        }
        Ok((base, span))
    }

    fn block(&mut self) -> Result<Block, Diag> {
        let start = self.expect(TokenKind::LBrace)?.span;
        let mut stmts = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let start_pos = self.pos;
            match self.stmt() {
                Ok(s) => stmts.push(s),
                Err(e) => {
                    // Recover inside a block: record and skip to the next statement boundary.
                    self.errors.push(e);
                    self.sync_stmt();
                    if self.pos == start_pos {
                        self.advance();
                    }
                }
            }
        }
        let end = self.expect(TokenKind::RBrace)?.span;
        Ok(Block { stmts, span: Span::new(start.file, start.line, start.col.max(end.col)) })
    }

    /// Skip tokens until the next likely statement start (a `;` is consumed).
    fn sync_stmt(&mut self) {
        let mut depth = 0usize;
        while !self.at(&TokenKind::Eof) {
            match self.peek_kind() {
                TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                    self.advance();
                }
                TokenKind::Semi => {
                    self.advance();
                    if depth == 0 {
                        return;
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn stmt(&mut self) -> Result<Stmt, Diag> {
        match self.peek_kind().clone() {
            TokenKind::Var | TokenKind::Let => {
                let mutable = self.peek_kind() == &TokenKind::Var;
                let span = self.advance().span;
                let (name, _) = self.expect_ident("variable name")?;
                let ty = if self.eat(TokenKind::Colon) {
                    Some(self.parse_type()?.0)
                } else {
                    None
                };
                self.expect(TokenKind::Assign)?;
                let init = self.expr()?;
                self.expect(TokenKind::Semi)?;
                Ok(Stmt::Var { name, ty, init, mutable, span })
            }
            TokenKind::If => self.if_stmt(),
            TokenKind::While => {
                let span = self.advance().span;
                self.expect(TokenKind::LParen)?;
                let cond = self.expr()?;
                self.expect(TokenKind::RParen)?;
                let body = self.block()?;
                Ok(Stmt::While { cond, body, span })
            }
            TokenKind::For => {
                let span = self.advance().span;
                self.expect(TokenKind::LParen)?;
                if self.at(&TokenKind::Var) || self.at(&TokenKind::Let) {
                    let mutable = self.peek_kind() == &TokenKind::Var;
                    let var_span = self.advance().span;
                    let (name, _) = self.expect_ident("loop variable name")?;
                    let ty = if self.eat(TokenKind::Colon) {
                        Some(self.parse_type()?.0)
                    } else {
                        None
                    };
                    if self.eat(TokenKind::In) {
                        let start = self.expr()?;
                        self.expect(TokenKind::DotDot)?;
                        let end = self.expr()?;
                        self.expect(TokenKind::RParen)?;
                        return self.build_for_range(span, var_span, name, ty.or(Some(Ty::Int)), start, end);
                    } else {
                        self.expect(TokenKind::Assign)?;
                        let init = self.expr()?;
                        self.expect(TokenKind::Semi)?;
                        let cond = if self.at(&TokenKind::Semi) {
                            Expr::Bool(true, self.peek().span)
                        } else {
                            self.expr()?
                        };
                        self.expect(TokenKind::Semi)?;
                        let step = if self.at(&TokenKind::RParen) {
                            None
                        } else {
                            Some(self.assign_like_stmt()?)
                        };
                        self.expect(TokenKind::RParen)?;
                        let body = self.block()?;
                        let init_stmt = Stmt::Var { name, ty, init, mutable, span: var_span };
                        let mut loop_body = body.stmts;
                        if let Some(s) = step {
                            loop_body.push(s);
                        }
                        let body = Block { stmts: loop_body, span: body.span };
                        let while_stmt = Stmt::While { cond, body, span };
                        return Ok(Stmt::Block(Block { stmts: vec![init_stmt, while_stmt], span }));
                    }
                } else if matches!(self.peek_kind(), TokenKind::Ident(_)) {
                    let (name, name_span) = self.expect_ident("loop variable")?;
                    if self.eat(TokenKind::In) {
                        let start = self.expr()?;
                        self.expect(TokenKind::DotDot)?;
                        let end = self.expr()?;
                        self.expect(TokenKind::RParen)?;
                        return self.build_for_range(span, name_span, name, Some(Ty::Int), start, end);
                    } else {
                        return Err(Diag::new(
                            self.peek().span,
                            format!("expected 'in' or '=' after loop variable, found {}", self.peek_kind().describe()),
                        ));
                    }
                } else {
                    return self.for_c(span);
                }
            }
            TokenKind::Return => {
                let span = self.advance().span;
                let value = if self.at(&TokenKind::Semi) {
                    None
                } else {
                    Some(self.expr()?)
                };
                self.expect(TokenKind::Semi)?;
                Ok(Stmt::Return { value, span })
            }
            TokenKind::Break => {
                let span = self.advance().span;
                self.expect(TokenKind::Semi)?;
                Ok(Stmt::Break(span))
            }
            TokenKind::Continue => {
                let span = self.advance().span;
                self.expect(TokenKind::Semi)?;
                Ok(Stmt::Continue(span))
            }
            TokenKind::Switch => {
                let span = self.advance().span;
                self.expect(TokenKind::LParen)?;
                let value = self.expr()?;
                self.expect(TokenKind::RParen)?;
                self.expect(TokenKind::LBrace)?;
                let mut cases = Vec::new();
                let mut default = None;
                loop {
                    if self.eat(TokenKind::RBrace) {
                        break;
                    }
                    if self.eat(TokenKind::Case) {
                        let case_span = self.peek().span;
                        let mut patterns = Vec::new();
                        loop {
                            patterns.push(self.pattern()?);
                            if !self.eat(TokenKind::Comma) {
                                break;
                            }
                        }
                        self.expect(TokenKind::Colon)?;
                        let body = self.block()?;
                        cases.push(SwitchCase { patterns, body, span: case_span });
                    } else if self.eat(TokenKind::Default) {
                        self.expect(TokenKind::Colon)?;
                        let body = self.block()?;
                        if default.is_some() {
                            return Err(Diag::new(self.peek().span, "duplicate 'default' in switch"));
                        }
                        default = Some(body);
                    } else {
                        return Err(Diag::new(
                            self.peek().span,
                            format!("expected 'case' or 'default', found {}", self.peek_kind().describe()),
                        ));
                    }
                }
                Ok(Stmt::Switch { value, cases, default, span })
            }
            TokenKind::LBrace => Ok(Stmt::Block(self.block()?)),
            _ => {
                let expr = self.expr()?;
                let span = expr.span();
                if self.eat(TokenKind::Assign) {
                    let value = self.expr()?;
                    self.expect(TokenKind::Semi)?;
                    Ok(Stmt::Assign { target: expr, value, span })
                } else {
                    let compound_op = match self.peek_kind() {
                        TokenKind::PlusEq => Some(BinOp::Add),
                        TokenKind::MinusEq => Some(BinOp::Sub),
                        TokenKind::StarEq => Some(BinOp::Mul),
                        TokenKind::SlashEq => Some(BinOp::Div),
                        TokenKind::PercentEq => Some(BinOp::Mod),
                        _ => None,
                    };
                    if let Some(op) = compound_op {
                        self.advance();
                        let rhs = self.expr()?;
                        self.expect(TokenKind::Semi)?;
                        let value = Expr::Binary {
                            op,
                            lhs: Box::new(expr.clone()),
                            rhs: Box::new(rhs),
                            span,
                        };
                        Ok(Stmt::Assign { target: expr, value, span })
                    } else {
                        self.expect(TokenKind::Semi)?;
                        Ok(Stmt::Expr(expr, span))
                    }
                }
            }
        }
    }

    fn if_stmt(&mut self) -> Result<Stmt, Diag> {
        let span = self.expect(TokenKind::If)?.span;
        self.expect(TokenKind::LParen)?;
        let cond = self.expr()?;
        self.expect(TokenKind::RParen)?;
        let then = self.block()?;
        let els = if self.eat(TokenKind::Else) {
            if self.at(&TokenKind::If) {
                Some(Box::new(self.if_stmt()?))
            } else {
                Some(Box::new(Stmt::Block(self.block()?)))
            }
        } else {
            None
        };
        Ok(Stmt::If { cond, then, els, span })
    }

    fn for_c(&mut self, span: Span) -> Result<Stmt, Diag> {
        let init = if self.at(&TokenKind::Semi) {
            self.advance();
            None
        } else {
            Some(Box::new(self.stmt()?))
        };
        let cond = if self.at(&TokenKind::Semi) {
            Expr::Bool(true, self.peek().span)
        } else {
            self.expr()?
        };
        self.expect(TokenKind::Semi)?;
        let step = if self.at(&TokenKind::RParen) {
            None
        } else {
            Some(self.assign_like_stmt()?)
        };
        self.expect(TokenKind::RParen)?;
        let body = self.block()?;
        let mut loop_body = body.stmts;
        if let Some(s) = step {
            loop_body.push(s);
        }
        let body = Block { stmts: loop_body, span: body.span };
        let while_stmt = Stmt::While { cond, body, span };
        Ok(match init {
            Some(i) => Stmt::Block(Block { stmts: vec![*i, while_stmt], span }),
            None => while_stmt,
        })
    }

    fn assign_like_stmt(&mut self) -> Result<Stmt, Diag> {
        let expr = self.expr()?;
        let span = expr.span();
        if self.eat(TokenKind::Assign) {
            let value = self.expr()?;
            return Ok(Stmt::Assign { target: expr, value, span });
        }
        let compound_op = match self.peek_kind() {
            TokenKind::PlusEq => Some(BinOp::Add),
            TokenKind::MinusEq => Some(BinOp::Sub),
            TokenKind::StarEq => Some(BinOp::Mul),
            TokenKind::SlashEq => Some(BinOp::Div),
            TokenKind::PercentEq => Some(BinOp::Mod),
            _ => None,
        };
        if let Some(op) = compound_op {
            self.advance();
            let rhs = self.expr()?;
            let value = Expr::Binary { op, lhs: Box::new(expr.clone()), rhs: Box::new(rhs), span };
            return Ok(Stmt::Assign { target: expr, value, span });
        }
        Ok(Stmt::Expr(expr, span))
    }

    fn build_for_range(
        &mut self,
        span: Span,
        var_span: Span,
        name: String,
        ty: Option<Ty>,
        start: Expr,
        end: Expr,
    ) -> Result<Stmt, Diag> {
        let body = self.block()?;
        let var_decl = Stmt::Var {
            name: name.clone(),
            ty: ty.or(Some(Ty::Int)),
            init: start,
            mutable: true,
            span: var_span,
        };
        let cond = Expr::Binary {
            op: BinOp::Lt,
            lhs: Box::new(Expr::Var(name.clone(), span)),
            rhs: Box::new(end),
            span,
        };
        let step = Expr::Binary {
            op: BinOp::Add,
            lhs: Box::new(Expr::Var(name.clone(), span)),
            rhs: Box::new(Expr::Int(1, span)),
            span,
        };
        let mut body_stmts = body.stmts;
        body_stmts.push(Stmt::Assign {
            target: Expr::Var(name.clone(), span),
            value: step,
            span,
        });
        let body = Block { stmts: body_stmts, span: body.span };
        let while_stmt = Stmt::While { cond, body, span };
        Ok(Stmt::Block(Block { stmts: vec![var_decl, while_stmt], span }))
    }

    fn expr(&mut self) -> Result<Expr, Diag> {
        self.or_expr()
    }

    fn or_expr(&mut self) -> Result<Expr, Diag> {
        let mut lhs = self.and_expr()?;
        while self.at(&TokenKind::PipePipe) {
            let span = self.advance().span;
            let rhs = self.and_expr()?;
            lhs = Expr::Binary { op: BinOp::Or, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn and_expr(&mut self) -> Result<Expr, Diag> {
        let mut lhs = self.bitor_expr()?;
        while self.at(&TokenKind::AmpAmp) {
            let span = self.advance().span;
            let rhs = self.bitor_expr()?;
            lhs = Expr::Binary { op: BinOp::And, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn bitor_expr(&mut self) -> Result<Expr, Diag> {
        let mut lhs = self.bitxor_expr()?;
        while self.at(&TokenKind::Pipe) {
            let span = self.advance().span;
            let rhs = self.bitxor_expr()?;
            lhs = Expr::Binary { op: BinOp::BitOr, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn bitxor_expr(&mut self) -> Result<Expr, Diag> {
        let mut lhs = self.bitand_expr()?;
        while self.at(&TokenKind::Caret) {
            let span = self.advance().span;
            let rhs = self.bitand_expr()?;
            lhs = Expr::Binary { op: BinOp::BitXor, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn bitand_expr(&mut self) -> Result<Expr, Diag> {
        let mut lhs = self.equality()?;
        while self.at(&TokenKind::Amp) {
            let span = self.advance().span;
            let rhs = self.equality()?;
            lhs = Expr::Binary { op: BinOp::BitAnd, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn equality(&mut self) -> Result<Expr, Diag> {
        let mut lhs = self.comparison()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::Ne,
                _ => break,
            };
            let span = self.advance().span;
            let rhs = self.comparison()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn comparison(&mut self) -> Result<Expr, Diag> {
        let mut lhs = self.shift()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::LtEq => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::GtEq => BinOp::Ge,
                _ => break,
            };
            let span = self.advance().span;
            let rhs = self.shift()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn shift(&mut self) -> Result<Expr, Diag> {
        let mut lhs = self.additive()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Shl => BinOp::Shl,
                TokenKind::Shr => BinOp::Shr,
                _ => break,
            };
            let span = self.advance().span;
            let rhs = self.additive()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn additive(&mut self) -> Result<Expr, Diag> {
        let mut lhs = self.multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            let span = self.advance().span;
            let rhs = self.multiplicative()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn multiplicative(&mut self) -> Result<Expr, Diag> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            let span = self.advance().span;
            let rhs = self.unary()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, Diag> {
        if self.at(&TokenKind::Minus) {
            let span = self.advance().span;
            let operand = self.unary()?;
            return Ok(Expr::Unary { op: UnOp::Neg, operand: Box::new(operand), span });
        }
        if self.at(&TokenKind::Bang) {
            let span = self.advance().span;
            let operand = self.unary()?;
            return Ok(Expr::Unary { op: UnOp::Not, operand: Box::new(operand), span });
        }
        if self.at(&TokenKind::Tilde) {
            let span = self.advance().span;
            let operand = self.unary()?;
            return Ok(Expr::Unary { op: UnOp::BitNot, operand: Box::new(operand), span });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, Diag> {
        let mut expr = self.primary()?;
        loop {
            if self.at(&TokenKind::LParen) {
                let span = self.advance().span;
                let Expr::Var(name, _) = expr else {
                    return Err(Diag::new(span, "only named functions can be called"));
                };
                let mut args = Vec::new();
                if !self.at(&TokenKind::RParen) {
                    loop {
                        args.push(self.expr()?);
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen)?;
                expr = Expr::Call { name, args, span };
            } else if self.at(&TokenKind::LBracket) {
                let span = self.advance().span;
                let index = self.expr()?;
                self.expect(TokenKind::RBracket)?;
                expr = Expr::Index(Box::new(expr), Box::new(index), span);
            } else if self.at(&TokenKind::Dot) {
                self.advance();
                let (fname, fspan) = if let TokenKind::Int(v) = self.peek_kind().clone() {
                    let tok = self.advance();
                    (format!("{}", v), tok.span)
                } else {
                    self.expect_ident("field name")?
                };
                if self.at(&TokenKind::LParen) {
                    let call_span = self.advance().span;
                    let mut args = vec![expr];
                    if !self.at(&TokenKind::RParen) {
                        loop {
                            args.push(self.expr()?);
                            if !self.eat(TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    expr = Expr::Call { name: fname, args, span: call_span };
                } else {
                    expr = Expr::Field(Box::new(expr), fname, fspan);
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, Diag> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Int(v) => {
                self.advance();
                Ok(Expr::Int(*v, tok.span))
            }
            TokenKind::Float(v) => {
                self.advance();
                Ok(Expr::Float(*v, tok.span))
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr::Str(s.clone(), tok.span))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true, tok.span))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false, tok.span))
            }
            TokenKind::Nil => {
                self.advance();
                Ok(Expr::Nil(tok.span))
            }
            TokenKind::Ident(name) => {
                if self.peek_at(1) == &TokenKind::LBrace {
                    self.advance();
                    self.advance();
                    let mut fields = Vec::new();
                    while !self.at(&TokenKind::RBrace) {
                        let (fname, fspan) = self.expect_ident("field name")?;
                        self.expect(TokenKind::Colon)?;
                        let fval = self.expr()?;
                        fields.push((fname, fval, fspan));
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(TokenKind::RBrace)?;
                    return Ok(Expr::StructLit { name: name.clone(), fields, span: tok.span });
                }
                self.advance();
                Ok(Expr::Var(name.clone(), tok.span))
            }
            TokenKind::LParen => {
                self.advance();
                let first = self.expr()?;
                if self.eat(TokenKind::Comma) {
                    let mut items = vec![first];
                    loop {
                        items.push(self.expr()?);
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr::Tuple(items, tok.span))
                } else {
                    self.expect(TokenKind::RParen)?;
                    Ok(first)
                }
            }
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if !self.at(&TokenKind::RBracket) {
                    loop {
                        items.push(self.expr()?);
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::Array(items, tok.span))
            }
            other => Err(Diag::new(
                tok.span,
                format!("expected an expression, found {}", other.describe()),
            )),
        }
    }
}
