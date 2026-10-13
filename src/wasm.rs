use crate::ast::*;
use crate::diag::{Diag, Span};
use std::collections::{HashMap, HashSet};

const WASM_MAGIC: [u8; 8] = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

const VT_I32: u8 = 0x7F;
const VT_I64: u8 = 0x7E;
const VT_F64: u8 = 0x7C;

const IMP_PRINT_I: u32 = 0;
const IMP_PRINT_F: u32 = 1;
const IMP_PRINT_B: u32 = 2;
const IMP_PRINT_S: u32 = 3;
const IMP_PRINT_SP: u32 = 4;
const IMP_PRINT_NL: u32 = 5;
const IMP_EXIT: u32 = 6;
const IMP_SIN: u32 = 7;
const IMP_COS: u32 = 8;
const IMP_POW: u32 = 9;
const IMP_RAND: u32 = 10;
const IMP_GFX_INIT: u32 = 11;
const IMP_GFX_CLEAR: u32 = 12;
const IMP_GFX_COLOR: u32 = 13;
const IMP_GFX_BEGIN: u32 = 14;
const IMP_GFX_VERTEX: u32 = 15;
const IMP_GFX_END: u32 = 16;
const IMP_GFX_SWAP: u32 = 17;
const IMP_GFX_WIDTH: u32 = 18;
const IMP_GFX_HEIGHT: u32 = 19;
const IMP_GFX_RECT: u32 = 20;
const IMP_GFX_CIRCLE: u32 = 21;
const IMP_GFX_LINE: u32 = 22;
const IMP_GFX_TITLE: u32 = 23;
const IMP_GPU_INIT: u32 = 24;
const IMP_GPU_CLEAR: u32 = 25;
const IMP_GPU_COLOR: u32 = 26;
const IMP_GPU_BEGIN: u32 = 27;
const IMP_GPU_VERTEX: u32 = 28;
const IMP_GPU_END: u32 = 29;
const IMP_GPU_SWAP: u32 = 30;
const IMP_GPU_WIDTH: u32 = 31;
const IMP_GPU_HEIGHT: u32 = 32;
const IMP_GPU_RECT: u32 = 33;
const IMP_GPU_CIRCLE: u32 = 34;
const IMP_GPU_LINE: u32 = 35;
const IMP_GPU_TITLE: u32 = 36;
const NIMPORTS: u32 = 37;

const IMPORTS: &[(&str, &[u8], &[u8])] = &[
    ("print_i", &[VT_I64], &[]),
    ("print_f", &[VT_F64], &[]),
    ("print_b", &[VT_I32], &[]),
    ("print_s", &[VT_I32, VT_I32], &[]),
    ("print_sp", &[], &[]),
    ("print_nl", &[], &[]),
    ("exit", &[VT_I32], &[]),
    ("sin", &[VT_F64], &[VT_F64]),
    ("cos", &[VT_F64], &[VT_F64]),
    ("pow", &[VT_F64, VT_F64], &[VT_F64]),
    ("rand", &[], &[VT_F64]),
    ("gfx_init", &[VT_I32, VT_I32, VT_I32, VT_I32], &[]),
    ("gfx_clear", &[VT_I32, VT_I32, VT_I32], &[]),
    ("gfx_color", &[VT_I32, VT_I32, VT_I32], &[]),
    ("gfx_begin", &[VT_I32], &[]),
    ("gfx_vertex", &[VT_F64, VT_F64, VT_F64], &[]),
    ("gfx_end", &[], &[]),
    ("gfx_swap", &[], &[VT_I32]),
    ("gfx_width", &[], &[VT_I32]),
    ("gfx_height", &[], &[VT_I32]),
    ("gfx_rect", &[VT_F64, VT_F64, VT_F64, VT_F64], &[]),
    ("gfx_circle", &[VT_F64, VT_F64, VT_F64, VT_I32], &[]),
    ("gfx_line", &[VT_F64, VT_F64, VT_F64, VT_F64], &[]),
    ("gfx_title", &[VT_I32, VT_I32], &[]),
    ("gpu_init", &[VT_I32, VT_I32, VT_I32, VT_I32], &[]),
    ("gpu_clear", &[VT_I32, VT_I32, VT_I32], &[]),
    ("gpu_color", &[VT_I32, VT_I32, VT_I32], &[]),
    ("gpu_begin", &[VT_I32], &[]),
    ("gpu_vertex", &[VT_F64, VT_F64, VT_F64], &[]),
    ("gpu_end", &[], &[]),
    ("gpu_swap", &[], &[VT_I32]),
    ("gpu_width", &[], &[VT_I32]),
    ("gpu_height", &[], &[VT_I32]),
    ("gpu_rect", &[VT_F64, VT_F64, VT_F64, VT_F64], &[]),
    ("gpu_circle", &[VT_F64, VT_F64, VT_F64, VT_I32], &[]),
    ("gpu_line", &[VT_F64, VT_F64, VT_F64, VT_F64], &[]),
    ("gpu_title", &[VT_I32, VT_I32], &[]),
];

const RESERVED_WASM: &[&str] = &[
    "print", "len", "str", "int", "float", "input", "push", "rand", "exit", "sleep", "assert", "beep",
    "sqrt", "abs", "floor",
    "gfx_open", "gfx_poll", "gfx_present", "gfx_clear", "gfx_pixel", "gfx_add", "gfx_line",
    "gfx_width", "gfx_height", "gfx_time", "gfx_mouse_x", "gfx_mouse_y", "gfx_key_down",
    "touch_x", "touch_y",
    "sprite_load", "sprite_draw", "sprite_width", "sprite_height", "sprite_unload",
    "cpu_load", "gpu_load",
    "npu_available", "npu_name", "npu_count", "npu_tops", "npu_load",
    "d3d_init", "d3d_clear", "d3d_draw", "d3d_present",
    "d3d_color", "d3d_begin", "d3d_vertex", "d3d_end",
    "d3d_title", "d3d_width", "d3d_height", "d3d_rect", "d3d_circle", "d3d_line",
    "dx12_init", "dx12_clear", "dx12_color", "dx12_begin", "dx12_vertex", "dx12_end", "dx12_swap",
    "dx12_title", "dx12_width", "dx12_height", "dx12_rect", "dx12_circle", "dx12_line",
    "d3d9_init", "d3d9_clear", "d3d9_color", "d3d9_begin", "d3d9_vertex", "d3d9_end", "d3d9_swap",
    "d3d9_title", "d3d9_width", "d3d9_height", "d3d9_rect", "d3d9_circle", "d3d9_line",
    "dx10_init", "dx10_clear", "dx10_color", "dx10_begin", "dx10_vertex", "dx10_end", "dx10_swap",
    "dx10_title", "dx10_width", "dx10_height", "dx10_rect", "dx10_circle", "dx10_line",
    "vk_init", "vk_clear", "vk_color", "vk_begin", "vk_vertex", "vk_end", "vk_swap",
    "vk_title", "vk_width", "vk_height", "vk_rect", "vk_circle", "vk_line",
];

