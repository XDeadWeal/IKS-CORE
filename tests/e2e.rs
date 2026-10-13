use iks_compiler::Compiler;

fn compile(src: &str) -> Result<String, String> {
    let mut c = Compiler::new();
    c.add_file("test.ik", src);
    c.compile().map_err(|d| c.render_all(&d))
}

#[test]
fn hello_world_generates_c() {
    let c = compile(
        "fn main() -> int {\n    print(\"hello\");\n    return 0;\n}\n",
    )
    .expect("should compile");
    assert!(c.contains("int main(void)"));
    assert!(c.contains("printf"));
    assert!(c.contains("return 0;"));
}

#[test]
fn functions_and_arithmetic() {
    let c = compile(
        "fn add(a: int, b: int) -> int { return a + b; }\nfn main() -> int { print(add(1, 2)); return 0; }\n",
    )
    .expect("should compile");
    assert!(c.contains("f_add"));
    assert!(!c.contains("ik_div"));
}

#[test]
fn missing_main_is_an_error() {
    let err = compile("fn foo() -> int { return 1; }\n").unwrap_err();
    assert!(err.contains("main"), "got: {}", err);
}

#[test]
fn type_mismatch_is_reported() {
    let err = compile("fn main() -> int { var x = 1 + \"a\"; return 0; }\n").unwrap_err();
    assert!(err.contains("cannot add"), "got: {}", err);
    assert!(err.contains("test.ik:1"), "got: {}", err);
}

#[test]
fn unknown_variable_is_reported() {
    let err = compile("fn main() -> int { return y; }\n").unwrap_err();
    assert!(err.contains("undefined variable"), "got: {}", err);
}

#[test]
fn let_is_immutable() {
    let err = compile("fn main() -> int { let x = 1; x = 2; return 0; }\n").unwrap_err();
    assert!(err.contains("immutable"), "got: {}", err);
}

#[test]
fn unterminated_string_is_reported() {
    let err = compile("fn main() -> int { print(\"oops); return 0; }\n").unwrap_err();
    assert!(err.contains("unterminated string"), "got: {}", err);
}

#[test]
fn break_outside_loop_is_reported() {
    let err = compile("fn main() -> int { break; return 0; }\n").unwrap_err();
    assert!(err.contains("'break'"), "got: {}", err);
}

#[test]
fn non_void_function_must_return() {
    let err = compile("fn main() -> int { var x = 1; }\n").unwrap_err();
    assert!(err.contains("must return"), "got: {}", err);
}

#[test]
fn empty_array_needs_annotation() {
    let err = compile("fn main() -> int { var a = []; return 0; }\n").unwrap_err();
    assert!(err.contains("empty array"), "got: {}", err);
}

#[test]
fn empty_array_with_annotation_is_ok() {
    let c = compile("fn main() -> int { var a: int[] = []; print(len(a)); return 0; }\n")
        .expect("should compile");
    assert!(c.contains("iks_arr"));
}

#[test]
fn if_else_while_generate_code() {
    let c = compile(
        "fn main() -> int {\n    var i = 0;\n    while (i < 3) {\n        if (i == 1) { print(i); } else { print(\"no\"); }\n        i = i + 1;\n    }\n    return 0;\n}\n",
    )
    .expect("should compile");
    assert!(c.contains("while"));
    assert!(c.contains("else"));
}

fn gcc_available() -> bool {
    std::process::Command::new("gcc")
        .arg("--version")
        .output()
        .is_ok()
}

