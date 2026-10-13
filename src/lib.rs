pub mod ast;
pub mod codegen;
pub mod config;
pub mod diag;
pub mod lexer;
pub mod parser;
pub mod report;
pub mod wasm;

use codegen::BuildMode;
use diag::{Diag, Diagnostics, SourceMap};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub struct Compiler {
    map: SourceMap,
    warnings: Vec<Diag>,
}

fn path_key(p: &Path) -> String {
    match fs::canonicalize(p) {
        Ok(abs) => abs.to_string_lossy().to_string(),
        Err(_) => p.to_string_lossy().to_string(),
    }
}

impl Compiler {
    pub fn new() -> Self {
        Compiler { map: SourceMap::new(), warnings: Vec::new() }
    }

    pub fn add_file(&mut self, name: impl Into<String>, text: impl Into<String>) -> u32 {
        self.map.add(name, text)
    }

    pub fn compile(&mut self) -> Result<String, Diagnostics> {
        let (program, warnings) = self.build_program()?;
        self.warnings = warnings;
        match codegen::generate(&program, BuildMode::Normal) {
            Ok((code, errors, w)) => {
                self.warnings.extend(w);
                if !errors.is_empty() {
                    return Err(Diagnostics {
                        errors,
                        warnings: std::mem::take(&mut self.warnings),
                    });
                }
                Ok(code)
            }
            Err(errors) => Err(Diagnostics {
                errors,
                warnings: std::mem::take(&mut self.warnings),
            }),
        }
    }

    pub fn compile_test(&mut self) -> Result<String, Diagnostics> {
        let (program, warnings) = self.build_program()?;
        self.warnings = warnings;
        match codegen::generate(&program, BuildMode::Test) {
            Ok((code, errors, w)) => {
                self.warnings.extend(w);
                if !errors.is_empty() {
                    return Err(Diagnostics {
                        errors,
                        warnings: std::mem::take(&mut self.warnings),
                    });
                }
                Ok(code)
            }
            Err(errors) => Err(Diagnostics {
                errors,
                warnings: std::mem::take(&mut self.warnings),
            }),
        }
    }

    pub fn compile_wasm(&mut self) -> Result<Vec<u8>, Diagnostics> {
        let (program, warnings) = self.build_program()?;
        self.warnings = warnings;
        match wasm::generate(&program) {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(Diagnostics {
                errors: vec![e],
                warnings: std::mem::take(&mut self.warnings),
            }),
        }
    }

    /// Warnings collected during the last compile/check.
    pub fn warnings(&self) -> &[Diag] {
        &self.warnings
    }

    pub fn source_map(&self) -> &SourceMap {
        &self.map
    }

    fn build_program(&mut self) -> Result<(ast::Program, Vec<Diag>), Diagnostics> {
        let mut merged = ast::Program::default();
        let mut visited: HashSet<String> = HashSet::new();
        let mut warnings: Vec<Diag> = Vec::new();
        let mut errors: Vec<Diag> = Vec::new();
        let nroots = self.map.len();
        for i in 0..nroots {
            let (name, _) = self.map.file(i);
            visited.insert(path_key(Path::new(name)));
        }
        let mut queue: Vec<u32> = (0..nroots as u32).collect();
        let mut qi = 0;
        while qi < queue.len() {
            let fi = queue[qi];
            qi += 1;
            let (name, text) = self.map.file(fi as usize);
            let name = name.to_string();
            let text = text.to_string();
            let tokens = match lexer::Lexer::new(fi, &text).tokenize() {
                Ok(t) => t,
                Err(mut d) => {
                    errors.append(&mut d.errors);
                    continue;
                }
            };
            let part = match parser::parse(tokens) {
                Ok(p) => p,
                Err(mut d) => {
                    errors.append(&mut d.errors);
                    warnings.append(&mut d.warnings);
                    continue;
                }
            };
            warnings.append(&mut part.warnings.clone());
            for imp in &part.imports {
                let base = Path::new(&name).parent().map(|p| p.to_path_buf()).unwrap_or_default();
                let full = base.join(&imp.path);
                let key = path_key(&full);
                if visited.contains(&key) {
                    // Duplicate import (same file already pulled in).
                    warnings.push(Diag::warning(
                        imp.span,
                        format!("duplicate import of '{}' (already loaded)", imp.path),
                    ));
                    continue;
                }
                let content = match fs::read_to_string(&full) {
                    Ok(c) => c,
                    Err(_) => {
                        errors.push(Diag::new(
                            imp.span,
                            format!("cannot read imported file '{}'", full.display()),
                        ));
                        continue;
                    }
                };
                visited.insert(key);
                let display = full.to_string_lossy().to_string();
                let new_idx = self.map.add(&display, content);
                queue.push(new_idx);
            }
            merged.funcs.extend(part.funcs);
            merged.structs.extend(part.structs);
            merged.enums.extend(part.enums);
        }
        if !errors.is_empty() {
            return Err(Diagnostics { errors, warnings });
        }
        Ok((merged, warnings))
    }

    pub fn render(&self, d: &Diag) -> String {
        self.map.render(d)
    }

    pub fn render_all(&self, d: &Diagnostics) -> String {
        let mut out = String::new();
        for e in &d.errors {
            out.push_str(&self.map.render(e));
            out.push('\n');
        }
        for w in &d.warnings {
            out.push_str(&self.map.render(w));
            out.push('\n');
        }
        out
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