fn uleb(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

fn sleb(out: &mut Vec<u8>, mut v: i64) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        let sign_bit = byte & 0x40 != 0;
        if (v == 0 && !sign_bit) || (v == -1 && sign_bit) {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

fn section(out: &mut Vec<u8>, id: u8, payload: &[u8]) {
    out.push(id);
    uleb(out, payload.len() as u64);
    out.extend_from_slice(payload);
}

fn valtype(t: &Ty, span: Span) -> Result<u8, Diag> {
    match t {
        Ty::Int => Ok(VT_I64),
        Ty::Float => Ok(VT_F64),
        Ty::Bool => Ok(VT_I32),
        Ty::Void => Err(Diag::new(span, "value of type 'void' cannot be used here")),
        Ty::Str => Err(Diag::new(
            span,
            "string values are only supported as direct 'print'/'assert' arguments on the web target",
        )),
        Ty::Ptr => Err(Diag::new(span, "'ptr' is not supported on the web target yet")),
        Ty::Array(_) => Err(Diag::new(span, "arrays are not supported on the web target yet")),
        Ty::Named(_) => Err(Diag::new(span, "structs are not supported on the web target yet")),
        Ty::HashMap => Err(Diag::new(span, "maps are not supported on the web target yet")),
        Ty::Tuple(_) => Err(Diag::new(span, "tuples are not supported on the web target yet")),
        Ty::Vec2 | Ty::Vec3 | Ty::Vec4 => Err(Diag::new(
            span,
            "vector types are not supported on the web target yet",
        )),
    }
}

fn vt_byte(t: &Ty) -> Option<u8> {
    match t {
        Ty::Int => Some(VT_I64),
        Ty::Float => Some(VT_F64),
        Ty::Bool => Some(VT_I32),
        _ => None,
    }
}

#[derive(Clone)]
struct FuncSig {
    params: Vec<Ty>,
    ret: Ty,
}

#[derive(Clone)]
struct VarInfo {
    ty: Ty,
    mutable: bool,
    idx: u32,
}

enum Frame {
    Block,
    Loop,
    If,
}

struct WasmGen {
    types: Vec<(Vec<u8>, Vec<u8>)>,
    type_map: HashMap<(Vec<u8>, Vec<u8>), u32>,
    funcs: Vec<FuncSig>,
    func_typeidx: Vec<u32>,
    name_to_idx: HashMap<String, usize>,
    body: Vec<u8>,
    locals: Vec<u8>,
    scopes: Vec<HashMap<String, VarInfo>>,
    frames: Vec<Frame>,
    nparams: u32,
    ret: Ty,
    str_data: Vec<u8>,
    str_map: HashMap<String, u32>,
}

impl WasmGen {
    fn new() -> Self {
        WasmGen {
            types: Vec::new(),
            type_map: HashMap::new(),
            funcs: Vec::new(),
            func_typeidx: Vec::new(),
            name_to_idx: HashMap::new(),
            body: Vec::new(),
            locals: Vec::new(),
            scopes: Vec::new(),
            frames: Vec::new(),
            nparams: 0,
            ret: Ty::Void,
            str_data: Vec::new(),
            str_map: HashMap::new(),
        }
    }

    fn type_idx(&mut self, params: &[u8], results: &[u8]) -> u32 {
        let key = (params.to_vec(), results.to_vec());
        if let Some(i) = self.type_map.get(&key) {
            return *i;
        }
        let i = self.types.len() as u32;
        self.types.push(key.clone());
        self.type_map.insert(key, i);
        i
    }

    fn intern(&mut self, s: &str) -> u32 {
        if let Some(off) = self.str_map.get(s) {
            return *off;
        }
        let off = self.str_data.len() as u32;
        self.str_data.extend_from_slice(s.as_bytes());
        self.str_map.insert(s.to_string(), off);
        off
    }

    fn lookup(&self, name: &str) -> Option<&VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        None
    }

    fn call_import(&mut self, idx: u32) {
        self.body.push(0x10);
        uleb(&mut self.body, idx as u64);
    }

    fn ty_of(&self, e: &Expr) -> Result<Ty, Diag> {
        match e {
            Expr::Int(_, _) => Ok(Ty::Int),
            Expr::Float(_, _) => Ok(Ty::Float),
            Expr::Str(_, _) => Ok(Ty::Str),
            Expr::Bool(_, _) => Ok(Ty::Bool),
            Expr::Nil(span) => Err(Diag::new(*span, "'nil' is not supported on the web target yet")),
            Expr::Var(name, span) => match self.lookup(name) {
                Some(v) => Ok(v.ty.clone()),
                None => Err(Diag::new(*span, format!("undefined variable '{}'", name))),
            },
            Expr::Array(_, span) => Err(Diag::new(*span, "arrays are not supported on the web target yet")),
            Expr::Tuple(_, span) => Err(Diag::new(*span, "tuples are not supported on the web target yet")),
            Expr::Field(_, _, span) => Err(Diag::new(*span, "structs are not supported on the web target yet")),
            Expr::StructLit { span, .. } => Err(Diag::new(*span, "structs are not supported on the web target yet")),
            Expr::Index(_, _, span) => Err(Diag::new(*span, "arrays are not supported on the web target yet")),
            Expr::Unary { op, operand, span } => match op {
                UnOp::Not => Ok(Ty::Bool),
                UnOp::Neg => {
                    let t = self.ty_of(operand)?;
                    if !t.is_numeric() {
                        return Err(Diag::new(*span, format!("cannot negate a value of type '{}'", t)));
                    }
                    Ok(t)
                }
                UnOp::BitNot => {
                    let t = self.ty_of(operand)?;
                    if t != Ty::Int {
                        return Err(Diag::new(*span, format!("'~' expects int, found '{}'", t)));
                    }
                    Ok(Ty::Int)
                }
            },
            Expr::Binary { op, lhs, rhs, span } => {
                let lt = self.ty_of(lhs)?;
                let rt = self.ty_of(rhs)?;
                match op {
                    BinOp::And | BinOp::Or => Ok(Ty::Bool),
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        unify_cmp(&lt, &rt, *span)?;
                        Ok(Ty::Bool)
                    }
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                        if lt != Ty::Int || rt != Ty::Int {
                            return Err(Diag::new(
                                *span,
                                format!("bitwise operator expects int operands, found '{}' and '{}'", lt, rt),
                            ));
                        }
                        Ok(Ty::Int)
                    }
                    _ => unify_num(&lt, &rt, *span),
                }
            }
            Expr::Call { name, args, span } => self.call_ty(name, args, *span),
        }
    }

    fn call_ty(&self, name: &str, args: &[Expr], span: Span) -> Result<Ty, Diag> {
        match name {
            "print" => Err(Diag::new(
                span,
                "'print' returns no value; call it as a statement, e.g. print(x);",
            )),
            "assert" => Ok(Ty::Void),
            "exit" => Ok(Ty::Void),
            "vec2" | "vec3" | "vec4" | "dot" | "cross" | "length" | "normalize" | "distance"
            | "rotate" => Err(Diag::new(
                span,
                format!("'{}' (vector math) is not supported on the web target yet", name),
            )),
            "sleep" | "input" | "push" | "len" | "str" | "beep" => Err(Diag::new(
                span,
                format!("'{}' is not supported on the web target yet", name),
            )),
            "int" => {
                if args.len() != 1 {
                    return Err(Diag::new(span, "'int' expects 1 argument"));
                }
                let t = self.ty_of(&args[0])?;
                match t {
                    Ty::Int | Ty::Float | Ty::Bool => Ok(Ty::Int),
                    other => Err(Diag::new(
                        args[0].span(),
                        format!("'int' expects int, float or bool, found '{}'", other),
                    )),
                }
            }
            "float" => {
                if args.len() != 1 {
                    return Err(Diag::new(span, "'float' expects 1 argument"));
                }
                let t = self.ty_of(&args[0])?;
                match t {
                    Ty::Int | Ty::Float | Ty::Bool => Ok(Ty::Float),
                    other => Err(Diag::new(
                        args[0].span(),
                        format!("'float' expects int, float or bool, found '{}'", other),
                    )),
                }
            }
            "sin" | "cos" | "sqrt" | "abs" | "floor" => {
                if args.len() != 1 {
                    return Err(Diag::new(span, format!("'{}' expects 1 argument", name)));
                }
                let t = self.ty_of(&args[0])?;
                if !matches!(t, Ty::Int | Ty::Float) {
                    return Err(Diag::new(
                        args[0].span(),
                        format!("'{}' expects a number, found '{}'", name, t),
                    ));
                }
                Ok(Ty::Float)
            }
            "pow" => {
                if args.len() != 2 {
                    return Err(Diag::new(span, "'pow' expects 2 arguments"));
                }
                Ok(Ty::Float)
            }
            "rand" => {
                if args.len() != 0 {
                    return Err(Diag::new(span, "'rand' expects 0 arguments"));
                }
                Ok(Ty::Float)
            }
            "gl_init" | "webgpu_init" => {
                if args.len() != 3 {
                    return Err(Diag::new(span, format!("'{}' expects 3 arguments", name)));
                }
                Ok(Ty::Void)
            }
            "gl_clear" | "webgpu_clear" => {
                if args.len() != 3 {
                    return Err(Diag::new(span, format!("'{}' expects 3 arguments", name)));
                }
                Ok(Ty::Void)
            }
            "gl_color" | "webgpu_color" => {
                if args.len() != 4 {
                    return Err(Diag::new(span, format!("'{}' expects 4 arguments", name)));
                }
                Ok(Ty::Void)
            }
            "gl_begin" | "webgpu_begin" => {
                if args.len() != 1 {
                    return Err(Diag::new(span, format!("'{}' expects 1 argument", name)));
                }
                Ok(Ty::Void)
            }
            "gl_vertex" | "webgpu_vertex" => {
                if args.len() != 3 {
                    return Err(Diag::new(span, format!("'{}' expects 3 arguments", name)));
                }
                Ok(Ty::Void)
            }
            "gl_end" | "gl_swap" | "webgpu_end" | "webgpu_swap" => {
                if !args.is_empty() {
                    return Err(Diag::new(span, format!("'{}' expects 0 arguments", name)));
                }
                if name.ends_with("swap") {
                    Ok(Ty::Int)
                } else {
                    Ok(Ty::Void)
                }
            }
            "gl_width" | "gl_height" | "webgpu_width" | "webgpu_height" => {
                if !args.is_empty() {
                    return Err(Diag::new(span, format!("'{}' expects 0 arguments", name)));
                }
                Ok(Ty::Int)
            }
            "gl_rect" | "gl_line" | "webgpu_rect" | "webgpu_line" => {
                if args.len() != 4 {
                    return Err(Diag::new(span, format!("'{}' expects 4 arguments", name)));
                }
                Ok(Ty::Void)
            }
            "gl_circle" | "webgpu_circle" => {
                if args.len() != 4 {
                    return Err(Diag::new(span, format!("'{}' expects 4 arguments", name)));
                }
                Ok(Ty::Void)
            }
            "gl_title" | "webgpu_title" => {
                if args.len() != 1 {
                    return Err(Diag::new(span, format!("'{}' expects 1 argument", name)));
                }
                Ok(Ty::Void)
            }
            _ => {
                if RESERVED_WASM.contains(&name) {
                    return Err(Diag::new(
                        span,
                        format!("'{}' is not available on the web target", name),
                    ));
                }
                if let Some(idx) = self.name_to_idx.get(name) {
                    let sig = self.funcs[*idx].clone();
                    if args.len() != sig.params.len() {
                        return Err(Diag::new(
                            span,
                            format!(
                                "'{}' expects {} argument(s), got {}",
                                name,
                                sig.params.len(),
                                args.len()
                            ),
                        ));
                    }
                    Ok(sig.ret)
                } else {
                    Err(Diag::new(span, format!("unknown function '{}'", name)))
                }
            }
        }
    }

    fn truthy(&mut self, ty: &Ty, span: Span) -> Result<(), Diag> {
        match ty {
            Ty::Bool => Ok(()),
            Ty::Int => {
                self.body.push(0x42);
                sleb(&mut self.body, 0);
                self.body.push(0x52);
                Ok(())
            }
            Ty::Float => {
                self.body.push(0x44);
                self.body.extend_from_slice(&0f64.to_bits().to_le_bytes());
                self.body.push(0x62);
                Ok(())
            }
            other => Err(Diag::new(
                span,
                format!("expected a bool or number for a condition, found '{}'", other),
            )),
        }
    }

    fn coerce(&mut self, from: &Ty, to: &Ty, span: Span) -> Result<(), Diag> {
        if from == to {
            return Ok(());
        }
        match (from, to) {
            (Ty::Int, Ty::Float) => {
                self.body.push(0xB9);
                Ok(())
            }
            (Ty::Float, Ty::Int) => {
                self.body.push(0xB0);
                Ok(())
            }
            (Ty::Bool, Ty::Int) => {
                self.body.push(0xAD);
                Ok(())
            }
            (Ty::Void, _) => Err(Diag::new(span, "cannot use a value of type 'void'")),
            _ => Err(Diag::new(
                span,
                format!("type mismatch: expected '{}', found '{}'", to, from),
            )),
        }
    }

    fn expr(&mut self, e: &Expr) -> Result<Ty, Diag> {
        match e {
            Expr::Int(v, _) => {
                self.body.push(0x42);
                sleb(&mut self.body, *v);
                Ok(Ty::Int)
            }
            Expr::Float(v, _) => {
                self.body.push(0x44);
                self.body.extend_from_slice(&v.to_bits().to_le_bytes());
                Ok(Ty::Float)
            }
            Expr::Bool(b, _) => {
                self.body.push(0x41);
                sleb(&mut self.body, if *b { 1 } else { 0 });
                Ok(Ty::Bool)
            }
            Expr::Str(_, span) => Err(Diag::new(
                *span,
                "string values are only supported as direct 'print'/'assert' arguments on the web target",
            )),
            Expr::Nil(span) => Err(Diag::new(*span, "'nil' is not supported on the web target yet")),
            Expr::Var(name, span) => {
                let Some(v) = self.lookup(name).cloned() else {
                    return Err(Diag::new(*span, format!("undefined variable '{}'", name)));
                };
                self.body.push(0x20);
                uleb(&mut self.body, v.idx as u64);
                Ok(v.ty)
            }
            Expr::Array(_, span) => Err(Diag::new(*span, "arrays are not supported on the web target yet")),
            Expr::Tuple(_, span) => Err(Diag::new(*span, "tuples are not supported on the web target yet")),
            Expr::Field(_, _, span) => Err(Diag::new(*span, "structs are not supported on the web target yet")),
            Expr::StructLit { span, .. } => Err(Diag::new(*span, "structs are not supported on the web target yet")),
            Expr::Index(_, _, span) => Err(Diag::new(*span, "arrays are not supported on the web target yet")),
            Expr::Unary { op, operand, span } => match op {
                UnOp::Not => {
                    let t = self.expr(operand)?;
                    self.truthy(&t, *span)?;
                    self.body.push(0x45);
                    Ok(Ty::Bool)
                }
                UnOp::Neg => {
                    let t = self.ty_of(operand)?;
                    match t {
                        Ty::Int => {
                            self.body.push(0x42);
                            sleb(&mut self.body, 0);
                            self.expr(operand)?;
                            self.body.push(0x7D);
                            Ok(Ty::Int)
                        }
                        Ty::Float => {
                            self.expr(operand)?;
                            self.body.push(0x9A);
                            Ok(Ty::Float)
                        }
                        other => Err(Diag::new(
                            *span,
                            format!("cannot negate a value of type '{}'", other),
                        )),
                    }
                }
                UnOp::BitNot => {
                    let t = self.expr(operand)?;
                    if t != Ty::Int {
                        return Err(Diag::new(*span, format!("'~' expects int, found '{}'", t)));
                    }
                    self.body.push(0x42);
                    sleb(&mut self.body, -1);
                    self.body.push(0x85);
                    Ok(Ty::Int)
                }
            },
            Expr::Binary { op, lhs, rhs, span } => self.binary(*op, lhs, rhs, *span),
            Expr::Call { name, args, span } => self.call(name, args, *span),
        }
    }

    fn binary(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr, span: Span) -> Result<Ty, Diag> {
        match op {
            BinOp::And => {
                let lt = self.ty_of(lhs)?;
                self.expr(lhs)?;
                self.truthy(&lt, lhs.span())?;
                self.body.push(0x04);
                self.body.push(VT_I32);
                let rt = self.ty_of(rhs)?;
                self.expr(rhs)?;
                self.truthy(&rt, rhs.span())?;
                self.body.push(0x05);
                self.body.push(0x41);
                sleb(&mut self.body, 0);
                self.body.push(0x0B);
                return Ok(Ty::Bool);
            }
            BinOp::Or => {
                let lt = self.ty_of(lhs)?;
                self.expr(lhs)?;
                self.truthy(&lt, lhs.span())?;
                self.body.push(0x04);
                self.body.push(VT_I32);
                self.body.push(0x41);
                sleb(&mut self.body, 1);
                self.body.push(0x05);
                let rt = self.ty_of(rhs)?;
                self.expr(rhs)?;
                self.truthy(&rt, rhs.span())?;
                self.body.push(0x0B);
                return Ok(Ty::Bool);
            }
            _ => {}
        }
        let lt = self.ty_of(lhs)?;
        let rt = self.ty_of(rhs)?;
        match op {
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let t = unify_cmp(&lt, &rt, span)?;
                self.expr(lhs)?;
                self.coerce(&lt, &t, lhs.span())?;
                self.expr(rhs)?;
                self.coerce(&rt, &t, rhs.span())?;
                let opc = match (&t, op) {
                    (Ty::Int, BinOp::Eq) => 0x51,
                    (Ty::Int, BinOp::Ne) => 0x52,
                    (Ty::Int, BinOp::Lt) => 0x53,
                    (Ty::Int, BinOp::Gt) => 0x55,
                    (Ty::Int, BinOp::Le) => 0x57,
                    (Ty::Int, BinOp::Ge) => 0x59,
                    (Ty::Float, BinOp::Eq) => 0x61,
                    (Ty::Float, BinOp::Ne) => 0x62,
                    (Ty::Float, BinOp::Lt) => 0x63,
                    (Ty::Float, BinOp::Gt) => 0x64,
                    (Ty::Float, BinOp::Le) => 0x65,
                    (Ty::Float, BinOp::Ge) => 0x66,
                    (Ty::Bool, BinOp::Eq) => 0x46,
                    (Ty::Bool, BinOp::Ne) => 0x47,
                    _ => {
                        return Err(Diag::new(
                            span,
                            format!("cannot compare values of type '{}'", t),
                        ))
                    }
                };
                self.body.push(opc);
                Ok(Ty::Bool)
            }
            _ => {
                let t = unify_num(&lt, &rt, span)?;
                self.expr(lhs)?;
                self.coerce(&lt, &t, lhs.span())?;
                self.expr(rhs)?;
                self.coerce(&rt, &t, rhs.span())?;
                let opc = match (&t, op) {
                    (Ty::Int, BinOp::Add) => 0x7C,
                    (Ty::Int, BinOp::Sub) => 0x7D,
                    (Ty::Int, BinOp::Mul) => 0x7E,
                    (Ty::Int, BinOp::Div) => 0x7F,
                    (Ty::Int, BinOp::Mod) => 0x81,
                    (Ty::Int, BinOp::BitAnd) => 0x83,
                    (Ty::Int, BinOp::BitOr) => 0x84,
                    (Ty::Int, BinOp::BitXor) => 0x85,
                    (Ty::Int, BinOp::Shl) => 0x86,
                    (Ty::Int, BinOp::Shr) => 0x87,
                    (Ty::Float, BinOp::Add) => 0xA0,
                    (Ty::Float, BinOp::Sub) => 0xA1,
                    (Ty::Float, BinOp::Mul) => 0xA2,
                    (Ty::Float, BinOp::Div) => 0xA3,
                    (Ty::Float, BinOp::Mod) => {
                        return Err(Diag::new(span, "cannot use '%' on floats"))
                    }
                    (Ty::Float, BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr) => {
                        return Err(Diag::new(
                            span,
                            "bitwise operators expect int operands, found float",
                        ))
                    }
                    _ => {
                        return Err(Diag::new(
                            span,
                            format!("cannot apply arithmetic to '{}'", t),
                        ))
                    }
                };
                self.body.push(opc);
                Ok(t)
            }
        }
    }

    fn call(&mut self, name: &str, args: &[Expr], span: Span) -> Result<Ty, Diag> {
        self.call_ty(name, args, span)?;
        match name {
            "assert" => {
                if args.len() != 2 {
                    return Err(Diag::new(span, "'assert' expects 2 arguments"));
                }
                let ct = self.ty_of(&args[0])?;
                self.expr(args.first().unwrap())?;
                self.truthy(&ct, args[0].span())?;
                self.body.push(0x45);
                self.body.push(0x04);
                self.body.push(0x40);
                let Expr::Str(msg, _) = &args[1] else {
                    return Err(Diag::new(
                        args[1].span(),
                        "on the web target 'assert' needs a string literal message",
                    ));
                };
                let full = format!("assertion failed: {}", msg);
                let off = self.intern(&full);
                self.body.push(0x41);
                sleb(&mut self.body, off as i64);
                self.body.push(0x41);
                sleb(&mut self.body, full.len() as i64);
                self.call_import(IMP_PRINT_S);
                self.call_import(IMP_PRINT_NL);
                self.body.push(0x41);
                sleb(&mut self.body, 1);
                self.call_import(IMP_EXIT);
                self.body.push(0x0B);
                return Ok(Ty::Void);
            }
            "exit" => {
                if args.len() != 1 {
                    return Err(Diag::new(span, "'exit' expects 1 argument"));
                }
                let t = self.ty_of(&args[0])?;
                self.expr(&args[0])?;
                self.coerce(&t, &Ty::Int, args[0].span())?;
                self.body.push(0xA7);
                self.call_import(IMP_EXIT);
                return Ok(Ty::Void);
            }
            "sin" | "cos" | "sqrt" | "abs" | "floor" => {
                let t = self.ty_of(&args[0])?;
                self.expr(&args[0])?;
                self.coerce(&t, &Ty::Float, args[0].span())?;
                match name {
                    "sin" => self.call_import(IMP_SIN),
                    "cos" => self.call_import(IMP_COS),
                    "sqrt" => self.body.push(0x9F),
                    "abs" => self.body.push(0x99),
                    _ => self.body.push(0x9C),
                }
                return Ok(Ty::Float);
            }
            "pow" => {
                let t1 = self.ty_of(&args[0])?;
                self.expr(&args[0])?;
                self.coerce(&t1, &Ty::Float, args[0].span())?;
                let t2 = self.ty_of(&args[1])?;
                self.expr(&args[1])?;
                self.coerce(&t2, &Ty::Float, args[1].span())?;
                self.call_import(IMP_POW);
                return Ok(Ty::Float);
            }
            "rand" => {
                self.call_import(IMP_RAND);
                return Ok(Ty::Float);
            }
            "gl_init" => {
                if args.len() != 3 {
                    return Err(Diag::new(span, "'gl_init' expects 3 arguments"));
                }
                let t1 = self.ty_of(&args[0])?;
                self.expr(&args[0])?;
                self.coerce(&t1, &Ty::Int, args[0].span())?;
                let t2 = self.ty_of(&args[1])?;
                self.expr(&args[1])?;
                self.coerce(&t2, &Ty::Int, args[1].span())?;
                let Expr::Str(s, _) = &args[2] else {
                    return Err(Diag::new(args[2].span(), "on the web target 'gl_init' needs a string literal title"));
                };
                let off = self.intern(s);
                self.body.push(0x41);
                sleb(&mut self.body, off as i64);
                self.body.push(0x41);
                sleb(&mut self.body, s.len() as i64);
                self.call_import(IMP_GFX_INIT);
                return Ok(Ty::Void);
            }
            "gl_clear" => {
                if args.len() != 3 {
                    return Err(Diag::new(span, "'gl_clear' expects 3 arguments"));
                }
                for i in 0..3 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Int, args[i].span())?;
                }
                self.call_import(IMP_GFX_CLEAR);
                return Ok(Ty::Void);
            }
            "gl_color" => {
                if args.len() != 4 {
                    return Err(Diag::new(span, "'gl_color' expects 4 arguments (r,g,b,a)"));
                }
                for i in 0..4 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Int, args[i].span())?;
                }
                /* JS side ignores alpha for now — only pass rgb */
                self.body.push(0x1A); /* drop */
                self.call_import(IMP_GFX_COLOR);
                return Ok(Ty::Void);
            }
            "gl_begin" => {
                if args.len() != 1 {
                    return Err(Diag::new(span, "'gl_begin' expects 1 argument"));
                }
                let t = self.ty_of(&args[0])?;
                self.expr(&args[0])?;
                self.coerce(&t, &Ty::Int, args[0].span())?;
                self.call_import(IMP_GFX_BEGIN);
                return Ok(Ty::Void);
            }
            "gl_vertex" => {
                if args.len() != 3 {
                    return Err(Diag::new(span, "'gl_vertex' expects 3 arguments"));
                }
                for i in 0..3 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Float, args[i].span())?;
                }
                self.call_import(IMP_GFX_VERTEX);
                return Ok(Ty::Void);
            }
            "gl_end" => {
                if args.len() != 0 {
                    return Err(Diag::new(span, "'gl_end' expects 0 arguments"));
                }
                self.call_import(IMP_GFX_END);
                return Ok(Ty::Void);
            }
            "gl_swap" => {
                if args.len() != 0 {
                    return Err(Diag::new(span, "'gl_swap' expects 0 arguments"));
                }
                self.call_import(IMP_GFX_SWAP);
                return Ok(Ty::Int);
            }
            "gl_width" => {
                if args.len() != 0 {
                    return Err(Diag::new(span, "'gl_width' expects 0 arguments"));
                }
                self.call_import(IMP_GFX_WIDTH);
                return Ok(Ty::Int);
            }
            "gl_height" => {
                if args.len() != 0 {
                    return Err(Diag::new(span, "'gl_height' expects 0 arguments"));
                }
                self.call_import(IMP_GFX_HEIGHT);
                return Ok(Ty::Int);
            }
            "gl_rect" => {
                if args.len() != 4 {
                    return Err(Diag::new(span, "'gl_rect' expects 4 arguments"));
                }
                for i in 0..4 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Float, args[i].span())?;
                }
                self.call_import(IMP_GFX_RECT);
                return Ok(Ty::Void);
            }
            "gl_circle" => {
                if args.len() != 4 {
                    return Err(Diag::new(span, "'gl_circle' expects 4 arguments"));
                }
                for i in 0..3 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Float, args[i].span())?;
                }
                let t = self.ty_of(&args[3])?;
                self.expr(&args[3])?;
                self.coerce(&t, &Ty::Int, args[3].span())?;
                self.call_import(IMP_GFX_CIRCLE);
                return Ok(Ty::Void);
            }
            "gl_line" => {
                if args.len() != 4 {
                    return Err(Diag::new(span, "'gl_line' expects 4 arguments"));
                }
                for i in 0..4 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Float, args[i].span())?;
                }
                self.call_import(IMP_GFX_LINE);
                return Ok(Ty::Void);
            }
            "gl_title" => {
                if args.len() != 1 {
                    return Err(Diag::new(span, "'gl_title' expects 1 argument"));
                }
                let Expr::Str(s, _) = &args[0] else {
                    return Err(Diag::new(args[0].span(), "on the web target 'gl_title' needs a string literal"));
                };
                let off = self.intern(s);
                self.body.push(0x41);
                sleb(&mut self.body, off as i64);
                self.body.push(0x41);
                sleb(&mut self.body, s.len() as i64);
                self.call_import(IMP_GFX_TITLE);
                return Ok(Ty::Void);
            }
            "webgpu_init" => {
                if args.len() != 3 {
                    return Err(Diag::new(span, "'webgpu_init' expects 3 arguments"));
                }
                let t1 = self.ty_of(&args[0])?;
                self.expr(&args[0])?;
                self.coerce(&t1, &Ty::Int, args[0].span())?;
                let t2 = self.ty_of(&args[1])?;
                self.expr(&args[1])?;
                self.coerce(&t2, &Ty::Int, args[1].span())?;
                let Expr::Str(s, _) = &args[2] else {
                    return Err(Diag::new(args[2].span(), "on the web target 'webgpu_init' needs a string literal title"));
                };
                let off = self.intern(s);
                self.body.push(0x41);
                sleb(&mut self.body, off as i64);
                self.body.push(0x41);
                sleb(&mut self.body, s.len() as i64);
                self.call_import(IMP_GPU_INIT);
                return Ok(Ty::Void);
            }
            "webgpu_clear" => {
                if args.len() != 3 {
                    return Err(Diag::new(span, "'webgpu_clear' expects 3 arguments"));
                }
                for i in 0..3 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Int, args[i].span())?;
                }
                self.call_import(IMP_GPU_CLEAR);
                return Ok(Ty::Void);
            }
            "webgpu_color" => {
                if args.len() != 4 {
                    return Err(Diag::new(span, "'webgpu_color' expects 4 arguments (r,g,b,a)"));
                }
                for i in 0..4 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Int, args[i].span())?;
                }
                self.body.push(0x1A); /* drop alpha */
                self.call_import(IMP_GPU_COLOR);
                return Ok(Ty::Void);
            }
            "webgpu_begin" => {
                if args.len() != 1 {
                    return Err(Diag::new(span, "'webgpu_begin' expects 1 argument"));
                }
                let t = self.ty_of(&args[0])?;
                self.expr(&args[0])?;
                self.coerce(&t, &Ty::Int, args[0].span())?;
                self.call_import(IMP_GPU_BEGIN);
                return Ok(Ty::Void);
            }
            "webgpu_vertex" => {
                if args.len() != 3 {
                    return Err(Diag::new(span, "'webgpu_vertex' expects 3 arguments"));
                }
                for i in 0..3 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Float, args[i].span())?;
                }
                self.call_import(IMP_GPU_VERTEX);
                return Ok(Ty::Void);
            }
            "webgpu_end" => {
                if args.len() != 0 {
                    return Err(Diag::new(span, "'webgpu_end' expects 0 arguments"));
                }
                self.call_import(IMP_GPU_END);
                return Ok(Ty::Void);
            }
            "webgpu_swap" => {
                if args.len() != 0 {
                    return Err(Diag::new(span, "'webgpu_swap' expects 0 arguments"));
                }
                self.call_import(IMP_GPU_SWAP);
                return Ok(Ty::Int);
            }
            "webgpu_width" => {
                if args.len() != 0 {
                    return Err(Diag::new(span, "'webgpu_width' expects 0 arguments"));
                }
                self.call_import(IMP_GPU_WIDTH);
                return Ok(Ty::Int);
            }
            "webgpu_height" => {
                if args.len() != 0 {
                    return Err(Diag::new(span, "'webgpu_height' expects 0 arguments"));
                }
                self.call_import(IMP_GPU_HEIGHT);
                return Ok(Ty::Int);
            }
            "webgpu_rect" => {
                if args.len() != 4 {
                    return Err(Diag::new(span, "'webgpu_rect' expects 4 arguments"));
                }
                for i in 0..4 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Float, args[i].span())?;
                }
                self.call_import(IMP_GPU_RECT);
                return Ok(Ty::Void);
            }
            "webgpu_circle" => {
                if args.len() != 4 {
                    return Err(Diag::new(span, "'webgpu_circle' expects 4 arguments"));
                }
                for i in 0..3 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Float, args[i].span())?;
                }
                let t = self.ty_of(&args[3])?;
                self.expr(&args[3])?;
                self.coerce(&t, &Ty::Int, args[3].span())?;
                self.call_import(IMP_GPU_CIRCLE);
                return Ok(Ty::Void);
            }
            "webgpu_line" => {
                if args.len() != 4 {
                    return Err(Diag::new(span, "'webgpu_line' expects 4 arguments"));
                }
                for i in 0..4 {
                    let t = self.ty_of(&args[i])?;
                    self.expr(&args[i])?;
                    self.coerce(&t, &Ty::Float, args[i].span())?;
                }
                self.call_import(IMP_GPU_LINE);
                return Ok(Ty::Void);
            }
            "webgpu_title" => {
                if args.len() != 1 {
                    return Err(Diag::new(span, "'webgpu_title' expects 1 argument"));
                }
                let Expr::Str(s, _) = &args[0] else {
                    return Err(Diag::new(args[0].span(), "on the web target 'webgpu_title' needs a string literal"));
                };
                let off = self.intern(s);
                self.body.push(0x41);
                sleb(&mut self.body, off as i64);
                self.body.push(0x41);
                sleb(&mut self.body, s.len() as i64);
                self.call_import(IMP_GPU_TITLE);
                return Ok(Ty::Void);
            }
            "int" => {
                let t = self.ty_of(&args[0])?;
                self.expr(&args[0])?;
                match t {
                    Ty::Int => {}
                    Ty::Bool => {
                        self.body.push(0xAD);
                    }
                    _ => {
                        self.body.push(0xB0);
                    }
                }
                return Ok(Ty::Int);
            }
            "float" => {
                let t = self.ty_of(&args[0])?;
                self.expr(&args[0])?;
                match t {
                    Ty::Float => {}
                    Ty::Bool => {
                        self.body.push(0xB8);
                    }
                    _ => {
                        self.body.push(0xB9);
                    }
                }
                return Ok(Ty::Float);
            }
            "print" => Err(Diag::new(
                span,
                "'print' returns no value; call it as a statement, e.g. print(x);",
            )),
            _ => {
                let idx = *self
                    .name_to_idx
                    .get(name)
                    .ok_or_else(|| Diag::new(span, format!("unknown function '{}'", name)))?;
                let sig = self.funcs[idx].clone();
                for (i, arg) in args.iter().enumerate() {
                    let at = self.ty_of(arg)?;
                    self.expr(arg)?;
                    self.coerce(&at, &sig.params[i], arg.span())?;
                }
                self.body.push(0x10);
                uleb(&mut self.body, (NIMPORTS + idx as u32) as u64);
                Ok(sig.ret)
            }
        }
    }

    fn emit_print(&mut self, args: &[Expr], span: Span) -> Result<(), Diag> {
        if args.is_empty() {
            self.call_import(IMP_PRINT_NL);
            return Ok(());
        }
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.call_import(IMP_PRINT_SP);
            }
            match arg {
                Expr::Str(s, _) => {
                    let off = self.intern(s);
                    self.body.push(0x41);
                    sleb(&mut self.body, off as i64);
                    self.body.push(0x41);
                    sleb(&mut self.body, s.len() as i64);
                    self.call_import(IMP_PRINT_S);
                }
                _ => {
                    let t = self.expr(arg)?;
                    match t {
                        Ty::Int => self.call_import(IMP_PRINT_I),
                        Ty::Float => self.call_import(IMP_PRINT_F),
                        Ty::Bool => self.call_import(IMP_PRINT_B),
                        other => {
                            return Err(Diag::new(
                                arg.span(),
                                format!("cannot print a value of type '{}' on the web target yet", other),
                            ))
                        }
                    }
                }
            }
        }
        let _ = span;
        self.call_import(IMP_PRINT_NL);
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diag> {
        match stmt {
            Stmt::Var { name, ty, init, mutable, span } => {
                let it = self.expr(init)?;
                let ft = match ty {
                    Some(a) => {
                        self.coerce(&it, a, *span)?;
                        a.clone()
                    }
                    None => it,
                };
                if ft == Ty::Str {
                    return Err(Diag::new(
                        *span,
                        "string variables are not supported on the web target yet",
                    ));
                }
                let vt = valtype(&ft, *span)?;
                let idx = self.nparams + self.locals.len() as u32;
                self.locals.push(vt);
                self.body.push(0x21);
                uleb(&mut self.body, idx as u64);
                self.scopes.last_mut().unwrap().insert(
                    name.clone(),
                    VarInfo { ty: ft, mutable: *mutable, idx },
                );
                Ok(())
            }
            Stmt::Assign { target, value, span } => {
                let Expr::Var(name, vspan) = target else {
                    return Err(Diag::new(
                        *span,
                        "on the web target only simple variables can be assigned",
                    ));
                };
                let Some(v) = self.lookup(name).cloned() else {
                    return Err(Diag::new(*vspan, format!("undefined variable '{}'", name)));
                };
                if !v.mutable {
                    return Err(Diag::new(*vspan, format!("variable '{}' is immutable", name)));
                }
                let vt = self.ty_of(value)?;
                self.expr(value)?;
                self.coerce(&vt, &v.ty, value.span())?;
                self.body.push(0x21);
                uleb(&mut self.body, v.idx as u64);
                Ok(())
            }
            Stmt::If { cond, then, els, .. } => {
                let ct = self.ty_of(cond)?;
                self.expr(cond)?;
                self.truthy(&ct, cond.span())?;
                self.body.push(0x04);
                self.body.push(0x40);
                self.frames.push(Frame::If);
                self.scopes.push(HashMap::new());
                for s in &then.stmts {
                    self.emit_stmt(s)?;
                }
                self.scopes.pop();
                self.frames.pop();
                if let Some(e) = els {
                    self.body.push(0x05);
                    self.frames.push(Frame::If);
                    self.scopes.push(HashMap::new());
                    self.emit_stmt(e)?;
                    self.scopes.pop();
                    self.frames.pop();
                }
                self.body.push(0x0B);
                Ok(())
            }
            Stmt::While { cond, body, .. } => {
                self.body.push(0x02);
                self.body.push(0x40);
                self.frames.push(Frame::Block);
                self.body.push(0x03);
                self.body.push(0x40);
                self.frames.push(Frame::Loop);
                let ct = self.ty_of(cond)?;
                self.expr(cond)?;
                self.truthy(&ct, cond.span())?;
                self.body.push(0x45);
                self.body.push(0x0D);
                uleb(&mut self.body, 1);
                self.scopes.push(HashMap::new());
                for s in &body.stmts {
                    self.emit_stmt(s)?;
                }
                self.scopes.pop();
                self.body.push(0x0C);
                uleb(&mut self.body, 0);
                self.body.push(0x0B);
                self.frames.pop();
                self.body.push(0x0B);
                self.frames.pop();
                Ok(())
            }
            Stmt::Return { value, span } => {
                let want = self.ret.clone();
                match (value, want) {
                    (None, Ty::Void) => {
                        self.body.push(0x0F);
                        Ok(())
                    }
                    (None, other) => Err(Diag::new(
                        *span,
                        format!("this function must return a value of type '{}'", other),
                    )),
                    (Some(_), Ty::Void) => {
                        Err(Diag::new(*span, "cannot return a value from a void function"))
                    }
                    (Some(e), want) => {
                        let t = self.ty_of(e)?;
                        self.expr(e)?;
                        self.coerce(&t, &want, e.span())?;
                        self.body.push(0x0F);
                        Ok(())
                    }
                }
            }
            Stmt::Break(span) => {
                let mut loop_idx = None;
                for (i, f) in self.frames.iter().enumerate().rev() {
                    if matches!(f, Frame::Loop) {
                        loop_idx = Some(i);
                        break;
                    }
                }
                let Some(li) = loop_idx else {
                    return Err(Diag::new(*span, "'break' outside of a loop"));
                };
                let depth = (self.frames.len() - 1 - (li - 1)) as u64;
                self.body.push(0x0C);
                uleb(&mut self.body, depth);
                Ok(())
            }
            Stmt::Continue(span) => {
                let mut loop_idx = None;
                for (i, f) in self.frames.iter().enumerate().rev() {
                    if matches!(f, Frame::Loop) {
                        loop_idx = Some(i);
                        break;
                    }
                }
                let Some(li) = loop_idx else {
                    return Err(Diag::new(*span, "'continue' outside of a loop"));
                };
                let depth = (self.frames.len() - 1 - li) as u64;
                self.body.push(0x0C);
                uleb(&mut self.body, depth);
                Ok(())
            }
            Stmt::Block(b) => {
                self.scopes.push(HashMap::new());
                for s in &b.stmts {
                    self.emit_stmt(s)?;
                }
                self.scopes.pop();
                Ok(())
            }
            Stmt::Expr(e, span) => {
                if let Expr::Call { name, args, .. } = e {
                    if name == "print" {
                        return self.emit_print(args, *span);
                    }
                }
                let t = self.expr(e)?;
                if t != Ty::Void {
                    self.body.push(0x1A);
                }
                Ok(())
            }
            Stmt::Switch { value, cases, default, span } => {
                let vt = self.ty_of(value)?;
                if vt == Ty::Str {
                    return Err(Diag::new(*span, "switch on strings is not supported on the web target"));
                }
                // Evaluate value once into a temp local
                self.expr(value)?;
                let idx = self.nparams + self.locals.len() as u32;
                self.locals.push(valtype(&vt, *span)?);
                self.body.push(0x21); // local.set
                uleb(&mut self.body, idx as u64);

                // For each case: push cond; if { body } else { ... next case or default ... }
                // Structure:
                //   local.get v; pattern; eq
                //   if
                //     body1
                //   else
                //     local.get v; pattern2; eq
                //     if
                //       body2
                //     else
                //       default
                //     end
                //   end
                for case in cases {
                    if case.patterns.is_empty() {
                        return Err(Diag::new(case.span, "'case' needs at least one pattern"));
                    }
                    for (pi, p) in case.patterns.iter().enumerate() {
                        let e = crate::codegen::as_value_pattern(p);
                        self.body.push(0x20); // local.get
                        uleb(&mut self.body, idx as u64);
                        let pt = self.ty_of(&e)?;
                        self.expr(&e)?;
                        self.coerce(&pt, &vt, e.span())?;
                        match vt {
                            Ty::Int | Ty::Bool => self.body.push(0x46),
                            Ty::Float => self.body.push(0x61),
                            _ => return Err(Diag::new(*span, "unsupported switch type on web")),
                        }
                        if pi > 0 {
                            self.body.push(0x71); // i32.or
                        }
                    }
                    self.body.push(0x04); // if
                    self.body.push(0x40);
                    self.frames.push(Frame::If);
                    self.scopes.push(HashMap::new());
                    for s in &case.body.stmts {
                        self.emit_stmt(s)?;
                    }
                    self.scopes.pop();
                    self.frames.pop();
                    self.body.push(0x05); // else
                }
                // default or empty
                if let Some(db) = default {
                    self.frames.push(Frame::If);
                    self.scopes.push(HashMap::new());
                    for s in &db.stmts {
                        self.emit_stmt(s)?;
                    }
                    self.scopes.pop();
                    self.frames.pop();
                }
                // Close every open if-block
                for _ in 0..cases.len() {
                    self.body.push(0x0B); // end
                }
                Ok(())
            }
        }
    }
}