fn link_gcc(cfile: &std::path::Path, exefile: &std::path::Path) -> bool {
    let mut cmd = std::process::Command::new("gcc");
    cmd.arg("-O2")
        .arg("-o")
        .arg(exefile)
        .arg(cfile)
        .arg("-lm")
        .arg("-lgdi32")
        .arg("-luser32")
        .arg("-lopengl32")
        .arg("-ld3d9")
        .arg("-ld3d10")
        .arg("-ld3d11")
        .arg("-ldxgi")
        .arg("-ld3dcompiler")
        .arg("-ld3d12")
        .arg("-lpdh");
    if let Some(sdk) = detect_vulkan_sdk() {
        cmd.arg(format!("-I{}/Include", sdk));
        cmd.arg(format!("{}/Lib/vulkan-1.lib", sdk));
        cmd.arg("-DIKS_VK=1");
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

fn detect_vulkan_sdk() -> Option<String> {
    let candidates: Vec<String> = {
        let mut v = Vec::new();
        if let Ok(sdk) = std::env::var("VULKAN_SDK") {
            v.push(sdk);
        }
        if let Ok(entries) = std::fs::read_dir("C:/VulkanSDK") {
            let mut dirs: Vec<String> = entries
                .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
                .map(|n| format!("C:/VulkanSDK/{}", n))
                .collect();
            dirs.sort();
            dirs.reverse();
            v.extend(dirs);
        }
        v
    };
    for sdk in candidates {
        let inc = format!("{}/Include/vulkan/vulkan.h", sdk);
        let lib = format!("{}/Lib/vulkan-1.lib", sdk);
        if std::path::Path::new(&inc).exists() && std::path::Path::new(&lib).exists() {
            return Some(sdk);
        }
    }
    None
}

fn unique_dir(tag: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("iks_e2e_{}_{}_{}", tag, std::process::id(), n));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_e2e(src: &str, input: &str) -> Option<String> {
    if !gcc_available() {
        eprintln!("gcc not found, skipping e2e test");
        return None;
    }
    let c = compile(src).expect("e2e source should compile");
    let dir = unique_dir("run");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    let status_ok = link_gcc(&cfile, &exefile);
    assert!(status_ok, "gcc should succeed");
    let mut child = std::process::Command::new(&exefile)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "program should exit 0");
    let _ = std::fs::remove_dir_all(&dir);
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

#[test]
fn e2e_hello() {
    let Some(out) = run_e2e("fn main() -> int { print(\"hello\", 1 + 1); return 0; }", "") else {
        return;
    };
    assert_eq!(out, "hello 2\n");
}

#[test]
fn e2e_fib_and_arrays() {
    let src = "
fn fib(n: int) -> int {
    if (n < 2) { return n; }
    return fib(n - 1) + fib(n - 2);
}

fn main() -> int {
    var xs = [fib(0), fib(1), fib(10)];
    print(xs);
    print(len(xs));
    var t = true;
    print(t, !t, 7 / 2, 7 % 2, 1.5 * 2);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "[0, 1, 55]\n3\ntrue false 3 1 3\n");
}

#[test]
fn e2e_strings_and_input() {
    let src = "
fn main() -> int {
    let name = str(10) + \"!\";
    print(name, len(name));
    var line = input(\"\");
    print(int(line) + 1);
    return 0;
}
";
    let Some(out) = run_e2e(src, "41\n") else {
        return;
    };
    assert_eq!(out, "10! 3\n42\n");
}

#[test]
fn e2e_runtime_bounds_check() {
    let src = "
fn main() -> int {
    var a = [1, 2];
    print(a[5]);
    return 0;
}
";
    if !gcc_available() {
        return;
    }
    let c = compile(src).unwrap();
    let dir = unique_dir("bounds");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    let ok = link_gcc(&cfile, &exefile);
    assert!(ok);
    let out = std::process::Command::new(&exefile).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("out of bounds"), "got: {}", stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_push_builds_dynamic_arrays() {
    let src = "
fn main() -> int {
    var xs: int[] = [];
    var i = 0;
    while (i < 4) {
        xs = push(xs, i * i);
        i = i + 1;
    }
    print(xs);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "[0, 1, 4, 9]\n");
}

#[test]
fn e2e_switch_int() {
    let src = "
fn main() -> int {
    var x = 2;
    switch (x) {
        case 1: { print(\"one\"); }
        case 2: { print(\"two\"); }
        case 3, 4: { print(\"three or four\"); }
        default: { print(\"other\"); }
    }
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "two\n");
}

#[test]
fn e2e_switch_fallthrough_default() {
    let src = "
fn main() -> int {
    var x = 99;
    switch (x) {
        case 1: { print(\"one\"); }
        default: { print(\"fallback\"); }
    }
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "fallback\n");
}

#[test]
fn e2e_switch_string() {
    let src = "
fn main() -> int {
    var cmd = \"jump\";
    switch (cmd) {
        case \"jump\": { print(\"jumping\"); }
        case \"run\": { print(\"running\"); }
        default: { print(\"idle\"); }
    }
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "jumping\n");
}

#[test]
fn e2e_switch_multi_pattern() {
    let src = "
fn main() -> int {
    for (i in 0..6) {
        switch (i) {
            case 0: { print(\"zero\"); }
            case 1, 2, 3: { print(\"small\"); }
            default: { print(\"big\"); }
        }
    }
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "zero\nsmall\nsmall\nsmall\nbig\nbig\n");
}

#[test]
fn e2e_switch_returns() {
    let src = "
fn classify(n: int) -> int {
    switch (n) {
        case 0: { return 100; }
        case 1: { return 200; }
        default: { return 300; }
    }
    return -1;
}
fn main() -> int {
    print(classify(0));
    print(classify(1));
    print(classify(42));
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "100\n200\n300\n");
}

#[test]
fn e2e_map_basic() {
    let src = "
fn main() -> int {
    var m: map = map_new();
    map_set(m, \"hp\", 100);
    map_set(m, \"name\", \"hero\");
    map_set(m, \"speed\", 3.5);
    map_set(m, \"alive\", true);
    print(map_get(m, \"hp\", 0));
    print(map_get(m, \"name\", \"\"));
    print(map_get(m, \"speed\", 0.0));
    print(map_get(m, \"alive\", false));
    print(map_get(m, \"missing\", -1));
    print(map_has(m, \"hp\"));
    print(map_has(m, \"xyz\"));
    print(map_len(m));
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "100\nhero\n3.5\ntrue\n-1\ntrue\nfalse\n4\n");
}

#[test]
fn e2e_map_remove_keys() {
    let src = "
fn main() -> int {
    var m: map = map_new();
    map_set(m, \"a\", 1);
    map_set(m, \"b\", 2);
    map_set(m, \"c\", 3);
    print(map_len(m));
    print(map_remove(m, \"b\"));
    print(map_remove(m, \"zzz\"));
    print(map_len(m));
    print(map_has(m, \"b\"));
    var ks = map_keys(m);
    print(len(ks));
    print(map_get(m, \"a\", 0) + map_get(m, \"c\", 0));
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "3\ntrue\nfalse\n2\nfalse\n2\n4\n");
}

#[test]
fn e2e_math_builtins() {
    let src = "
fn main() -> int {
    print(sin(0.0), cos(0.0), sqrt(16.0), abs(-2.5), floor(3.7));
    print(pow(2.0, 10.0));
    let r = rand();
    print(r >= 0.0 && r <= 1.0);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "0 1 4 2.5 3");
    assert_eq!(lines[1], "1024");
    assert_eq!(lines[2], "true");
}

#[test]
fn push_expects_an_array() {
    let err = compile("fn main() -> int { var x = push(1, 2); return 0; }\n").unwrap_err();
    assert!(err.contains("'push' expects an array"), "got: {}", err);
}

#[test]
fn unknown_builtin_is_reported() {
    let err = compile("fn main() -> int { return gfx_openn(1); }\n").unwrap_err();
    assert!(err.contains("unknown function"), "got: {}", err);
}

#[test]
fn e2e_gfx_program_links() {
    if !gcc_available() {
        return;
    }
    let src = "
fn main() -> int {
    gfx_open(320, 240, \"gfx test\");
    var frames = 0;
    while (gfx_poll() && frames < 5) {
        gfx_clear(10, 20, 30);
        gfx_pixel(5, 5, 255, 0, 0);
        gfx_add(6, 5, 100, 100, 100);
        gfx_line(0, 0, 100, 100, 0, 255, 0);
        gfx_present();
        frames = frames + 1;
    }
    return 0;
}
";
    let c = compile(src).expect("gfx program should compile");
    assert!(c.contains("iks_gfx_open"));
    let dir = unique_dir("gfx");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    let ok = link_gcc(&cfile, &exefile);
    assert!(ok, "gfx program should link against gdi32/user32");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn gfx_key_down_takes_an_int() {
    let err = compile("fn main() -> int { return gfx_key_down(\"a\"); }\n").unwrap_err();
    assert!(err.contains("type mismatch"), "got: {}", err);
}

#[test]
fn structs_declare_fields_and_literal() {
    let c = compile(
        "struct Vec2 { x: float, y: float }\nfn main() -> int { var v = Vec2 { x: 1.5, y: 2.5 }; print(v.x); return 0; }\n",
    )
    .expect("should compile");
    assert!(c.contains("typedef struct S_Vec2"));
    assert!(c.contains("f_x"));
    assert!(c.contains("(S_Vec2)"));
}

#[test]
fn struct_field_access_and_assign() {
    let src = "
struct P { x: int, y: int }
fn main() -> int {
    var p = P { x: 1, y: 2 };
    p.x = p.x + 10;
    print(p.x, p.y);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "11 2\n");
}

#[test]
fn struct_unknown_field_is_reported() {
    let err = compile(
        "struct P { x: int }\nfn main() -> int { var p = P { z: 1 }; return 0; }\n",
    )
    .unwrap_err();
    assert!(err.contains("has no field"), "got: {}", err);
}

#[test]
fn struct_missing_field_is_reported() {
    let err = compile(
        "struct P { x: int, y: int }\nfn main() -> int { var p = P { x: 1 }; return 0; }\n",
    )
    .unwrap_err();
    assert!(err.contains("missing field"), "got: {}", err);
}

#[test]
fn extern_fn_emits_unmangled_prototype() {
    let c = compile(
        "extern \"C\" fn puts(s: string) -> int;\nfn main() -> int { puts(\"hi\"); return 0; }\n",
    )
    .expect("should compile");
    assert!(c.contains("int64_t puts(char* s);") || c.contains("puts("), "got: {}", c);
    assert!(!c.contains("f_puts"), "extern must not be mangled: {}", c);
}

#[test]
fn e2e_extern_calls_real_c() {
    if !gcc_available() {
        return;
    }
    let src = "
extern \"C\" fn iks_helper_add(a: int, b: int) -> int;
fn main() -> int {
    print(iks_helper_add(40, 2));
    return 0;
}
";
    let c = compile(src).expect("extern program should compile");
    let dir = unique_dir("ext");
    let cfile = dir.join("prog.c");
    let helper = dir.join("helper.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    std::fs::write(
        &helper,
        "long long iks_helper_add(long long a, long long b) { return a + b; }\n",
    )
    .unwrap();
    let ok = std::process::Command::new("gcc")
        .arg("-O2")
        .arg("-o")
        .arg(&exefile)
        .arg(&cfile)
        .arg(&helper)
        .arg("-lm")
        .arg("-lgdi32")
        .arg("-luser32")
        .arg("-lopengl32")
        .arg("-ld3d11")
        .arg("-ldxgi")
        .arg("-ld3dcompiler")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(ok, "extern program should link with the helper C file");
    let out = std::process::Command::new(&exefile).output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "42\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ptr_and_nil_compare() {
    let src = "
fn main() -> int {
    var p: ptr = nil;
    print(p == nil);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "true\n");
}

fn compile_test_mode(src: &str) -> Result<String, String> {
    let mut c = Compiler::new();
    c.add_file("test.ik", src);
    c.compile_test().map_err(|d| c.render_all(&d))
}

#[test]
fn test_mode_builds_harness_without_main() {
    let c = compile_test_mode("fn test_one() -> bool { return 1 + 1 == 2; }\n")
        .expect("should compile in test mode");
    assert!(c.contains("int main(void)"));
    assert!(c.contains("f_test_one"));
    assert!(c.contains("all tests passed"));
}

#[test]
fn test_mode_still_requires_main_in_normal_mode() {
    let err = compile("fn test_one() -> bool { return true; }\n").unwrap_err();
    assert!(err.contains("main"), "got: {}", err);
}

fn run_test_mode(src: &str) -> Option<(bool, String, String)> {
    if !gcc_available() {
        return None;
    }
    let c = compile_test_mode(src).expect("test-mode source should compile");
    let dir = unique_dir("tst");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    assert!(link_gcc(&cfile, &exefile), "gcc should succeed");
    let out = std::process::Command::new(&exefile).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    Some((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    ))
}

#[test]
fn e2e_test_mode_passing() {
    let src = "
fn add(a: int, b: int) -> int { return a + b; }
fn test_add() -> bool { return add(2, 3) == 5; }
fn test_void() { assert(1 == 1, \"must hold\"); }
";
    let Some((ok, stdout, _)) = run_test_mode(src) else {
        return;
    };
    assert!(ok, "stdout: {} ", stdout);
    assert!(stdout.contains("running 2 test(s)"), "got: {}", stdout);
    assert!(stdout.contains("test_add: ok"), "got: {}", stdout);
    assert!(stdout.contains("test_void: ok"), "got: {}", stdout);
    assert!(stdout.contains("all tests passed"), "got: {}", stdout);
}

#[test]
fn e2e_test_mode_failing_bool() {
    let src = "fn test_fail() -> bool { return 1 == 2; }\n";
    let Some((ok, stdout, _)) = run_test_mode(src) else {
        return;
    };
    assert!(!ok);
    assert!(stdout.contains("test_fail: FAILED"), "got: {}", stdout);
    assert!(stdout.contains("some tests FAILED"), "got: {}", stdout);
}

#[test]
fn e2e_assert_failure_aborts() {
    let src = "
fn main() -> int {
    assert(1 == 2, \"one is not two\");
    return 0;
}
";
    if !gcc_available() {
        return;
    }
    let c = compile(src).unwrap();
    let dir = unique_dir("assert");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    assert!(link_gcc(&cfile, &exefile));
    let out = std::process::Command::new(&exefile).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("assertion failed"), "got: {}", stderr);
    assert!(stderr.contains("one is not two"), "got: {}", stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_assert_passes_when_true() {
    let src = "
fn main() -> int {
    assert(2 + 2 == 4, \"math works\");
    print(\"ok\");
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "ok\n");
}

fn compile_with_imports(files: &[(&str, &str)]) -> Result<String, String> {
    let mut c = Compiler::new();
    for (name, text) in files {
        c.add_file(*name, *text);
    }
    c.compile().map_err(|d| c.render_all(&d))
}

#[test]
fn import_merges_functions() {
    let dir = unique_dir("imp");
    let util = dir.join("util.ik");
    std::fs::write(&util, "fn double(n: int) -> int { return n * 2; }\n").unwrap();
    let main_path = dir.join("main.ik").to_string_lossy().to_string();
    let c = compile_with_imports(&[
        (
            main_path.as_str(),
            "import \"util.ik\";\nfn main() -> int { print(double(21)); return 0; }\n",
        ),
    ])
    .expect("imported program should compile");
    assert!(c.contains("f_double"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_import_runs() {
    if !gcc_available() {
        return;
    }
    let dir = unique_dir("imprun");
    std::fs::write(dir.join("util.ik"), "fn double(n: int) -> int { return n * 2; }\n").unwrap();
    let main_path = dir.join("main.ik").to_string_lossy().to_string();
    let c = compile_with_imports(&[(
        main_path.as_str(),
        "import \"util.ik\";\nfn main() -> int { print(double(21)); return 0; }\n",
    )])
    .expect("imported program should compile");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    assert!(link_gcc(&cfile, &exefile));
    let out = std::process::Command::new(&exefile).output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "42\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_import_is_reported() {
    let err = compile_with_imports(&[("main.ik", "import \"nope.ik\";\nfn main() -> int { return 0; }\n")])
        .unwrap_err();
    assert!(err.contains("cannot read imported file"), "got: {}", err);
}

#[test]
fn duplicate_function_across_files_is_reported() {
    let dir = unique_dir("dup");
    std::fs::write(dir.join("a.ik"), "fn shared() -> int { return 1; }\n").unwrap();
    let main_path = dir.join("main.ik").to_string_lossy().to_string();
    let err = compile_with_imports(&[
        (main_path.as_str(), "import \"a.ik\";\nfn shared() -> int { return 2; }\nfn main() -> int { return 0; }\n"),
    ])
    .unwrap_err();
    assert!(err.contains("already defined"), "got: {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_gl_program_links() {
    if !gcc_available() {
        return;
    }
    let src = "
fn main() -> int {
    gl_init(320, 240, \"gl test\");
    gl_view(0, 0, 320, 240);
    gl_proj(60.0, 1.333, 0.1, 100.0);
    var frames = 0;
    while (gl_swap() && frames < 3) {
        gl_clear(20, 20, 40);
        gl_push();
        gl_translate(0.0, 0.0, -3.0);
        gl_rotate(45.0, 0.0, 1.0, 0.0);
        gl_color(255, 0, 0, 255);
        gl_begin(3);
        gl_vertex(-0.5, -0.5, 0.0);
        gl_vertex(0.5, -0.5, 0.0);
        gl_vertex(0.0, 0.5, 0.0);
        gl_end();
        gl_pop();
        frames = frames + 1;
    }
    return 0;
}
";
    let c = compile(src).expect("gl program should compile");
    assert!(c.contains("iks_gl_init"));
    assert!(c.contains("iks_gl_swap"));
    let dir = unique_dir("gl");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    assert!(link_gcc(&cfile, &exefile), "gl program should link against opengl32");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_d3d_program_links() {
    if !gcc_available() {
        return;
    }
    let src = "
fn main() -> int {
    d3d_init(320, 240, \"d3d test\");
    var frames = 0;
    while (d3d_present() && frames < 3) {
        d3d_clear(10, 10, 30);
        var verts = [
            0.0, 0.5, 0.5, 1.0, 0.0, 0.0,
            -0.5, -0.5, 0.5, 0.0, 1.0, 0.0,
            0.5, -0.5, 0.5, 0.0, 0.0, 1.0
        ];
        d3d_draw(verts);
        frames = frames + 1;
    }
    return 0;
}
";
    let c = compile(src).expect("d3d program should compile");
    assert!(c.contains("iks_d3d_init"));
    assert!(c.contains("iks_d3d_draw"));
    let dir = unique_dir("d3d");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    assert!(link_gcc(&cfile, &exefile), "d3d program should link against d3d11");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_d3d_immediate_mode_links() {
    if !gcc_available() {
        return;
    }
    let src = "
fn main() -> int {
    d3d_init(320, 240, \"d3d immediate\");
    var frames = 0;
    while (d3d_present() && frames < 3) {
        d3d_clear(20, 20, 40);
        d3d_color(255, 0, 0);
        d3d_begin(3);
        d3d_vertex(160.0, 60.0, 0.0);
        d3d_vertex(100.0, 180.0, 0.0);
        d3d_vertex(220.0, 180.0, 0.0);
        d3d_end();
        d3d_color(0, 255, 0);
        d3d_line(0.0, 0.0, 320.0, 240.0);
        d3d_color(0, 128, 255);
        d3d_circle(160.0, 120.0, 50.0, 32);
        frames = frames + 1;
    }
    return 0;
}
";
    let c = compile(src).expect("d3d immediate mode should compile");
    assert!(c.contains("iks_d3d_begin"));
    assert!(c.contains("iks_d3d_vertex"));
    assert!(c.contains("iks_d3d_line"));
    assert!(c.contains("iks_d3d_circle"));
    let dir = unique_dir("d3d_imm");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    assert!(link_gcc(&cfile, &exefile), "d3d immediate mode program should link");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_dx12_program_links() {
    if !gcc_available() {
        return;
    }
    let src = "
fn main() -> int {
    dx12_init(320, 240, \"dx12 test\");
    var frames = 0;
    while (dx12_swap() && frames < 3) {
        dx12_clear(20, 20, 40);
        dx12_color(255, 0, 0);
        dx12_begin(3);
        dx12_vertex(160.0, 60.0, 0.0);
        dx12_vertex(100.0, 180.0, 0.0);
        dx12_vertex(220.0, 180.0, 0.0);
        dx12_end();
        dx12_color(0, 255, 0);
        dx12_line(0.0, 0.0, 320.0, 240.0);
        frames = frames + 1;
    }
    return 0;
}
";
    let c = compile(src).expect("dx12 program should compile");
    assert!(c.contains("iks_d12_init"));
    assert!(c.contains("iks_d12_swap"));
    assert!(c.contains("iks_d12_begin"));
    assert!(c.contains("iks_d12_vertex"));
    let dir = unique_dir("dx12");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    assert!(link_gcc(&cfile, &exefile), "dx12 program should link against d3d12");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_d3d9_program_links() {
    if !gcc_available() {
        return;
    }
    let src = "
fn main() -> int {
    d3d9_init(320, 240, \"d3d9 test\");
    var frames = 0;
    while (d3d9_swap() && frames < 3) {
        d3d9_clear(20, 20, 40);
        d3d9_color(255, 0, 0);
        d3d9_begin(3);
        d3d9_vertex(160.0, 60.0, 0.0);
        d3d9_vertex(100.0, 180.0, 0.0);
        d3d9_vertex(220.0, 180.0, 0.0);
        d3d9_end();
        d3d9_color(0, 255, 0);
        d3d9_line(0.0, 0.0, 320.0, 240.0);
        frames = frames + 1;
    }
    return 0;
}
";
    let c = compile(src).expect("d3d9 program should compile");
    assert!(c.contains("iks_d9_init"));
    assert!(c.contains("iks_d9_swap"));
    let dir = unique_dir("d3d9");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    assert!(link_gcc(&cfile, &exefile), "d3d9 program should link against d3d9");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_dx10_program_links() {
    if !gcc_available() {
        return;
    }
    let src = "
fn main() -> int {
    dx10_init(320, 240, \"dx10 test\");
    var frames = 0;
    while (dx10_swap() && frames < 3) {
        dx10_clear(20, 20, 40);
        dx10_color(255, 0, 0);
        dx10_begin(3);
        dx10_vertex(160.0, 60.0, 0.0);
        dx10_vertex(100.0, 180.0, 0.0);
        dx10_vertex(220.0, 180.0, 0.0);
        dx10_end();
        dx10_color(0, 255, 0);
        dx10_line(0.0, 0.0, 320.0, 240.0);
        frames = frames + 1;
    }
    return 0;
}
";
    let c = compile(src).expect("dx10 program should compile");
    assert!(c.contains("iks_d10_init"));
    assert!(c.contains("iks_d10_swap"));
    let dir = unique_dir("dx10");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    assert!(link_gcc(&cfile, &exefile), "dx10 program should link against d3d10");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_webgl_wasm_compiles() {
    let src = "
fn main() -> int {
    gl_init(320, 240, \"webgl test\");
    var frames = 0;
    while (gl_swap() && frames < 3) {
        gl_clear(20, 20, 40);
        gl_color(255, 0, 0, 255);
        gl_begin(3);
        gl_vertex(160.0, 60.0, 0.0);
        gl_vertex(100.0, 180.0, 0.0);
        gl_vertex(220.0, 180.0, 0.0);
        gl_end();
        frames = frames + 1;
    }
    return 0;
}
";
    let mut comp = iks_compiler::Compiler::new();
    comp.add_file("test.iks".to_string(), src.to_string());
    let bytes = comp.compile_wasm().expect("webgl program should compile to wasm");
    assert!(bytes.len() > 8);
    assert_eq!(&bytes[0..8], &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn e2e_webgpu_wasm_compiles() {
    let src = "
fn main() -> int {
    webgpu_init(320, 240, \"webgpu test\");
    var frames = 0;
    while (webgpu_swap() && frames < 3) {
        webgpu_clear(20, 20, 40);
        webgpu_color(0, 255, 0, 255);
        webgpu_begin(3);
        webgpu_vertex(160.0, 60.0, 0.0);
        webgpu_vertex(100.0, 180.0, 0.0);
        webgpu_vertex(220.0, 180.0, 0.0);
        webgpu_end();
        frames = frames + 1;
    }
    return 0;
}
";
    let mut comp = iks_compiler::Compiler::new();
    comp.add_file("test.iks".to_string(), src.to_string());
    let bytes = comp.compile_wasm().expect("webgpu program should compile to wasm");
    assert!(bytes.len() > 8);
    assert_eq!(&bytes[0..8], &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn e2e_vk_program_links() {
    if !gcc_available() {
        return;
    }
    let src = "
fn main() -> int {
    vk_init(320, 240, \"vk test\");
    var frames = 0;
    while (vk_swap() && frames < 3) {
        vk_clear(20, 20, 40);
        vk_color(255, 0, 0);
        vk_begin(3);
        vk_vertex(160.0, 60.0, 0.0);
        vk_vertex(100.0, 180.0, 0.0);
        vk_vertex(220.0, 180.0, 0.0);
        vk_end();
        vk_color(0, 255, 0);
        vk_line(0.0, 0.0, 320.0, 240.0);
        frames = frames + 1;
    }
    return 0;
}
";
    let c = compile(src).expect("vk program should compile");
    assert!(c.contains("iks_vk_init"));
    assert!(c.contains("iks_vk_swap"));
    assert!(c.contains("iks_vk_begin"));
    let dir = unique_dir("vk");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    assert!(link_gcc(&cfile, &exefile), "vk program should link against vulkan-1");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn gl_color_takes_four_args() {
    let err = compile("fn main() -> int { gl_color(1, 2, 3); return 0; }\n").unwrap_err();
    assert!(err.contains("argument"), "got: {}", err);
}

#[test]
fn gl_begin_unknown_mode_defaults_to_triangles() {
    let c = compile(
        "fn main() -> int { gl_init(320, 240, \"t\"); gl_begin(3); gl_end(); return 0; }\n",
    )
    .expect("should compile");
    assert!(c.contains("iks_gl_begin(3)"));
}

#[test]
fn e2e_gl_fur_benchmark_example_compiles() {
    if !gcc_available() {
        return;
    }
    let src = std::fs::read_to_string("examples/bench_fur.ik").expect("bench_fur.ik should exist");
    let c = compile(&src).expect("fur benchmark should compile");
    assert!(c.contains("iks_gl_init"));
    assert!(c.contains("iks_gfx_time"));
    assert!(c.contains("iks_gl_vertex"));
    let dir = unique_dir("fur");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    assert!(link_gcc(&cfile, &exefile), "fur benchmark should link");
    let _ = std::fs::remove_dir_all(&dir);
}

fn compile_wasm_bin(src: &str) -> Result<Vec<u8>, String> {
    let mut c = Compiler::new();
    c.add_file("test.ik", src);
    c.compile_wasm().map_err(|d| c.render_all(&d))
}

fn node_available() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_ok()
}

fn run_wasm(src: &str) -> Option<(bool, String)> {
    if !node_available() {
        eprintln!("node not found, skipping wasm e2e test");
        return None;
    }
    let bytes = compile_wasm_bin(src).expect("wasm source should compile");
    let dir = unique_dir("wasm");
    std::fs::write(dir.join("prog.wasm"), &bytes).unwrap();
    let runner_path = dir.join("prog.run.mjs");
    std::fs::write(&runner_path, iks_compiler::wasm::web_runner("prog")).unwrap();
    let out = std::process::Command::new("node")
        .arg(&runner_path)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    Some((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
    ))
}

#[test]
fn wasm_binary_has_magic_header() {
    let bytes = compile_wasm_bin("fn main() -> int { return 0; }\n").expect("should compile");
    assert_eq!(&bytes[0..4], b"\0asm");
    assert_eq!(&bytes[4..8], &[1, 0, 0, 0]);
}

#[test]
fn wasm_rejects_arrays() {
    let err =
        compile_wasm_bin("fn main() -> int { var a = [1, 2]; print(a[0]); return 0; }\n")
            .unwrap_err();
    assert!(err.contains("arrays are not supported"), "got: {}", err);
}

#[test]
fn wasm_rejects_structs() {
    let err = compile_wasm_bin(
        "struct P { x: int }\nfn main() -> int { var p = P{x: 1}; return p.x; }\n",
    )
    .unwrap_err();
    assert!(err.contains("structs are not supported"), "got: {}", err);
}

#[test]
fn wasm_rejects_extern() {
    let err = compile_wasm_bin(
        "extern \"C\" fn puts(s: string) -> int;\nfn main() -> int { return 0; }\n",
    )
    .unwrap_err();
    assert!(err.contains("extern function"), "got: {}", err);
}

#[test]
fn wasm_rejects_string_vars() {
    let err = compile_wasm_bin("fn main() -> int { var s = \"hi\"; return 0; }\n").unwrap_err();
    assert!(
        err.contains("string values are only supported"),
        "got: {}",
        err
    );
}

#[test]
fn wasm_rejects_nil() {
    let err = compile_wasm_bin("fn main() -> int { var p = nil; return 0; }\n").unwrap_err();
    assert!(err.contains("'nil' is not supported"), "got: {}", err);
}

#[test]
fn e2e_wasm_hello_print() {
    let Some((ok, out)) =
        run_wasm("fn main() -> int { print(\"hello\", 42, 1.5, true); return 0; }\n")
    else {
        return;
    };
    assert!(ok, "stdout: {}", out);
    assert_eq!(out, "hello 42 1.5 true\n");
}

#[test]
fn e2e_wasm_fib_and_comparisons() {
    let src = "
fn fib(n: int) -> int {
    if (n < 2) { return n; }
    return fib(n - 1) + fib(n - 2);
}

fn main() -> int {
    print(fib(20));
    print(7 > 3, 7 < 3, 7 >= 7, 7 <= 6, 7 == 7, 7 != 7);
    print(1 == 2 || 3 > 2, 1 == 1 && 2 < 3, !true);
    return 0;
}
";
    let Some((ok, out)) = run_wasm(src) else {
        return;
    };
    assert!(ok, "stdout: {}", out);
    assert_eq!(
        out,
        "6765\ntrue false true false true false\ntrue true false\n"
    );
}

#[test]
fn e2e_wasm_math_and_control_flow() {
    let src = "
fn main() -> int {
    print(sin(0.0), abs(-2.5), sqrt(16.0), floor(2.9));
    print(-5 / 2, -5 % 2, 2 + 3 * 4, (2 + 3) * 4);
    var i = 0;
    var sum = 0;
    while (i < 5) {
        if (i == 2) { i = i + 1; continue; }
        sum = sum + i;
        i = i + 1;
    }
    print(sum);
    return 0;
}
";
    let Some((ok, out)) = run_wasm(src) else {
        return;
    };
    assert!(ok, "stdout: {}", out);
    assert_eq!(out, "0 2.5 4 2\n-2 -1 14 20\n8\n");
}

#[test]
fn e2e_wasm_assert_pass_and_fail() {
    let Some((ok, out)) = run_wasm(
        "fn main() -> int { assert(1 + 1 == 2, \"math works\"); print(\"ok\"); return 0; }\n",
    ) else {
        return;
    };
    assert!(ok, "stdout: {}", out);
    assert_eq!(out, "ok\n");

    let Some((ok, out)) =
        run_wasm("fn main() -> int { assert(1 == 2, \"one is not two\"); return 0; }\n")
    else {
        return;
    };
    assert!(!ok, "assert failure should exit non-zero, stdout: {}", out);
    assert!(
        out.contains("assertion failed: one is not two"),
        "got: {}",
        out
    );
}

#[test]
fn e2e_wasm_exit_code() {
    let Some((ok, out)) = run_wasm("fn main() -> int { print(\"bye\"); return 3; }\n") else {
        return;
    };
    assert!(!ok, "return 3 should be a non-zero process exit");
    assert_eq!(out, "bye\n");
}

#[test]
fn e2e_wasm_imports_run() {
    if !node_available() {
        return;
    }
    let dir = unique_dir("wasmimp");
    std::fs::write(dir.join("util.ik"), "fn double(n: int) -> int { return n * 2; }\n").unwrap();
    let main_path = dir.join("main.ik").to_string_lossy().to_string();
    let mut c = Compiler::new();
    c.add_file(
        main_path,
        "import \"util.ik\";\nfn main() -> int { print(double(21)); return 0; }\n",
    );
    let bytes = c
        .compile_wasm()
        .map_err(|d| c.render_all(&d))
        .expect("imported program should compile to wasm");
    std::fs::write(dir.join("prog.wasm"), &bytes).unwrap();
    let runner_path = dir.join("prog.run.mjs");
    std::fs::write(&runner_path, iks_compiler::wasm::web_runner("prog")).unwrap();
    let out = std::process::Command::new("node")
        .arg(&runner_path)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "42\n");
}

#[test]
fn e2e_bitwise_operators() {
    let src = "
fn main() -> int {
    print(6 & 3, 6 | 3, 6 ^ 3, ~0, 1 << 4, 16 >> 2);
    var x = 12;
    x = x & 10;
    print(x);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "2 7 5 -1 16 4\n8\n");
}

#[test]
fn e2e_compound_assignment() {
    let src = "
fn main() -> int {
    var x = 10;
    x += 5;
    print(x);
    x -= 3;
    print(x);
    x *= 2;
    print(x);
    x /= 4;
    print(x);
    x %= 3;
    print(x);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "15\n12\n24\n6\n0\n");
}

#[test]
fn e2e_for_c_style() {
    let src = "
fn main() -> int {
    var sum = 0;
    for (var i = 0; i < 5; i += 1) {
        sum += i;
    }
    print(sum);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "10\n");
}

#[test]
fn e2e_for_range() {
    let src = "
fn main() -> int {
    var sum = 0;
    for (i in 0..5) {
        sum += i;
    }
    print(sum);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "10\n");
}

#[test]
fn e2e_bitwise_with_for_and_compound() {
    let src = "
fn hash(a: int, b: int) -> int {
    var n = a * 374761393 + b * 668265263;
    n = n ^ (n >> 13);
    n = n * 1274126177;
    n = n ^ (n >> 16);
    return n & 0x7fffffff;
}

fn main() -> int {
    var total = 0;
    for (i in 0..10) {
        total += hash(i, i * 2) & 255;
    }
    print(total >= 0);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "true\n");
}

#[test]
fn bitwise_on_float_is_error() {
    let err = compile("fn main() -> int { var x = 1.0 & 2; return 0; }\n").unwrap_err();
    assert!(err.contains("bitwise"), "got: {}", err);
}

#[test]
fn shift_precedence_is_correct() {
    let src = "
fn main() -> int {
    print(1 + 2 << 3);
    print(1 << 2 + 3);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "24\n32\n");
}

#[test]
fn bitnot_precedence() {
    let src = "
fn main() -> int {
    print(~5 + 1);
    print(~(5 + 1));
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "-5\n-7\n");
}

#[test]
fn e2e_extended_math_builtins() {
    let src = "
fn main() -> int {
    print(min(3, 7));
    print(max(3, 7));
    print(clamp(15, 0, 10));
    print(clamp(-5, 0, 10));
    print(clamp(5, 0, 10));
    print(lerp(0.0, 10.0, 0.5));
    print(round(2.5));
    print(ceil(2.1));
    print(sign(-5.0));
    print(sign(5.0));
    print(sign(0.0));
    print(hypot(3.0, 4.0));
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "3\n7\n10\n0\n5\n5\n3\n3\n-1\n1\n0\n5\n");
}

#[test]
fn e2e_string_builtins() {
    let src = "
fn main() -> int {
    var s = \"Hello, World!\";
    print(char_at(s, 0));
    print(substr(s, 0, 5));
    print(str_find(s, \"World\"));
    print(str_find(s, \"xyz\"));
    print(str_replace(s, \"World\", \"IKS\"));
    print(str_upper(s));
    print(str_lower(s));
    print(str_trim(\"  hi  \"));
    print(str_starts(s, \"Hello\"));
    print(str_starts(s, \"xyz\"));
    print(str_ends(s, \"!\"));
    print(str_cmp(\"abc\", \"abc\"));
    print(str_cmp(\"abc\", \"abd\"));
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "H\nHello\n7\n-1\nHello, IKS!\nHELLO, WORLD!\nhello, world!\nhi\ntrue\nfalse\ntrue\n0\n-1\n");
}

#[test]
fn e2e_str_split_join() {
    let src = "
fn main() -> int {
    var parts = str_split(\"a,b,c\", \",\");
    print(parts);
    print(len(parts));
    var joined = str_join(parts, \"-\");
    print(joined);
    var empty = str_split(\"no-sep-here\", \",\");
    print(len(empty));
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "[a, b, c]\n3\na-b-c\n1\n");
}

#[test]
fn e2e_array_builtins() {
    let src = "
fn main() -> int {
    var arr: int[] = [];
    arr = push(arr, 10);
    arr = push(arr, 20);
    arr = push(arr, 30);
    print(arr);
    print(pop(arr));
    print(arr_remove(arr, 0));
    print(arr_insert(arr, 1, 15));
    print(arr_slice(arr, 0, 2));
    print(arr_reverse(arr));
    var tmp: int[] = [];
    tmp = push(tmp, 30);
    tmp = push(tmp, 10);
    print(arr_sort(tmp));
    print(arr_index_of(arr, 20));
    print(arr_index_of(arr, 99));
    print(arr_contains(arr, 20));
    print(arr_contains(arr, 99));
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "[10, 20, 30]\n[10, 20]\n[20, 30]\n[10, 15, 20, 30]\n[10, 20]\n[30, 20, 10]\n[10, 30]\n1\n-1\ntrue\nfalse\n");
}

#[test]
fn e2e_file_io() {
    let src = "
fn main() -> int {
    var path = \"test_io_e2e.txt\";
    print(file_write(path, \"hello file\"));
    print(file_exists(path));
    var content = file_read(path);
    print(content);
    print(file_delete(path));
    print(file_exists(path));
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "true\ntrue\nhello file\ntrue\nfalse\n");
}

#[test]
fn e2e_rand_range_is_deterministic_bounds() {
    let src = "
fn main() -> int {
    var i = 0;
    while (i < 100) {
        var v = rand_range(5, 10);
        if (v < 5 || v > 10) {
            print(\"out of range\");
            return 1;
        }
        i += 1;
    }
    print(\"ok\");
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "ok\n");
}

#[test]
fn e2e_gl_rect_circle_line_link() {
    if !gcc_available() {
        eprintln!("gcc not found, skipping e2e test");
        return;
    }
    let src = "
fn main() -> int {
    gl_init(640, 480, \"test\");
    gl_clear(0, 0, 0);
    gl_color(255, 0, 0, 255);
    gl_rect(10.0, 10.0, 100.0, 50.0);
    gl_color(0, 255, 0, 255);
    gl_circle(320.0, 240.0, 50.0, 32);
    gl_color(0, 0, 255, 255);
    gl_line(0.0, 0.0, 640.0, 480.0);
    return 0;
}
";
    let c = compile(src).expect("should compile");
    let dir = unique_dir("glrect");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    let ok = link_gcc(&cfile, &exefile);
    assert!(ok, "gl_rect/circle/line program should link");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_gfx_mouse_button_links() {
    if !gcc_available() {
        eprintln!("gcc not found, skipping e2e test");
        return;
    }
    let src = "
fn main() -> int {
    gfx_open(320, 240, \"test\");
    while (gfx_poll()) {
        if (gfx_mouse_button(0)) {
            print(\"left\");
        }
        if (gfx_mouse_button(1)) {
            print(\"right\");
        }
        if (gfx_mouse_button(2)) {
            print(\"middle\");
        }
        gfx_present();
    }
    return 0;
}
";
    let c = compile(src).expect("should compile");
    let dir = unique_dir("mousebtn");
    let cfile = dir.join("prog.c");
    let exefile = dir.join("prog.exe");
    std::fs::write(&cfile, &c).unwrap();
    let ok = link_gcc(&cfile, &exefile);
    assert!(ok, "gfx_mouse_button program should link");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_struct_method_syntax() {
    let src = "
struct Vec2 { x: float, y: float }

fn vec_len(v: Vec2) -> float {
    return sqrt(v.x * v.x + v.y * v.y);
}

fn vec_dot(a: Vec2, b: Vec2) -> float {
    return a.x * b.x + a.y * b.y;
}

fn main() -> int {
    var v = Vec2 { x: 3.0, y: 4.0 };
    print(v.vec_len());
    var w = Vec2 { x: 1.0, y: 2.0 };
    print(v.vec_dot(w));
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "5\n11\n");
}

#[test]
fn e2e_method_on_int_expr() {
    let src = "
fn double(n: int) -> int { return n * 2; }
fn main() -> int {
    var x = 21;
    print(x.double());
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "42\n");
}

#[test]
fn e2e_tuple_literal_and_access() {
    let src = "
fn main() -> int {
    var t = (1, 2.5, true);
    print(t.0);
    print(t.1);
    print(t.2);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "1\n2.5\ntrue\n");
}

#[test]
fn e2e_tuple_destructure() {
    let src = "
fn main() -> int {
    var t = (10, 20);
    var a = t.0;
    var b = t.1;
    print(a + b);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "30\n");
}

#[test]
fn e2e_tuple_in_expression() {
    let src = "
fn main() -> int {
    var p = (3, 4);
    print(p.0 * p.0 + p.1 * p.1);
    return 0;
}
";
    let Some(out) = run_e2e(src, "") else {
        return;
    };
    assert_eq!(out, "25\n");
}

#[test]
fn npu_builtins_are_generated() {
    let c = compile(
        "fn main() -> int {
    var n = npu_count();
    var t = npu_tops();
    var l = npu_load();
    print(npu_name());
    if (npu_available()) { print(n + t + l); }
    return 0;
}
",
    )
    .expect("should compile");
    assert!(c.contains("iks_npu_available()"), "got: {}", c);
    assert!(c.contains("iks_npu_count()"), "got: {}", c);
    assert!(c.contains("iks_npu_name()"), "got: {}", c);
    assert!(c.contains("iks_npu_tops()"), "got: {}", c);
    assert!(c.contains("iks_npu_load()"), "got: {}", c);
}

#[test]
fn npu_names_are_builtin_reserved() {
    let err = compile("fn npu_name() -> int { return 0; }\n").unwrap_err();
    assert!(err.contains("built-in name"), "got: {}", err);
}

#[test]
fn npu_available_rejects_arguments() {
    let err = compile("fn main() -> int { return npu_available(1); }\n").unwrap_err();
    assert!(err.contains("npu_available"), "got: {}", err);
}

#[test]
fn vec_types_generate_helpers() {
    let c = compile(
        "fn main() -> int {
    var a = vec2(1.0, 2.0);
    var b = vec3(1.0, 2.0, 3.0);
    var c = vec4(1.0, 2.0, 3.0, 4.0);
    var d = a * 2.0;
    var e = normalize(b);
    var f = cross(b, b);
    print(dot(a, vec2(1.0, 1.0)));
    return 0;
}
",
    )
    .expect("should compile");
    assert!(c.contains("IksVec2"), "got: {}", c);
    assert!(c.contains("IksVec3"), "got: {}", c);
    assert!(c.contains("IksVec4"), "got: {}", c);
    assert!(c.contains("iks_v2_scale"), "got: {}", c);
    assert!(c.contains("iks_v3_norm"), "got: {}", c);
    assert!(c.contains("iks_v3_cross"), "got: {}", c);
    assert!(c.contains("iks_v2_dot"), "got: {}", c);
}

#[test]
fn vec_component_read_and_assign() {
    let c = compile(
        "fn main() -> int {
    var v = vec3(1.0, 2.0, 3.0);
    var y = v.y;
    v.z = 9.0;
    return 0;
}
",
    )
    .expect("should compile");
    assert!(c.contains("(v_v).y"), "got: {}", c);
    assert!(c.contains("(v_v).z = "), "got: {}", c);
}

#[test]
fn vec_scalar_and_vector_operators() {
    let c = compile(
        "fn main() -> int {
    var a = vec2(1.0, 2.0);
    var b = vec2(3.0, 4.0);
    var s = a + b;
    var t = a - b;
    var u = a / b;
    var w = b * 2.0;
    var x = 2.0 * b;
    var y = a + 1.0;
    var ok = a == b;
    return 0;
}
",
    )
    .expect("should compile");
    for want in [
        "iks_v2_add", "iks_v2_sub", "iks_v2_div", "iks_v2_scale", "iks_v2_adds", "iks_v2_eq",
    ] {
        assert!(c.contains(want), "missing {} in: {}", want, c);
    }
}

#[test]
fn vec_rejects_bad_component() {
    let err = compile("fn main() -> int { var v = vec2(1.0, 2.0); print(v.z); return 0; }\n")
        .unwrap_err();
    assert!(err.contains("has no component 'z'"), "got: {}", err);
}

#[test]
fn vec_rejects_dimension_mismatch() {
    let err = compile(
        "fn main() -> int { var a = vec2(1.0, 2.0); var b = vec3(1.0, 2.0, 3.0); print(a + b); return 0; }\n",
    )
    .unwrap_err();
    assert!(err.contains("cannot be applied"), "got: {}", err);
}

#[test]
fn vec_rejects_cross_on_wrong_type() {
    let err =
        compile("fn main() -> int { print(cross(vec2(1.0, 2.0), vec2(3.0, 4.0))); return 0; }\n")
            .unwrap_err();
    assert!(err.contains("'cross' expects two vec3"), "got: {}", err);
}

#[test]
fn vec_names_are_builtin_reserved() {
    let err = compile("fn vec2() -> int { return 0; }\n").unwrap_err();
    assert!(err.contains("built-in name"), "got: {}", err);
}

const SHAPE_ENUM: &str = "enum Shape { Circle(float), Rect(float, float), Point }\n";
const COLOR_ENUM: &str = "enum Color { Red, Green, Blue }\n";

#[test]
fn enum_decl_generates_tagged_union() {
    let c = compile(&format!(
        "{}fn main() -> int {{ switch (Circle(1.0)) {{ case Circle(r): {{ print(r); }} case Rect(w, h): {{ print(w); }} case Point: {{ print(0); }} }} return 0; }}\n",
        SHAPE_ENUM
    ))
    .expect("should compile");
    assert!(c.contains("typedef struct S_Shape"), "got: {}", c);
    assert!(c.contains("int64_t tag"), "got: {}", c);
    assert!(c.contains("._v0"), "got: {}", c);
    assert!(c.contains(".tag = 0"), "got: {}", c);
}

#[test]
fn enum_switch_binds_payload_values() {
    let c = compile(&format!(
        "{}fn main() -> int {{ switch (Rect(3.0, 4.0)) {{ case Rect(w, h): {{ print(w + h); }} case Circle(r): {{ print(r); }} case Point: {{ print(0); }} }} return 0; }}\n",
        SHAPE_ENUM
    ))
    .expect("should compile");
    assert!(c.contains("double w = "), "got: {}", c);
    assert!(c.contains("u._v1._0"), "got: {}", c);
    assert!(c.contains("u._v1._1"), "got: {}", c);
}

#[test]
fn exhaustive_switch_without_default_compiles() {
    compile(&format!(
        "{}fn main() -> int {{ switch (Point) {{ case Circle(r): {{ print(r); }} case Rect(w, h): {{ print(w); }} case Point: {{ print(0); }} }} return 0; }}\n",
        SHAPE_ENUM
    ))
    .expect("exhaustive switch needs no default");
}

#[test]
fn exhaustive_switch_may_be_the_last_statement() {
    compile(&format!(
        "{}fn pick(c: Color) -> int {{ switch (c) {{ case Red: {{ return 1; }} case Green: {{ return 2; }} case Blue: {{ return 3; }} }} }}\nfn main() -> int {{ print(pick(Blue)); return 0; }}\n",
        COLOR_ENUM
    ))
    .expect("exhaustive switch counts as a terminator");
}

#[test]
fn non_exhaustive_switch_is_rejected() {
    let err = compile(&format!(
        "{}fn main() -> int {{ switch (Point) {{ case Circle(r): {{ print(r); }} }} return 0; }}\n",
        SHAPE_ENUM
    ))
    .unwrap_err();
    assert!(err.contains("not exhaustive"), "got: {}", err);
    assert!(err.contains("Rect"), "got: {}", err);
    assert!(err.contains("Point"), "got: {}", err);
}

#[test]
fn switch_with_default_needs_no_exhaustive_coverage() {
    compile(&format!(
        "{}fn main() -> int {{ switch (Point) {{ case Circle(r): {{ print(r); }} default: {{ print(0); }} }} return 0; }}\n",
        SHAPE_ENUM
    ))
    .expect("default covers the rest");
}

#[test]
fn enum_rejects_unknown_variant_in_pattern() {
    let err = compile(&format!(
        "{}fn main() -> int {{ switch (Point) {{ case Square(s): {{ print(0); }} case Circle(r): {{ print(r); }} case Rect(w, h): {{ print(w); }} case Point: {{ print(0); }} }} return 0; }}\n",
        SHAPE_ENUM
    ))
    .unwrap_err();
    assert!(err.contains("is not a variant of enum 'Shape'"), "got: {}", err);
}

#[test]
fn enum_rejects_wrong_binding_count() {
    let err = compile(&format!(
        "{}fn main() -> int {{ switch (Point) {{ case Circle(r, s): {{ print(0); }} case Circle(r): {{ print(r); }} case Rect(w, h): {{ print(w); }} case Point: {{ print(0); }} }} return 0; }}\n",
        SHAPE_ENUM
    ))
    .unwrap_err();
    assert!(err.contains("but 2 bindings given"), "got: {}", err);
}

#[test]
fn enum_rejects_missing_bindings() {
    let err = compile(&format!(
        "{}fn main() -> int {{ switch (Point) {{ case Circle: {{ print(0); }} case Rect(w, h): {{ print(w); }} case Point: {{ print(0); }} }} return 0; }}\n",
        SHAPE_ENUM
    ))
    .unwrap_err();
    assert!(err.contains("needs 1 binding"), "got: {}", err);
}

#[test]
fn enum_rejects_value_pattern() {
    let err = compile(&format!(
        "{}fn main() -> int {{ switch (Point) {{ case 0: {{ print(0); }} case Circle(r): {{ print(r); }} case Rect(w, h): {{ print(w); }} case Point: {{ print(0); }} }} return 0; }}\n",
        SHAPE_ENUM
    ))
    .unwrap_err();
    assert!(err.contains("expected a variant of enum 'Shape'"), "got: {}", err);
}

#[test]
fn unit_variant_is_a_value() {
    let c = compile(&format!(
        "{}fn main() -> int {{ var c = Blue; print(c); return 0; }}\n",
        COLOR_ENUM
    ))
    .expect("unit variants are usable as values");
    assert!(c.contains(".tag = 2"), "got: {}", c);
}

#[test]
fn payload_variant_used_as_value_is_rejected() {
    let err = compile(&format!(
        "{}fn main() -> int {{ var c = Circle; print(0); return 0; }}\n",
        SHAPE_ENUM
    ))
    .unwrap_err();
    assert!(err.contains("carries 1 value"), "got: {}", err);
    assert!(err.contains("Circle(...)"), "got: {}", err);
}

#[test]
fn variant_name_is_globally_unique() {
    let err = compile(
        "enum A { Red }\nenum B { Red }\nfn main() -> int { return 0; }\n",
    )
    .unwrap_err();
    assert!(err.contains("is already defined"), "got: {}", err);
}

#[test]
fn variant_cannot_shadow_a_function() {
    let err = compile(&format!(
        "{}fn Circle() -> int {{ return 0; }}\nfn main() -> int {{ return 0; }}\n",
        SHAPE_ENUM
    ))
    .unwrap_err();
    assert!(err.contains("conflicts with the function"), "got: {}", err);
}

#[test]
fn variant_cannot_shadow_a_builtin() {
    let err = compile("enum E { vec2 }\nfn main() -> int { return 0; }\n").unwrap_err();
    assert!(err.contains("built-in name"), "got: {}", err);
}

#[test]
fn duplicate_variant_in_one_enum_is_rejected() {
    let err = compile("enum E { A, A }\nfn main() -> int { return 0; }\n").unwrap_err();
    assert!(err.contains("duplicate variant"), "got: {}", err);
}

#[test]
fn recursive_enum_is_rejected() {
    let err = compile("enum List { Cons(int, List), Nil }\nfn main() -> int { return 0; }\n")
        .unwrap_err();
    assert!(err.contains("recursive enum"), "got: {}", err);
}

#[test]
fn enum_unknown_payload_type_is_rejected() {
    let err = compile("enum E { A(Nope) }\nfn main() -> int { return 0; }\n").unwrap_err();
    assert!(err.contains("unknown type 'Nope'"), "got: {}", err);
}

#[test]
fn struct_may_hold_an_enum_field() {
    compile(&format!(
        "struct Item {{ kind: Color, amount: int }}\n{}fn main() -> int {{ var i = Item {{ kind: Red, amount: 5 }}; print(i.amount); return 0; }}\n",
        COLOR_ENUM
    ))
    .expect("struct/enum dependency order");
}

#[test]
fn value_switch_patterns_are_unchanged() {
    let c = compile(
        "fn main() -> int { var x = 2; switch (x) { case 1: { print(\"one\"); } case 2, 3: { print(\"few\"); } default: { print(\"many\"); } } return 0; }\n",
    )
    .expect("plain value switch still compiles");
    assert!(c.contains("strcmp") || c.contains("_sw0"), "got: {}", c);
}
