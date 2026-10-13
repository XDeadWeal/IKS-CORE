#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub file: u32,
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub fn new(file: u32, line: u32, col: u32) -> Self {
        Span { file, line, col }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Diag {
    pub span: Span,
    pub msg: String,
    pub severity: Severity,
}

impl Diag {
    pub fn new(span: Span, msg: impl Into<String>) -> Self {
        Diag { span, msg: msg.into(), severity: Severity::Error }
    }

    pub fn warning(span: Span, msg: impl Into<String>) -> Self {
        Diag { span, msg: msg.into(), severity: Severity::Warning }
    }
}

/// A bag of errors and warnings collected during compilation.
#[derive(Debug, Default, Clone)]
pub struct Diagnostics {
    pub errors: Vec<Diag>,
    pub warnings: Vec<Diag>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Diagnostics { errors: Vec::new(), warnings: Vec::new() }
    }

    pub fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.errors.push(Diag::new(span, msg));
    }

    pub fn push_error(&mut self, d: Diag) {
        self.errors.push(d);
    }

    pub fn warn(&mut self, span: Span, msg: impl Into<String>) {
        self.warnings.push(Diag::warning(span, msg));
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn merge(&mut self, other: Diagnostics) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }

    pub fn into_result<T>(self, value: T) -> Result<T, Diagnostics> {
        if self.has_errors() {
            Err(self)
        } else {
            Ok(value)
        }
    }
}

pub struct SourceMap {
    files: Vec<(String, String)>,
}

impl SourceMap {
    pub fn new() -> Self {
        SourceMap { files: Vec::new() }
    }

    pub fn add(&mut self, name: impl Into<String>, text: impl Into<String>) -> u32 {
        self.files.push((name.into(), text.into()));
        (self.files.len() - 1) as u32
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn file(&self, i: usize) -> (&str, &str) {
        (&self.files[i].0, &self.files[i].1)
    }

    /// Render a single diagnostic to plain text (used for the log file).
    pub fn render(&self, d: &Diag) -> String {
        let kind = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let mut out = format!("{}: {}\n", kind, d.msg);
        let idx = d.span.file as usize;
        if idx >= self.files.len() {
            out.push_str("  --> <unknown>\n");
            return out;
        }
        let (name, text) = &self.files[idx];
        out.push_str(&format!("  --> {}:{}:{}\n", name, d.span.line, d.span.col));
        if let Some(line) = text.lines().nth(d.span.line.saturating_sub(1) as usize) {
            let num = d.span.line.to_string();
            let pad = " ".repeat(num.len());
            out.push_str(&format!("{} |\n", pad));
            out.push_str(&format!("{} | {}\n", num, line));
            out.push_str(&format!("{} | {}^\n", pad, " ".repeat(d.span.col.saturating_sub(1) as usize)));
        }
        out
    }

    /// File name for a given span (for grouping in the pretty output).
    pub fn file_name(&self, span: Span) -> String {
        let idx = span.file as usize;
        if idx < self.files.len() {
            self.files[idx].0.clone()
        } else {
            "<unknown>".to_string()
        }
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}