fn unify_num(a: &Ty, b: &Ty, span: Span) -> Result<Ty, Diag> {
    if a == b && a.is_numeric() {
        return Ok(a.clone());
    }
    if a.is_numeric() && b.is_numeric() {
        return Ok(Ty::Float);
    }
    Err(Diag::new(
        span,
        format!("type mismatch: cannot combine '{}' and '{}'", a, b),
    ))
}

fn unify_cmp(a: &Ty, b: &Ty, span: Span) -> Result<Ty, Diag> {
    if a == b {
        return Ok(a.clone());
    }
    if a.is_numeric() && b.is_numeric() {
        return Ok(Ty::Float);
    }
    Err(Diag::new(
        span,
        format!("type mismatch: cannot compare '{}' and '{}'", a, b),
    ))
}

fn block_terminates(b: &Block) -> bool {
    b.stmts.iter().any(stmt_terminates)
}

fn stmt_terminates(s: &Stmt) -> bool {
    match s {
        Stmt::Return { .. } => true,
        Stmt::If { then, els, .. } => match els {
            Some(e) => block_terminates(then) && stmt_terminates(e),
            None => false,
        },
        Stmt::Switch { cases, default, .. } => match default {
            Some(d) => block_terminates(d) && cases.iter().all(|c| block_terminates(&c.body)),
            None => false,
        },
        Stmt::Block(b) => block_terminates(b),
        _ => false,
    }
}

