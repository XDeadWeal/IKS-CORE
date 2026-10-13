use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub version: String,
    pub files: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            name: "iks_project".to_string(),
            version: "0.1.0".to_string(),
            files: vec!["src/main.ik".to_string()],
        }
    }
}

pub fn load(path: &Path) -> Result<Config, String> {
    let text = fs::read_to_string(path).map_err(|_| {
        format!(
            "cannot read '{}'; run 'ik create' to scaffold a project first",
            path.display()
        )
    })?;
    parse(&text)
}

fn strip_comment(line: &str) -> String {
    let mut out = String::new();
    let mut in_str = false;
    let mut prev_escape = false;
    for c in line.chars() {
        match c {
            '"' if !prev_escape => in_str = !in_str,
            '#' if !in_str => break,
            _ => {}
        }
        prev_escape = c == '\\' && in_str;
        out.push(c);
    }
    out
}

fn parse(text: &str) -> Result<Config, String> {
    let mut cfg = Config::default();
    let mut files: Option<Vec<String>> = None;
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;

    let lines: Vec<String> = text.lines().map(|l| strip_comment(l).trim().to_string()).collect();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if line.is_empty() || line.starts_with('[') {
            i += 1;
            continue;
        }
        let Some(eq) = line.find('=') else {
            return Err(format!("invalid config line: '{}'", line));
        };
        let key = line[..eq].trim().to_string();
        let mut value = line[eq + 1..].trim().to_string();
        if value.starts_with('[') && !value.contains(']') {
            while i + 1 < lines.len() && !value.contains(']') {
                i += 1;
                value.push(' ');
                value.push_str(&lines[i]);
            }
        }
        match key.as_str() {
            "name" => name = Some(parse_string(&value, "name")?),
            "version" => version = Some(parse_string(&value, "version")?),
            "files" => files = Some(parse_string_array(&value)?),
            _ => {}
        }
        i += 1;
    }

    if let Some(n) = name {
        cfg.name = n;
    }
    if let Some(v) = version {
        cfg.version = v;
    }
    if let Some(f) = files {
        if f.is_empty() {
            return Err("config error: 'files' list is empty".to_string());
        }
        cfg.files = f;
    }
    Ok(cfg)
}

fn parse_string(value: &str, key: &str) -> Result<String, String> {
    let v = value.trim();
    let v = v.strip_prefix('"').and_then(|s| s.strip_suffix('"')).ok_or_else(|| {
        format!("config error: '{}' must be a quoted string, e.g. {} = \"value\"", key, key)
    })?;
    Ok(v.to_string())
}

fn parse_string_array(value: &str) -> Result<Vec<String>, String> {
    let v = value.trim();
    let inner = v
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| "config error: 'files' must be a list of strings, e.g. files = [\"src/main.ik\"]".to_string())?;
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    for c in inner.chars() {
        match c {
            '"' => in_str = !in_str,
            ',' if !in_str => {
                let item = cur.trim().to_string();
                if !item.is_empty() {
                    out.push(item);
                }
                cur.clear();
            }
            other if !in_str && other.is_whitespace() => {}
            other => cur.push(other),
        }
    }
    let item = cur.trim().to_string();
    if !item.is_empty() {
        out.push(item);
    }
    Ok(out)
}
