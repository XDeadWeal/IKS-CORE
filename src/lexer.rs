use crate::ast::{Token, TokenKind};
use crate::diag::{Diag, Diagnostics, Span};

pub struct Lexer {
    file: u32,
    chars: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    pub fn new(file: u32, src: &str) -> Self {
        Lexer {
            file,
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, Diagnostics> {
        let mut tokens = Vec::new();
        let mut errors: Vec<Diag> = Vec::new();
        loop {
            match self.next_token() {
                Ok(tok) => {
                    let is_eof = tok.kind == TokenKind::Eof;
                    tokens.push(tok);
                    if is_eof {
                        if errors.is_empty() {
                            return Ok(tokens);
                        }
                        return Err(Diagnostics { errors, warnings: Vec::new() });
                    }
                }
                Err(e) => {
                    // Recover: skip one char and continue to find more lex errors.
                    errors.push(e);
                    if self.pos >= self.chars.len() {
                        tokens.push(Token { kind: TokenKind::Eof, span: self.span() });
                        return Err(Diagnostics { errors, warnings: Vec::new() });
                    }
                    self.bump();
                }
            }
        }
    }

    fn span(&self) -> Span {
        Span::new(self.file, self.line, self.col)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, off: usize) -> Option<char> {
        self.chars.get(self.pos + off).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn skip_trivia(&mut self) -> Result<(), Diag> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    let start = self.span();
                    self.bump();
                    self.bump();
                    loop {
                        match self.peek() {
                            None => {
                                return Err(Diag::new(start, "unterminated block comment"));
                            }
                            Some('*') if self.peek_at(1) == Some('/') => {
                                self.bump();
                                self.bump();
                                break;
                            }
                            _ => {
                                self.bump();
                            }
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, Diag> {
        self.skip_trivia()?;
        let span = self.span();
        let Some(ch) = self.peek() else {
            return Ok(Token { kind: TokenKind::Eof, span });
        };

        if ch.is_ascii_alphabetic() || ch == '_' {
            let mut word = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    word.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
            let kind = match word.as_str() {
                "fn" => TokenKind::Fn,
                "var" => TokenKind::Var,
                "let" => TokenKind::Let,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "while" => TokenKind::While,
                "for" => TokenKind::For,
                "in" => TokenKind::In,
                "return" => TokenKind::Return,
                "break" => TokenKind::Break,
                "continue" => TokenKind::Continue,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                "struct" => TokenKind::Struct,
                "enum" => TokenKind::Enum,
                "extern" => TokenKind::Extern,
                "import" => TokenKind::Import,
                "nil" => TokenKind::Nil,
                "switch" => TokenKind::Switch,
                "case" => TokenKind::Case,
                "default" => TokenKind::Default,
                _ => TokenKind::Ident(word),
            };
            return Ok(Token { kind, span });
        }

        if ch.is_ascii_digit() {
            if ch == '0' && matches!(self.peek_at(1), Some('x') | Some('X')) {
                self.bump();
                self.bump();
                let mut text = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_hexdigit() {
                        text.push(c);
                        self.bump();
                    } else {
                        break;
                    }
                }
                if text.is_empty() {
                    return Err(Diag::new(span, "invalid hex literal"));
                }
                let value = i64::from_str_radix(&text, 16).map_err(|_| {
                    Diag::new(span, format!("hex literal '0x{}' is out of range", text))
                })?;
                return Ok(Token { kind: TokenKind::Int(value), span });
            }
            let mut text = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    text.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
            if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
                text.push('.');
                self.bump();
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        text.push(c);
                        self.bump();
                    } else {
                        break;
                    }
                }
                let value = text
                    .parse::<f64>()
                    .map_err(|_| Diag::new(span, format!("invalid float literal '{}'", text)))?;
                return Ok(Token { kind: TokenKind::Float(value), span });
            }
            let value = text.parse::<i64>().map_err(|_| {
                Diag::new(span, format!("integer literal '{}' is out of range", text))
            })?;
            return Ok(Token { kind: TokenKind::Int(value), span });
        }

        if ch == '"' {
            self.bump();
            let mut text = String::new();
            loop {
                match self.peek() {
                    None | Some('\n') => {
                        return Err(Diag::new(span, "unterminated string literal"));
                    }
                    Some('"') => {
                        self.bump();
                        break;
                    }
                    Some('\\') => {
                        self.bump();
                        let esc_span = self.span();
                        let Some(esc) = self.bump() else {
                            return Err(Diag::new(esc_span, "unterminated string escape"));
                        };
                        match esc {
                            'n' => text.push('\n'),
                            't' => text.push('\t'),
                            'r' => text.push('\r'),
                            '0' => text.push('\0'),
                            '\\' => text.push('\\'),
                            '"' => text.push('"'),
                            other => {
                                return Err(Diag::new(
                                    esc_span,
                                    format!("unknown escape sequence '\\{}'", other),
                                ));
                            }
                        }
                    }
                    Some(c) => {
                        text.push(c);
                        self.bump();
                    }
                }
            }
            return Ok(Token { kind: TokenKind::Str(text), span });
        }

        let kind = match ch {
            '+' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                self.bump();
                if self.peek() == Some('>') {
                    self.bump();
                    TokenKind::Arrow
                } else if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            '/' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }
            '%' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::PercentEq
                } else {
                    TokenKind::Percent
                }
            }
            '=' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::EqEq
                } else {
                    TokenKind::Assign
                }
            }
            '!' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }
            '<' => {
                self.bump();
                if self.peek() == Some('<') {
                    self.bump();
                    TokenKind::Shl
                } else if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                self.bump();
                if self.peek() == Some('>') {
                    self.bump();
                    TokenKind::Shr
                } else if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                self.bump();
                if self.peek() == Some('&') {
                    self.bump();
                    TokenKind::AmpAmp
                } else {
                    TokenKind::Amp
                }
            }
            '|' => {
                self.bump();
                if self.peek() == Some('|') {
                    self.bump();
                    TokenKind::PipePipe
                } else {
                    TokenKind::Pipe
                }
            }
            '^' => {
                self.bump();
                TokenKind::Caret
            }
            '~' => {
                self.bump();
                TokenKind::Tilde
            }
            ':' => {
                self.bump();
                TokenKind::Colon
            }
            ',' => {
                self.bump();
                TokenKind::Comma
            }
            ';' => {
                self.bump();
                TokenKind::Semi
            }
            '.' => {
                self.bump();
                if self.peek() == Some('.') {
                    self.bump();
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }
            '(' => {
                self.bump();
                TokenKind::LParen
            }
            ')' => {
                self.bump();
                TokenKind::RParen
            }
            '[' => {
                self.bump();
                TokenKind::LBracket
            }
            ']' => {
                self.bump();
                TokenKind::RBracket
            }
            '{' => {
                self.bump();
                TokenKind::LBrace
            }
            '}' => {
                self.bump();
                TokenKind::RBrace
            }
            other => {
                return Err(Diag::new(span, format!("unexpected character '{}'", other)));
            }
        };
        Ok(Token { kind, span })
    }
}