pub fn generate(program: &Program) -> Result<Vec<u8>, Diag> {
    let mut g = WasmGen::new();
    let mut seen = HashSet::new();
    let mut defined: Vec<&FnDecl> = Vec::new();
    for f in &program.funcs {
        if RESERVED_WASM.contains(&f.name.as_str()) {
            return Err(Diag::new(
                f.span,
                format!("'{}' is a built-in name and cannot be used as a function name", f.name),
            ));
        }
        if f.is_extern {
            return Err(Diag::new(
                f.span,
                format!("extern function '{}' is not supported on the web target", f.name),
            ));
        }
        if !seen.insert(f.name.clone()) {
            return Err(Diag::new(f.span, format!("function '{}' is already defined", f.name)));
        }
        for p in &f.params {
            valtype(&p.ty, p.span)?;
        }
        defined.push(f);
    }
    if !program.structs.is_empty() {
        return Err(Diag::new(
            program.structs[0].span,
            "structs are not supported on the web target yet",
        ));
    }
    if defined.iter().any(|f| matches!(f.ret, Ty::Array(_) | Ty::Named(_) | Ty::Ptr)) {
        let f = defined
            .iter()
            .find(|f| matches!(f.ret, Ty::Array(_) | Ty::Named(_) | Ty::Ptr))
            .unwrap();
        return Err(Diag::new(
            f.span,
            format!("return type '{}' is not supported on the web target yet", f.ret),
        ));
    }
    let Some(main_fn) = defined.iter().find(|f| f.name == "main") else {
        return Err(Diag::new(
            Span::new(0, 1, 1),
            "program has no 'main' function; add: fn main() -> int { return 0; }",
        ));
    };
    if !main_fn.params.is_empty() {
        return Err(Diag::new(main_fn.span, "'main' function must not take parameters"));
    }
    if !matches!(main_fn.ret, Ty::Int | Ty::Void) {
        return Err(Diag::new(main_fn.span, "'main' must return 'int' or nothing"));
    }
    for (i, f) in defined.iter().enumerate() {
        g.name_to_idx.insert(f.name.clone(), i);
        g.funcs.push(FuncSig {
            params: f.params.iter().map(|p| p.ty.clone()).collect(),
            ret: f.ret.clone(),
        });
    }

    let mut import_payload = Vec::new();
    uleb(&mut import_payload, IMPORTS.len() as u64);
    for (mname, params, results) in IMPORTS {
        let ti = g.type_idx(params, results);
        uleb(&mut import_payload, 3);
        import_payload.extend_from_slice(b"env");
        uleb(&mut import_payload, mname.len() as u64);
        import_payload.extend_from_slice(mname.as_bytes());
        import_payload.push(0x00);
        uleb(&mut import_payload, ti as u64);
    }

    let mut func_payload = Vec::new();
    uleb(&mut func_payload, defined.len() as u64);
    for f in &defined {
        let params: Vec<u8> = {
            let mut v = Vec::new();
            for p in &f.params {
                v.push(vt_byte(&p.ty).unwrap_or(VT_I64));
            }
            v
        };
        let results: Vec<u8> = match &f.ret {
            Ty::Void => Vec::new(),
            t => vec![vt_byte(t).unwrap()],
        };
        let ti = g.type_idx(&params, &results);
        g.func_typeidx.push(ti);
        uleb(&mut func_payload, ti as u64);
    }

    let mut bodies: Vec<Vec<u8>> = Vec::new();
    for f in &defined {
        g.body = Vec::new();
        g.locals = Vec::new();
        g.scopes = vec![HashMap::new()];
        g.frames = Vec::new();
        g.nparams = f.params.len() as u32;
        g.ret = f.ret.clone();
        for (i, p) in f.params.iter().enumerate() {
            let vt = vt_byte(&p.ty).unwrap();
            g.locals.push(vt);
            g.scopes.last_mut().unwrap().insert(
                p.name.clone(),
                VarInfo { ty: p.ty.clone(), mutable: true, idx: i as u32 },
            );
        }
        for s in &f.body.stmts {
            g.emit_stmt(s)?;
        }
        if f.ret != Ty::Void && !block_terminates(&f.body) {
            return Err(Diag::new(
                f.span,
                format!(
                    "function '{}' must return a value of type '{}' on every execution path",
                    f.name, f.ret
                ),
            ));
        }
        g.body.push(0x0B);
        let mut body = Vec::new();
        uleb(&mut body, g.locals.len() as u64);
        for vt in &g.locals {
            uleb(&mut body, 1);
            body.push(*vt);
        }
        body.extend_from_slice(&g.body);
        bodies.push(body);
    }

    let mut code_payload = Vec::new();
    uleb(&mut code_payload, bodies.len() as u64);
    for b in &bodies {
        uleb(&mut code_payload, b.len() as u64);
        code_payload.extend_from_slice(b);
    }

    let mut export_payload = Vec::new();
    uleb(&mut export_payload, (defined.len() + 1) as u64);
    export_payload.extend_from_slice(b"\x06memory");
    export_payload.push(0x02);
    uleb(&mut export_payload, 0);
    for (i, f) in defined.iter().enumerate() {
        uleb(&mut export_payload, f.name.len() as u64);
        export_payload.extend_from_slice(f.name.as_bytes());
        export_payload.push(0x00);
        uleb(&mut export_payload, (NIMPORTS + i as u32) as u64);
    }

    let pages = ((g.str_data.len() as u64) / 65536 + 1) as u32;
    let mut mem_payload = Vec::new();
    uleb(&mut mem_payload, 1);
    mem_payload.push(0x00);
    uleb(&mut mem_payload, pages as u64);

    let mut data_payload = Vec::new();
    if g.str_data.is_empty() {
        uleb(&mut data_payload, 0);
    } else {
        uleb(&mut data_payload, 1);
        data_payload.push(0x00);
        data_payload.push(0x41);
        sleb(&mut data_payload, 0);
        data_payload.push(0x0B);
        uleb(&mut data_payload, g.str_data.len() as u64);
        data_payload.extend_from_slice(&g.str_data);
    }

    let mut type_payload = Vec::new();
    uleb(&mut type_payload, g.types.len() as u64);
    for (params, results) in &g.types {
        type_payload.push(0x60);
        uleb(&mut type_payload, params.len() as u64);
        type_payload.extend_from_slice(params);
        uleb(&mut type_payload, results.len() as u64);
        type_payload.extend_from_slice(results);
    }

    let mut out = Vec::new();
    out.extend_from_slice(&WASM_MAGIC);
    section(&mut out, 1, &type_payload);
    section(&mut out, 2, &import_payload);
    section(&mut out, 3, &func_payload);
    section(&mut out, 5, &mem_payload);
    section(&mut out, 7, &export_payload);
    section(&mut out, 10, &code_payload);
    section(&mut out, 11, &data_payload);
    Ok(out)
}

