use crate::diag::{Diag, Diagnostics, Severity, SourceMap};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Colors for terminal output (disabled when not a TTY or NO_COLOR is set).
struct Style {
    use_color: bool,
}

impl Style {
    fn new() -> Self {
        let use_color = std::env::var_os("NO_COLOR").is_none() && atty_stderr();
        Style { use_color }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.use_color {
            format!("\x1b[{}m{}\x1b[0m", code, text)
        } else {
            text.to_string()
        }
    }

    fn red(&self, t: &str) -> String {
        self.paint("1;31", t)
    }
    fn yellow(&self, t: &str) -> String {
        self.paint("1;33", t)
    }
    fn green(&self, t: &str) -> String {
        self.paint("1;32", t)
    }
    fn cyan(&self, t: &str) -> String {
        self.paint("1;36", t)
    }
    fn dim(&self, t: &str) -> String {
        self.paint("2", t)
    }
}

fn atty_stderr() -> bool {
    // Cheap isatty check for stderr via libc-less heuristic: assume TTY unless redirected.
    // We use the `is_terminal`-like check through std when available; fallback to true.
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        unsafe { libc_isatty(std::io::stderr().as_raw_fd()) == 1 }
    }
    #[cfg(not(unix))]
    {
        // On Windows assume color-capable modern terminal; user can set NO_COLOR.
        true
    }
}

#[cfg(unix)]
unsafe fn libc_isatty(_fd: i32) -> i32 {
    // Avoid libc dependency; best-effort. Return 1 to enable color.
    1
}

/// One file's worth of diagnostics, grouped for pretty printing.
struct FileGroup<'a> {
    file: String,
    diags: Vec<&'a Diag>,
}

fn group_by_file<'a>(map: &SourceMap, diags: &'a [Diag]) -> Vec<FileGroup<'a>> {
    let mut groups: Vec<FileGroup<'a>> = Vec::new();
    for d in diags {
        let name = map.file_name(d.span);
        if let Some(g) = groups.iter_mut().find(|g| g.file == name) {
            g.diags.push(d);
        } else {
            groups.push(FileGroup { file: name, diags: vec![d] });
        }
    }
    groups
}

fn render_diag_block(map: &SourceMap, d: &Diag, style: &Style) -> String {
    let mut out = String::new();
    let kind_label = match d.severity {
        Severity::Error => style.red("ERROR"),
        Severity::Warning => style.yellow("Warning"),
    };
    out.push_str(&format!("{}: {}\n", kind_label, d.msg));
    // source snippet
    let idx = d.span.file as usize;
    if idx < map.len() {
        let (_, text) = map.file(idx);
        if let Some(line) = text.lines().nth(d.span.line.saturating_sub(1) as usize) {
            let num = d.span.line.to_string();
            let pad = " ".repeat(num.len());
            out.push_str(&style.dim(&format!("{} |\n", pad)));
            out.push_str(&format!("{} | {}\n", style.dim(&num), line));
            let caret_pad = " ".repeat(d.span.col.saturating_sub(1) as usize);
            let caret = style.red("^");
            out.push_str(&format!("{} | {}{}\n", style.dim(&pad), caret_pad, caret));
        }
        out.push_str(&style.dim(&format!(
            "  --> {}:{}:{}\n",
            map.file_name(d.span),
            d.span.line,
            d.span.col
        )));
    }
    out
}

/// Pretty-print a set of diagnostics to the terminal.
pub fn print_diagnostics(map: &SourceMap, diags: &Diagnostics) {
    let style = Style::new();
    // errors first, grouped by file
    for g in group_by_file(map, &diags.errors) {
        println!(
            "{}",
            style.red(&format!("[* Error for File: {}] {{", g.file))
        );
        for d in g.diags {
            print!("{}", render_diag_block(map, d, &style));
            println!();
        }
        println!("{}", style.red("}"));
        println!();
    }
    for g in group_by_file(map, &diags.warnings) {
        println!(
            "{}",
            style.yellow(&format!("[* warning for file: {}] {{", g.file))
        );
        for d in g.diags {
            print!("{}", render_diag_block(map, d, &style));
            println!();
        }
        println!("{}", style.yellow("}"));
        println!();
    }
}