pub fn web_html(name: &str) -> String {
    format!(
        r##"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>{name}</title>
<style>
body {{ background: #101216; color: #d8dee9; font: 14px/1.5 Consolas, monospace; padding: 1.5em; margin: 0; }}
#out {{ white-space: pre-wrap; }}
canvas {{ display: block; background: #000; }}
</style>
</head>
<body>
<div id="out"></div>
<script>
const out = document.getElementById("out");
function w(s) {{ out.textContent += s; }}
let text = "";
const flush = () => {{ if (text) {{ w(text); w("\n"); text = ""; }} }};

/* ---------- WebGL immediate-mode renderer ---------- */
const glCanvas = document.createElement("canvas");
glCanvas.width = 640; glCanvas.height = 480;
glCanvas.style.display = "none";
document.body.appendChild(glCanvas);
const gl = glCanvas.getContext("webgl", {{ antialias: false }});
let glInited = false, glMode = 0, glVerts = [], glCR = 1, glCG = 1, glCB = 1, glAlive = true;

const glVS = `
attribute vec3 aPos; attribute vec3 aCol;
uniform vec2 uScreen;
varying vec3 vCol;
void main() {{
  gl_Position = vec4(aPos.x / uScreen.x * 2.0 - 1.0, aPos.y / uScreen.y * 2.0 - 1.0, aPos.z, 1.0);
  vCol = aCol;
}}`;
const glFS = `
precision mediump float;
varying vec3 vCol;
void main() {{ gl_FragColor = vec4(vCol, 1.0); }}`;

function makeShader(glCtx, type, src) {{
  const s = glCtx.createShader(type);
  glCtx.shaderSource(s, src);
  glCtx.compileShader(s);
  if (!glCtx.getShaderParameter(s, glCtx.COMPILE_STATUS)) throw new Error(glCtx.getShaderInfoLog(s));
  return s;
}}
let glProg, glBuf, glUScreen;
function glInit(w, h, title) {{
  if (glInited) return;
  glCanvas.width = w; glCanvas.height = h;
  glCanvas.style.display = "block";
  document.title = title;
  const vs = makeShader(gl, gl.VERTEX_SHADER, glVS);
  const fs = makeShader(gl, gl.FRAGMENT_SHADER, glFS);
  glProg = gl.createProgram();
  gl.attachShader(glProg, vs); gl.attachShader(glProg, fs);
  gl.linkProgram(glProg);
  if (!gl.getProgramParameter(glProg, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(glProg));
  gl.useProgram(glProg);
  glBuf = gl.createBuffer();
  glUScreen = gl.getUniformLocation(glProg, "uScreen");
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  glInited = true;
}}
function glFlushBatch() {{
  if (glVerts.length === 0) return;
  const data = new Float32Array(glVerts);
  const n = glVerts.length / 6;
  gl.useProgram(glProg);
  gl.uniform2f(glUScreen, glCanvas.width, glCanvas.height);
  gl.bindBuffer(gl.ARRAY_BUFFER, glBuf);
  gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
  const aPos = gl.getAttribLocation(glProg, "aPos");
  const aCol = gl.getAttribLocation(glProg, "aCol");
  gl.enableVertexAttribArray(aPos); gl.vertexAttribPointer(aPos, 3, gl.FLOAT, false, 24, 0);
  gl.enableVertexAttribArray(aCol); gl.vertexAttribPointer(aCol, 3, gl.FLOAT, false, 24, 12);
  const mode = glMode === 1 ? gl.LINES : gl.TRIANGLES;
  gl.drawArrays(mode, 0, n);
  glVerts = [];
}}
function glClear(r, g, b) {{
  if (!glInited) return;
  gl.flushBatch();
  gl.viewport(0, 0, glCanvas.width, glCanvas.height);
  gl.clearColor(r / 255, g / 255, b / 255, 1);
  gl.clear(gl.COLOR_BUFFER_BIT);
}}
function glColor(r, g, b) {{ glCR = r / 255; glCG = g / 255; glCB = b / 255; }}
function glBegin(mode) {{ glMode = mode === 1 ? 1 : 0; }}
function glVertex(x, y, z) {{ glVerts.push(x, y, z, glCR, glCG, glCB); }}
function glEnd() {{ glFlushBatch(); }}
function glSwap() {{ if (!glInited || !glAlive) return 0; glFlushBatch(); return 1; }}

/* ---------- WebGPU immediate-mode renderer ---------- */
const gpuCanvas = document.createElement("canvas");
gpuCanvas.width = 640; gpuCanvas.height = 480;
gpuCanvas.style.display = "none";
document.body.appendChild(gpuCanvas);
let gpuInited = false, gpuMode = 0, gpuVerts = [], gpuCR = 1, gpuCG = 1, gpuCB = 1;
let gpuDevice, gpuCtx, gpuPipeline, gpuBuf, gpuAlive = true, gpuFrame = 0;

const gpuVS = `
struct VSOut {{ @builtin(position) pos: vec4f, @location(0) col: vec3f }};
@vertex fn vs_main(@location(0) pos: vec3f, @location(1) col: vec3f) -> VSOut {{
  var o: VSOut;
  o.pos = vec4f(pos.x / uScreen.x * 2.0 - 1.0, pos.y / uScreen.y * 2.0 - 1.0, pos.z, 1.0);
  o.col = col;
  return o;
}}`;
const gpuFS = `
@fragment fn fs_main(@location(0) col: vec3f) -> @location(0) vec4f {{
  return vec4f(col, 1.0);
}}`;

async function gpuInitAsync(w, h, title) {{
  if (!navigator.gpu) {{ gpuAlive = false; return; }}
  gpuCanvas.width = w; gpuCanvas.height = h;
  gpuCanvas.style.display = "block";
  document.title = title;
  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) {{ gpuAlive = false; return; }}
  gpuDevice = await adapter.requestDevice();
  gpuCtx = gpuCanvas.getContext("webgpu");
  const format = navigator.gpu.getPreferredCanvasFormat();
  gpuCtx.configure({{ device: gpuDevice, format, alphaMode: "opaque" }});
  const module = gpuDevice.createShaderModule({{
    code: `
      struct VSOut {{ @builtin(position) pos: vec4f, @location(0) col: vec3f }};
      @vertex fn vs_main(@location(0) pos: vec3f, @location(1) col: vec3f) -> VSOut {{
        var o: VSOut;
        o.pos = vec4f(pos, 1.0);
        o.col = col;
        return o;
      }}
      @fragment fn fs_main(@location(0) col: vec3f) -> @location(0) vec4f {{
        return vec4f(col, 1.0);
      }}
    `
  }});
  gpuPipeline = gpuDevice.createRenderPipeline({{
    layout: "auto",
    vertex: {{ module, entryPoint: "vs_main", buffers: [{{ arrayStride: 24, attributes: [
      {{ shaderLocation: 0, offset: 0, format: "float32x3" }},
      {{ shaderLocation: 1, offset: 12, format: "float32x3" }}
    ]}}]}},
    fragment: {{ module, entryPoint: "fs_main", targets: [{{ format }}] }},
    primitive: {{ topology: "triangle-list" }}
  }});
  gpuBuf = gpuDevice.createBuffer({{ size: 65536, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST }});
  gpuInited = true;
}}
function gpuFlushBatch(topology) {{
  if (gpuVerts.length === 0) return;
  const data = new Float32Array(gpuVerts);
  gpuDevice.queue.writeBuffer(gpuBuf, 0, data);
  const enc = gpuDevice.createCommandEncoder();
  const pass = enc.beginRenderPass({{ colorAttachments: [{{ view: gpuCtx.getCurrentTexture().createView(),
    clearValue: {{ r: 0, g: 0, b: 0, a: 1 }}, loadOp: "load", storeOp: "store" }}] }});
  const pipe = gpuPipeline; /* re-create with topology if needed — simplified: always triangles */
  pass.setPipeline(gpuPipeline);
  pass.setVertexBuffer(0, gpuBuf);
  pass.draw(gpuVerts.length / 6);
  pass.end();
  gpuDevice.queue.submit([enc.finish()]);
  gpuVerts = [];
}}
function gpuClear(r, g, b) {{
  if (!gpuInited || !gpuAlive) return;
  gpuClearCol = [r / 255, g / 255, b / 255];
  gpuNeedsClear = true;
}}
let gpuClearCol = [0, 0, 0], gpuNeedsClear = false;
function gpuColor(r, g, b) {{ gpuCR = r / 255; gpuCG = g / 255; gpuCB = b / 255; }}
function gpuBegin(mode) {{ gpuMode = mode === 1 ? 1 : 0; }}
function gpuVertex(x, y, z) {{
  const nx = x / gpuCanvas.width * 2 - 1;
  const ny = y / gpuCanvas.height * 2 - 1;
  gpuVerts.push(nx, ny, z, gpuCR, gpuCG, gpuCB);
}}
function gpuEnd() {{ /* batch flushed on swap */ }}
function gpuSwap() {{
  if (!gpuInited || !gpuAlive) return 0;
  if (gpuNeedsClear) {{
    const enc = gpuDevice.createCommandEncoder();
    const pass = enc.beginRenderPass({{ colorAttachments: [{{ view: gpuCtx.getCurrentTexture().createView(),
      clearValue: {{ r: gpuClearCol[0], g: gpuClearCol[1], b: gpuClearCol[2], a: 1 }},
      loadOp: "clear", storeOp: "store" }}] }});
    pass.end();
    gpuDevice.queue.submit([enc.finish()]);
    gpuNeedsClear = false;
  }}
  if (gpuVerts.length > 0) gpuFlushBatch(gpuMode);
  return 1;
}}

(async () => {{
  const res = await fetch("{name}.wasm");
  const bytes = await res.arrayBuffer();
  let instance;
  const imports = {{ env: {{
    print_i: v => {{ text += v.toString(); }},
    print_f: v => {{ text += String(Number(v)); }},
    print_b: v => {{ text += v ? "true" : "false"; }},
    print_s: (p, l) => {{
      const mem = new Uint8Array(instance.exports.memory.buffer, p, l);
      text += new TextDecoder().decode(mem);
    }},
    print_sp: () => {{ text += " "; }},
    print_nl: flush,
    exit: c => {{ flush(); w("[iks] exit " + c); throw new Error("exit"); }},
    sin: Math.sin, cos: Math.cos, pow: Math.pow, rand: Math.random,
    /* WebGL */
    gfx_init: (w, h, sp, sl) => {{
      const mem = new Uint8Array(instance.exports.memory.buffer, sp, sl);
      glInit(w, h, new TextDecoder().decode(mem));
    }},
    gfx_clear: glClear,
    gfx_color: glColor,
    gfx_begin: glBegin,
    gfx_vertex: glVertex,
    gfx_end: glEnd,
    gfx_swap: glSwap,
    gfx_width: () => glCanvas.width,
    gfx_height: () => glCanvas.height,
    gfx_rect: (x, y, ww, hh) => {{ glBegin(3); glVertex(x, y + hh, 0); glVertex(x + ww, y, 0); glVertex(x, y, 0); glVertex(x, y + hh, 0); glVertex(x + ww, y + hh, 0); glVertex(x + ww, y, 0); glEnd(); }},
    gfx_circle: (cx, cy, r, segs) => {{ glBegin(3); for (let i = 0; i < segs; i++) {{ const a0 = i / segs * 6.283185307179586, a1 = (i + 1) / segs * 6.283185307179586; glVertex(cx, cy, 0); glVertex(cx + r * Math.cos(a0), cy + r * Math.sin(a0), 0); glVertex(cx + r * Math.cos(a1), cy + r * Math.sin(a1), 0); }} glEnd(); }},
    gfx_line: (x1, y1, x2, y2) => {{ glBegin(1); glVertex(x1, y1, 0); glVertex(x2, y2, 0); glEnd(); }},
    gfx_title: (sp, sl) => {{ const mem = new Uint8Array(instance.exports.memory.buffer, sp, sl); document.title = new TextDecoder().decode(mem); }},
    /* WebGPU */
    gpu_init: (w, h, sp, sl) => {{
      const mem = new Uint8Array(instance.exports.memory.buffer, sp, sl);
      gpuInitAsync(w, h, new TextDecoder().decode(mem));
    }},
    gpu_clear: gpuClear,
    gpu_color: gpuColor,
    gpu_begin: gpuBegin,
    gpu_vertex: gpuVertex,
    gpu_end: gpuEnd,
    gpu_swap: gpuSwap,
    gpu_width: () => gpuCanvas.width,
    gpu_height: () => gpuCanvas.height,
    gpu_rect: (x, y, ww, hh) => {{ gpuBegin(3); gpuVertex(x, y + hh, 0); gpuVertex(x + ww, y, 0); gpuVertex(x, y, 0); gpuVertex(x, y + hh, 0); gpuVertex(x + ww, y + hh, 0); gpuVertex(x + ww, y, 0); gpuEnd(); }},
    gpu_circle: (cx, cy, r, segs) => {{ gpuBegin(3); for (let i = 0; i < segs; i++) {{ const a0 = i / segs * 6.283185307179586, a1 = (i + 1) / segs * 6.283185307179586; gpuVertex(cx, cy, 0); gpuVertex(cx + r * Math.cos(a0), cy + r * Math.sin(a0), 0); gpuVertex(cx + r * Math.cos(a1), cy + r * Math.sin(a1), 0); }} gpuEnd(); }},
    gpu_line: (x1, y1, x2, y2) => {{ gpuBegin(1); gpuVertex(x1, y1, 0); gpuVertex(x2, y2, 0); gpuEnd(); }},
    gpu_title: (sp, sl) => {{ const mem = new Uint8Array(instance.exports.memory.buffer, sp, sl); document.title = new TextDecoder().decode(mem); }},
  }} }};
  const result = await WebAssembly.instantiate(bytes, imports);
  instance = result.instance;
  /* wait a tick for async WebGPU init if used */
  await new Promise(r => setTimeout(r, 50));
  const code = instance.exports.main();
  flush();
  w("[iks] main returned " + Number(code ?? 0));
}})().catch(e => {{ w("[runtime] " + e); }});
</script>
</body>
</html>
"##,
        name = name
    )
}

pub fn web_runner(name: &str) -> String {
    format!(
        r##"import {{ readFileSync }} from "fs";
const bytes = readFileSync(new URL("./{name}.wasm", import.meta.url));
let text = "";
const flush = () => {{ if (text) {{ process.stdout.write(text + "\n"); text = ""; }} }};
let instance;
const imports = {{ env: {{
  print_i: v => {{ text += v.toString(); }},
  print_f: v => {{ text += String(Number(v)); }},
  print_b: v => {{ text += v ? "true" : "false"; }},
  print_s: (p, l) => {{
    const mem = new Uint8Array(instance.exports.memory.buffer, p, l);
    text += new TextDecoder().decode(mem);
  }},
  print_sp: () => {{ text += " "; }},
  print_nl: flush,
  exit: c => {{ flush(); process.exit(c); }},
  sin: Math.sin, cos: Math.cos, pow: Math.pow, rand: Math.random,
  /* gfx/gpu stubs for node (no browser canvas) */
  gfx_init: () => {{}},
  gfx_clear: () => {{}},
  gfx_color: () => {{}},
  gfx_begin: () => {{}},
  gfx_vertex: () => {{}},
  gfx_end: () => {{}},
  gfx_swap: () => 1,
  gfx_width: () => 0,
  gfx_height: () => 0,
  gfx_rect: () => {{}},
  gfx_circle: () => {{}},
  gfx_line: () => {{}},
  gfx_title: () => {{}},
  gpu_init: () => {{}},
  gpu_clear: () => {{}},
  gpu_color: () => {{}},
  gpu_begin: () => {{}},
  gpu_vertex: () => {{}},
  gpu_end: () => {{}},
  gpu_swap: () => 1,
  gpu_width: () => 0,
  gpu_height: () => 0,
  gpu_rect: () => {{}},
  gpu_circle: () => {{}},
  gpu_line: () => {{}},
  gpu_title: () => {{}},
}} }};
const {{ instance: inst }} = await WebAssembly.instantiate(bytes, imports);
instance = inst;
const code = instance.exports.main();
flush();
process.exit(Number(code ?? 0));
"##,
        name = name
    )
}