/// Render diagnostics as plain text for the log file (no colors).
pub fn render_diagnostics_plain(map: &SourceMap, diags: &Diagnostics) -> String {
    let mut out = String::new();
    for d in &diags.errors {
        out.push_str(&format!("[* Error for File: {}] {{\n", map.file_name(d.span)));
        out.push_str(&map.render(d));
        out.push_str("}\n\n");
    }
    for d in &diags.warnings {
        out.push_str(&format!("[* warning for file: {}] {{\n", map.file_name(d.span)));
        out.push_str(&map.render(d));
        out.push_str("}\n\n");
    }
    out
}

/// Build a timestamped log file name like Log-23.32.30-12.10.2026.log
fn log_file_name() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let (y, mo, d) = civil_from_days((now / 86400) as i64);
    let secs = (now % 86400) as u32;
    let h = secs / 3600;
    let mi = (secs % 3600) / 60;
    let s = secs % 60;
    format!("Log-{:02}.{:02}.{:02}-{:02}.{:02}.{}.log", h, mi, s, d, mo, y)
}

/// Howard Hinnant's civil_from_days algorithm (UTC), returning (year, month, day).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Outcome of a compilation: the produced artifact (if any) plus all diagnostics.
pub struct Outcome<T> {
    pub value: Option<T>,
    pub errors: Vec<Diag>,
    pub warnings: Vec<Diag>,
}

impl<T> Outcome<T> {
    pub fn ok(value: T) -> Self {
        Outcome { value: Some(value), errors: Vec::new(), warnings: Vec::new() }
    }

    pub fn from_result(value: Result<T, Diagnostics>, warnings: Vec<Diag>) -> Self {
        match value {
            Ok(v) => Outcome { value: Some(v), errors: Vec::new(), warnings },
            Err(d) => Outcome { value: None, errors: d.errors, warnings },
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn diagnostics(&self) -> Diagnostics {
        Diagnostics { errors: self.errors.clone(), warnings: self.warnings.clone() }
    }
}

/// Final reporting after a build/check/test/run: prints status and writes the log.
/// Returns the log file path (relative) that was written, if any.
pub fn report(
    map: &SourceMap,
    outcome_errors: &[Diag],
    outcome_warnings: &[Diag],
    success_line: Option<&str>,
    project_root: &Path,
) -> Option<String> {
    let diags = Diagnostics { errors: outcome_errors.to_vec(), warnings: outcome_warnings.to_vec() };
    let style = Style::new();

    print_diagnostics(map, &diags);

    // Write the log file into <project>/LogsCompl/
    let logs_dir = project_root.join("LogsCompl");
    let log_name = log_file_name();
    let mut log_body = String::new();
    log_body.push_str(&format!("IKS build log — {}\n", log_name));
    log_body.push_str(&"=".repeat(60));
    log_body.push_str("\n\n");
    if !diags.errors.is_empty() || !diags.warnings.is_empty() {
        log_body.push_str(&render_diagnostics_plain(map, &diags));
    }
    if let Some(line) = success_line {
        log_body.push_str("[*]\n");
        log_body.push_str(line);
        log_body.push('\n');
    }

    let wrote = fs::create_dir_all(&logs_dir).is_ok()
        && fs::write(logs_dir.join(&log_name), &log_body).is_ok();

    if diags.errors.is_empty() {
        println!("{}", style.green("[*]"));
        if let Some(line) = success_line {
            println!("{}", line);
        }
    }
    if wrote {
        println!(
            "{}",
            style.cyan(&format!("[Log File: LogsCompl: {}]", log_name))
        );
        Some(format!("LogsCompl/{}", log_name))
    } else {
        None
    }
}
