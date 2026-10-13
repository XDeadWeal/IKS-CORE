use iks_compiler::config;
use iks_compiler::diag::{Diagnostics, SourceMap};
use iks_compiler::report;
use iks_compiler::wasm;
use iks_compiler::Compiler;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

const VERSION: &str = "0.1.0";

const SCAFFOLD_MAIN: &str = r#"// IKS language demo
fn fib(n: int) -> int {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

fn main() -> int {
    var langs = ["IKS", "compiles", "via", "C"];
    var i = 0;
    while (i < len(langs)) {
        print(langs[i]);
        i = i + 1;
    }
    print("fib(20) =", fib(20));
    return 0;
}
"#;

/// Supported build targets.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Windows,
    Linux,
    Android,
    Web,
    Native,
}

impl Target {
    fn from_str(s: &str) -> Option<Target> {
        match s.to_ascii_lowercase().as_str() {
            "windows" | "win" | "win32" => Some(Target::Windows),
            "linux" => Some(Target::Linux),
            "android" | "apk" => Some(Target::Android),
            "web" | "wasm" => Some(Target::Web),
            "native" | "host" => Some(Target::Native),
            _ => None,
        }
    }
}

fn host_target() -> Target {
    if cfg!(target_os = "windows") {
        Target::Windows
    } else if cfg!(target_os = "linux") {
        Target::Linux
    } else {
        Target::Native
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str());
    match cmd {
        Some("create") => cmd_create(args.get(2).map(|s| s.as_str()).unwrap_or(".")),
        Some("build") => cmd_build(&args[2..]),
        Some("check") => cmd_check(),
        Some("test") => cmd_test(),
        Some("run") => cmd_run(&args[2..]),
        Some("clean") => cmd_clean(),
        Some("doctor") => cmd_doctor(),
        Some("version") | Some("-v") | Some("--version") => print_version(),
        Some("help") | Some("-h") | Some("--help") | None => print_help(),
        Some(other) => {
            eprintln!("ik: unknown command '{}'\n", other);
            print_help();
            exit(1);
        }
    }
}

fn print_version() {
    println!("ik {}", VERSION);
    println!("IKS language 1.0 — compiles to C, links with GCC");
}

fn print_help() {
    println!("{}", HELP);
}

const HELP: &str = "ik — the IKS programming language toolchain

USAGE:
    ik <command> [args]

COMMANDS:
    create [dir]             Scaffold a new project (default directory: '.')
    build [target]           Compile for a target (default: native host)
                             targets: windows | linux | android | web
    check                    Parse and type-check sources without producing a binary
    test                     Build and run all test_* functions, exit code 1 on failure
    run [target] [args...]   Build the project, then run the produced binary
    clean                    Delete build/ and LogsCompl/ artifacts
    doctor                   Check that every toolchain ik needs is installed
    version                  Print version information
    help                     Show this help

BUILD OPTIONS (after build/run):
    -o <path>                Write the output somewhere else (-o out/game)
    -O0|-O1|-O2|-Os          Optimisation level (default -O2)
    -v, --verbose            Print the exact compiler/linker command
    -nocmd                   windows: GUI subsystem, no console window
    -icon <path.ico>         windows: embed an icon into the .exe

TARGETS:
    windows                  Native Windows .exe (MinGW/MSYS2 gcc)
    linux                    Linux ELF binary (needs a native or cross gcc)
    android                  Android .so via the NDK (needs ANDROID_NDK_HOME)
    web                      WebAssembly + HTML (WebGL/WebGPU)

CROSS-COMPILING:
    Each target is built with the toolchain for that platform. For 'linux' on
    Windows ik auto-detects a cross-gcc or WSL and produces a real ELF. If a
    required toolchain is missing, ik prints a friendly hint on how to install it.

LOGS:
    Every build writes a timestamped log into <project>/LogsCompl/.

PROJECT LAYOUT:
    iks.toml               project config (name, version, list of source files)
    src/main.ik            program entry point, must define: fn main() -> int

LANGUAGE CHEAT SHEET:
    fn add(a: int, b: int) -> int { return a + b; }

    var x = 5;              // mutable variable (type is inferred)
    let y: float = 1.5;     // immutable variable with an explicit type
    var xs = [1, 2, 3];     // arrays, xs[0], len(xs)
    x = x + 1;

    struct Vec2 { x: float, y: float }   // plain C-style structs
    var v = Vec2 { x: 1.0, y: 2.0 };     // struct literal
    v.x = 3.0;                            // field access / assignment
    var p: ptr = nil;                     // opaque pointer type, nil literal

    import \"util.ik\";                   // include another source file
    extern \"C\" fn puts(s: string) -> int;  // link against a C symbol

    if (x > 3) { print(\"big\"); } else { print(\"small\"); }
    while (x > 0) { x = x - 1; }
    break; continue;

    print(\"hello\", x);     // print values, newline at the end
    len(xs)                 // array or string length
    str(42) int(\"5\") float(\"1.25\")
    input(\"prompt: \")      // read a line from stdin (string)
    push(xs, 9)             // returns a new array with the value appended
    rand()                  // random float in [0.0, 1.0)
    exit(0) sleep(16)       // terminate the program / sleep milliseconds

MATH:
    sin(x) cos(x) sqrt(x) abs(x) floor(x) pow(x, y)

VECTORS (vec2 / vec3 / vec4 - native targets, component math for games):
    vec2(x, y) vec3(x, y, z) vec4(x, y, z, w)   // construct; int args are promoted
    v.x v.y v.z v.w                              // component read and assign (float)
    a + b  a - b                                 // component-wise
    a * b  a / b                                 // component-wise (GLSL-compatible)
    a * s  s * a  a / s                          // scale / divide by a scalar
    a + s  a - s                                 // add or subtract a scalar in every component
    a == b  a != b                               // exact component equality
    dot(a, b)  length(v)  normalize(v)  distance(a, b)
    cross(a, b)                                  // vec3 only, right-handed
    rotate(v, radians)                           // vec2, around the origin
    lerp(a, b, t)                                // works for float and for vectors
    print(v)  str(v)                             // \"vec2(3, 4)\"

GRAPHICS (built-in window renderer):
    gfx_open(640, 480, \"title\")   // open a window (call once at start)
    while (gfx_poll()) { ... }     // false when the user closes the window
    gfx_clear(r, g, b)             // fill the frame, colors 0..255
    gfx_pixel(x, y, r, g, b)       // set a pixel
    gfx_add(x, y, r, g, b)         // add light to a pixel (glow effects)
    gfx_line(x0, y0, x1, y1, r, g, b)
    gfx_present()                  // show the frame
    gfx_width() gfx_height()       // framebuffer size
    gfx_time()                     // seconds since the window opened
    gfx_mouse_x() gfx_mouse_y()    // mouse position
    gfx_key_down(vk)               // key state, Windows VK codes (27=ESC, 37=LEFT ...)
    touch_x() touch_y()            // touch position (mouse on desktop, finger on mobile)

OPENGL (hardware-accelerated, Windows, immediate mode):
    gl_init(w, h, \"title\")        // open a window + OpenGL context (once at start)
    while (gl_swap()) { ... }       // present the frame, false when the window closes
    gl_clear(r, g, b)               // clear color+depth, colors 0..255
    gl_view(x, y, w, h)             // viewport in pixels
    gl_proj(fov_deg, aspect, zn, zf)// set the perspective projection
    gl_push() gl_pop()              // matrix stack
    gl_translate(x, y, z) gl_rotate(deg, x, y, z) gl_scale(x, y, z)
    gl_color(r, g, b, a)            // current color, 0..255
    gl_begin(mode) gl_vertex(x, y, z) gl_end()
                                    // 0=points 1=lines 2=line_strip 3=triangles
                                    // 4=tri_strip 5=tri_fan 6=quads 7=quad_strip
    gl_point_size(s) gl_line_width(w)
    gl_depth(on) gl_blend(on)       // toggle depth test / additive blending
    gl_title(\"text\")                // set the window title at runtime
    gl_fullscreen(on)                // toggle borderless fullscreen
    gl_width() gl_height()           // current client size in pixels
    gl_proj_ortho(l, r, b, t)        // 2D orthographic projection (for HUDs)
    gl_rect(x, y, w, h) gl_circle(cx, cy, r, segs) gl_line(x0, y0, x1, y1)
                                    // 2D helpers (WebGL subset mirrors these)
    cpu_load()                       // process cpu usage, percent of one core
    gpu_load()                       // gpu busy percent (GL timer queries)

NPU (neural accelerator detection - PnP/sysfs description heuristic):
    npu_available()                  // true when an NPU/neural device is present
    npu_count()                      // number of detected NPU devices
    npu_name()                       // device description (\"\" when none detected)
    npu_tops()                       // 0 unless a capability table is installed
    npu_load()                       // 0.0 (no cross-vendor load API wired yet)

SPRITES (PNG/JPG/TGA/BMP loading via stb_image, textured quads):
    sprite_load(\"path\")             // load an image, returns an id (-1 on error;
                                     // requires gl_init() first for a GL context)
    sprite_draw(id, x, y, w, h)      // draw the sprite at x,y sized w*h (current color tints)
    sprite_width(id) sprite_height(id)   // natural pixel size of the loaded image
    sprite_unload(id)                // free the texture

WEB (WebGL/WebGPU, `ik build web`, runs in a browser via WebAssembly):
    gl_init(w, h, \"title\") gl_clear(r, g, b) gl_color(r, g, b, a)
    gl_begin(mode) gl_vertex(x, y, z) gl_end() gl_swap()
    gl_width() gl_height() gl_rect(x, y, w, h) gl_circle(cx, cy, r, segs)
    gl_line(x0, y0, x1, y1) gl_title(\"text\")
    webgpu_init(w, h, \"title\") webgpu_clear(r, g, b) webgpu_color(r, g, b, a)
    webgpu_begin(mode) webgpu_vertex(x, y, z) webgpu_end() webgpu_swap()
    webgpu_width() webgpu_height() webgpu_rect(x, y, w, h)
    webgpu_circle(cx, cy, r, segs) webgpu_line(x0, y0, x1, y1) webgpu_title(\"text\")

DIRECT3D 11 (triangle pipeline, Windows):
    d3d_init(w, h, \"title\")        // open a window + D3D11 device, runtime HLSL
    while (d3d_present()) { ... }   // present, false when the window closes
    d3d_clear(r, g, b)              // clear the render target, colors 0..255
    d3d_draw(verts)                 // draw triangles from a float[] where each
                                    // vertex is x, y, z, r, g, b (NDC coords -1..1)

TESTS:
    fn test_add() -> bool { return add(1, 2) == 3; }
    ik test                         // runs every fn named test_*, bool return = pass/fail
    assert(cond, \"message\")        // abort the program when the condition is false

    Types: int, float, bool, string, void, ptr, T[], StructName
    Operators: + - * / %  == != < <= > >=  && || !  =
    Comments: // line  /* block */";

fn cmd_create(path: &str) {
    let root = Path::new(path);
    let proj_name = if path == "." || path.is_empty() {
        "iks_project".to_string()
    } else {
        root.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "iks_project".to_string())
    };
    if let Err(e) = fs::create_dir_all(root.join("src")) {
        eprintln!("ik: cannot create project directory: {}", e);
        exit(1);
    }
    let main_ik = root.join("src").join("main.ik");
    write_if_missing(&main_ik, SCAFFOLD_MAIN);
    let toml = root.join("iks.toml");
    let toml_text = format!(
        "[project]\nname = \"{}\"\nversion = \"0.1.0\"\n\n[build]\nfiles = [\n    \"src/main.ik\",\n]\n",
        sanitize_name(&proj_name)
    );
    write_if_missing(&toml, &toml_text);
    println!("[iks] project '{}' is ready", proj_name);
    println!("[iks] next: cd {} && ik build && ik run", path);
}

fn write_if_missing(path: &Path, content: &str) {
    if path.exists() {
        println!("[iks] {} already exists, keeping it", path.display());
    } else {
        if let Err(e) = fs::write(path, content) {
            eprintln!("ik: cannot write {}: {}", path.display(), e);
            exit(1);
        }
        println!("[iks] wrote {}", path.display());
    }
}

fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if cleaned.is_empty() {
        "iks_project".to_string()
    } else {
        cleaned
    }
}

/// Print diagnostics + success line and write the LogsCompl log file.
/// Returns after exiting on error.
fn finish(map: &SourceMap, result: Result<String, Diagnostics>, success: &str) -> String {
    match result {
        Ok(code) => {
            report::report(map, &[], &[], Some(success), Path::new("."));
            code
        }
        Err(d) => {
            report::report(map, &d.errors, &d.warnings, None, Path::new("."));
            exit(1);
        }
    }
}

/// Compile sources into C, printing diagnostics and writing logs.
/// Returns the C file path on success.
fn compile_to_c_with_target(target: Target, test_mode: bool) -> String {
    let cfg = match config::load(Path::new("iks.toml")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ik: {}", e);
            exit(1);
        }
    };
    let mut comp = Compiler::new();
    for f in &cfg.files {
        let text = match fs::read_to_string(f) {
            Ok(t) => t,
            Err(_) => {
                eprintln!("ik: cannot read source file '{}'", f);
                exit(1);
            }
        };
        comp.add_file(f.clone(), text);
    }
    let res = if test_mode { comp.compile_test() } else { comp.compile() };
    let c_code = finish(comp.source_map(), res, "compilation succeeded");
    if let Err(e) = fs::create_dir_all("build") {
        eprintln!("ik: cannot create 'build' directory: {}", e);
        exit(1);
    }
    let name = sanitize_name(&cfg.name);
    let stem = if test_mode { format!("{}_test", name) } else { name };
    let c_path = format!("build/{}.c", stem);
    if let Err(e) = fs::write(&c_path, &c_code) {
        eprintln!("ik: cannot write {}: {}", c_path, e);
        exit(1);
    }
    if c_code.contains("stb_image.h") {
        const STB_IMAGE_H: &str = include_str!("../vendor/stb_image.h");
        if let Err(e) = fs::write("build/stb_image.h", STB_IMAGE_H) {
            eprintln!("ik: cannot write build/stb_image.h: {}", e);
            exit(1);
        }
    }
    let _ = target;
    c_path
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
        if Path::new(&inc).exists() && Path::new(&lib).exists() {
            return Some(sdk);
        }
    }
    None
}

/// Try a compiler command; return Some(path_to_tool) if it runs.
fn tool_available(cmd: &str, probe_args: &[&str]) -> bool {
    Command::new(cmd)
        .args(probe_args)
        .output()
        .map(|o| o.status.success() || !o.stdout.is_empty() || !o.stderr.is_empty())
        .unwrap_or(false)
}

enum Cc {
    Plain(String),
    Wsl(String),
}

impl Cc {
    fn name(&self) -> &str {
        match self {
            Cc::Plain(s) | Cc::Wsl(s) => s,
        }
    }
}

fn wsl_gcc_available() -> bool {
    Command::new("wsl")
        .args(["-e", "bash", "-lc", "gcc --version 2>/dev/null | head -1"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("gcc"))
        .unwrap_or(false)
}

fn wsl_shell_arg(a: &str) -> String {
    let b = a.as_bytes();
    if b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')
    {
        format!("$(wslpath -u '{}')", a)
    } else {
        a.to_string()
    }
}

fn embed_icon(icon: &str) -> Result<Option<String>, String> {
    if !Path::new(icon).exists() {
        return Err(format!("icon file not found: '{}'", icon));
    }
    let mut windres: Option<String> = None;
    for c in [
        "windres",
        "x86_64-w64-mingw32-windres",
        "i686-w64-mingw32-windres",
    ] {
        if tool_available(c, &["--version"]) {
            windres = Some(c.to_string());
            break;
        }
    }
    let windres = match windres {
        Some(w) => w,
        None => {
            eprintln!("warning: windres not found; -icon ignored (install MinGW binutils)");
            return Ok(None);
        }
    };
    let _ = fs::create_dir_all("build");
    let rc = "build/ik_icon.rc";
    let obj = "build/ik_icon.o";
    let content = format!("1 ICON \"{}\"\n", icon.replace('\\', "/"));
    fs::write(rc, content).map_err(|e| format!("cannot write {}: {}", rc, e))?;
    let status = Command::new(&windres)
        .args(["-i", rc, "-o", obj, "--output-format=coff"])
        .status()
        .map_err(|e| format!("failed to run {}: {}", windres, e))?;
    if !status.success() {
        return Err(format!(
            "{} failed while compiling the icon; pass a valid .ico file",
            windres
        ));
    }
    Ok(Some(obj.to_string()))
}

/// Resolve the C compiler for a target, with friendly cross-compile hints.
fn resolve_compiler(target: Target) -> Cc {
    let host = host_target();
    match target {
        Target::Windows => {
            // On Windows host use plain gcc; elsewhere try a MinGW cross compiler.
            if host == Target::Windows && tool_available("gcc", &["--version"]) {
                return Cc::Plain("gcc".to_string());
            }
            for cand in ["x86_64-w64-mingw32-gcc", "i686-w64-mingw32-gcc", "gcc"] {
                if tool_available(cand, &["--version"]) {
                    return Cc::Plain(cand.to_string());
                }
            }
            eprintln!("ik: no MinGW cross-compiler found for target 'windows'.");
            eprintln!("   hint: install MinGW-w64 (MSYS2: pacman -S mingw-w64-x86_64-gcc),");
            eprintln!("         or add gcc to PATH if you are already on Windows.");
            exit(1);
        }
        Target::Linux => {
            if host == Target::Linux && tool_available("gcc", &["--version"]) {
                return Cc::Plain("gcc".to_string());
            }
            if tool_available("x86_64-linux-gnu-gcc", &["--version"]) {
                return Cc::Plain("x86_64-linux-gnu-gcc".to_string());
            }
            if wsl_gcc_available() {
                return Cc::Wsl("gcc".to_string());
            }
            if tool_available("gcc", &["--version"]) {
                eprintln!("warning: no Linux toolchain found; using host 'gcc',");
                eprintln!("         which emits a Windows/Cygwin binary, not a Linux ELF.");
                eprintln!("         install WSL (wsl --install) or x86_64-linux-gnu-gcc for a real Linux build.");
                return Cc::Plain("gcc".to_string());
            }
            eprintln!("ik: no Linux toolchain found for target 'linux'.");
            eprintln!("   hint: on Windows install WSL (wsl --install) or a Linux cross-gcc (MSYS2),");
            eprintln!("         or build on a Linux host where gcc is present.");
            exit(1);
        }
        Target::Android => {
            if let Some(tc) = detect_android_tc() {
                return Cc::Plain(tc.clang.to_string_lossy().to_string());
            }
            eprintln!("ik: Android NDK not found for target 'android'.");
            eprintln!("   hint: set ANDROID_NDK_HOME to your NDK path, e.g.");
            eprintln!("         ANDROID_NDK_HOME=C:\\Android\\ndk\\26.1.10909125");
            eprintln!("         or install it via Android Studio > SDK Manager > NDK.");
            exit(1);
        }
        _ => {
            if tool_available("gcc", &["--version"]) {
                Cc::Plain("gcc".to_string())
            } else {
                eprintln!("ik: 'gcc' not found in PATH.");
                eprintln!("   hint: install GCC (MinGW/MSYS2 on Windows, build-essential on Linux).");
                exit(1);
            }
        }
    }
}

/// Android NDK toolchain (raw clang + sysroot + NDK root for glue/headers).
struct AndroidTc {
    clang: PathBuf,
    sysroot: PathBuf,
    ndk: PathBuf,
}

fn detect_android_tc() -> Option<AndroidTc> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(v) = env::var("ANDROID_NDK_HOME") {
        roots.push(PathBuf::from(v));
    }
    if let Ok(v) = env::var("ANDROID_NDK_ROOT") {
        roots.push(PathBuf::from(v));
    }
    if let Ok(v) = env::var("ANDROID_HOME").or_else(|_| env::var("ANDROID_SDK_ROOT")) {
        roots.push(PathBuf::from(v).join("ndk"));
    }
    roots.push(PathBuf::from("C:/Android/ndk"));
    roots.push(PathBuf::from(env::var("LOCALAPPDATA").unwrap_or_default()).join("Android/Sdk/ndk"));

    let mut ndks: Vec<PathBuf> = Vec::new();
    for r in roots {
        if r.join("source.properties").exists() {
            ndks.push(r.clone());
        }
        if let Ok(entries) = fs::read_dir(&r) {
            let mut subs: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect();
            subs.sort();
            subs.reverse();
            for s in subs {
                if s.join("source.properties").exists() {
                    ndks.push(s);
                }
            }
        }
    }
    for ndk in ndks {
        if let Ok(hosts) = fs::read_dir(ndk.join("toolchains/llvm/prebuilt")) {
            let mut hosts: Vec<PathBuf> = hosts
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect();
            hosts.sort();
            for h in hosts {
                let clang = h.join("bin").join("clang.exe");
                let sysroot = h.join("sysroot");
                if clang.exists() && sysroot.exists() {
                    return Some(AndroidTc { clang, sysroot, ndk });
                }
            }
        }
    }
    None
}

/// Compile the generated C into a shared library for arm64-v8a, including the
/// NativeActivity glue that provides ANativeActivity_onCreate -> android_main.
fn build_android_so(c_path: &str, name: &str, tc: &AndroidTc) -> Result<String, String> {
    let lib = format!("build/lib{}.so", name);
    let glue_c = tc.ndk.join("sources/android/native_app_glue/android_native_app_glue.c");
    let glue_inc = tc.ndk.join("sources/android/native_app_glue");
    let mut cmd = Command::new(&tc.clang);
    cmd.arg("--target=aarch64-linux-android21");
    cmd.arg(format!("--sysroot={}", tc.sysroot.display()));
    cmd.arg("-shared").arg("-fPIC").arg("-O2");
    cmd.arg("-DIKS_PLATFORM_ANDROID");
    cmd.arg("-o").arg(&lib).arg(c_path);
    if glue_c.exists() {
        cmd.arg(&glue_c);
    }
    cmd.arg(format!("-I{}", glue_inc.display()));
    cmd.arg("-Ibuild");
    cmd.arg("-lEGL").arg("-lGLESv2").arg("-landroid").arg("-llog").arg("-lm");
    println!("[iks] compiling {} for arm64-v8a -> {}", c_path, lib);
    let status = cmd
        .status()
        .map_err(|e| format!("failed to run clang ({}): {}", tc.clang.display(), e))?;
    if !status.success() {
        return Err("clang failed for the android target; see the errors above".into());
    }
    Ok(lib)
}

/// Locate the Android SDK root (holds build-tools/ and platforms/).
fn detect_android_sdk() -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(v) = env::var("ANDROID_HOME") {
        roots.push(PathBuf::from(v));
    }
    if let Ok(v) = env::var("ANDROID_SDK_ROOT") {
        roots.push(PathBuf::from(v));
    }
    roots.push(PathBuf::from(env::var("LOCALAPPDATA").unwrap_or_default()).join("Android/Sdk"));
    roots.push(PathBuf::from("C:/Android/Sdk"));
    roots
        .into_iter()
        .find(|r| r.join("platforms").exists() && r.join("build-tools").exists())
}

fn find_build_tools(sdk: &Path) -> Option<PathBuf> {
    let mut vers: Vec<PathBuf> = fs::read_dir(sdk.join("build-tools")).ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    vers.sort();
    vers.reverse();
    vers.into_iter()
        .find(|v| v.join("aapt2.exe").exists() && v.join("zipalign.exe").exists())
}

fn find_android_jar(sdk: &Path) -> Option<PathBuf> {
    let mut vers: Vec<PathBuf> = fs::read_dir(sdk.join("platforms")).ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    vers.sort();
    vers.reverse();
    vers.into_iter()
        .find(|v| v.join("android.jar").exists())
        .map(|v| v.join("android.jar"))
}

fn find_keytool() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(v) = env::var("JAVA_HOME") {
        cands.push(PathBuf::from(v).join("bin").join("keytool.exe"));
    }
    for base in ["C:/Program Files/Java", "C:/Program Files (x86)/Java"] {
        if let Ok(entries) = fs::read_dir(base) {
            let mut vers: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect();
            vers.sort();
            vers.reverse();
            for v in vers {
                cands.push(v.join("bin").join("keytool.exe"));
            }
        }
    }
    cands.into_iter().find(|c| c.exists())
}

fn find_java() -> PathBuf {
    if let Ok(v) = env::var("JAVA_HOME") {
        let j = PathBuf::from(v).join("bin").join("java.exe");
        if j.exists() {
            return j;
        }
    }
    PathBuf::from("java")
}

fn sanitize_pkg(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    if s.is_empty() || s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        s = format!("app_{}", s);
    }
    s
}

/// Insert a file into an existing APK (zip) using PowerShell's ZipArchive.
fn add_lib_to_apk(apk: &str, so: &str, entry: &str) -> Result<(), String> {
    let script = format!(
        "Add-Type -AssemblyName System.IO.Compression; \
         Add-Type -AssemblyName System.IO.Compression.FileSystem; \
         $z=[System.IO.Compression.ZipFile]::Open('{apk}',[System.IO.Compression.ZipArchiveMode]::Update); \
         [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($z,'{so}','{entry}')|Out-Null; \
         $z.Dispose()"
    );
    let status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map_err(|e| format!("failed to run powershell: {}", e))?;
    if !status.success() {
        return Err(format!("failed to add {} into {}", so, apk));
    }
    Ok(())
}

fn ensure_debug_keystore() -> Result<PathBuf, String> {
    let ks = PathBuf::from("build/debug.keystore");
    if ks.exists() {
        return Ok(ks);
    }
    let keytool =
        find_keytool().ok_or("keytool not found (install a JDK or set JAVA_HOME)")?;
    let status = Command::new(&keytool)
        .args([
            "-genkeypair",
            "-v",
            "-keystore",
            &ks.display().to_string(),
            "-storepass",
            "android",
            "-alias",
            "androiddebugkey",
            "-keypass",
            "android",
            "-dname",
            "CN=Android Debug,O=Android,C=US",
            "-keyalg",
            "RSA",
            "-keysize",
            "2048",
            "-validity",
            "10000",
        ])
        .status()
        .map_err(|e| format!("failed to run keytool: {}", e))?;
    if !status.success() {
        return Err("keytool failed to create the debug keystore".into());
    }
    Ok(ks)
}

/// Package build/lib<name>.so into a signed, installable APK.
fn build_android_apk(lib: &str, name: &str, sdk: &Path) -> Result<String, String> {
    let bt = find_build_tools(sdk)
        .ok_or("Android build-tools not found (need aapt2 + zipalign)")?;
    let jar = find_android_jar(sdk).ok_or("android.jar not found under platforms/")?;
    let aapt2 = bt.join("aapt2.exe");
    let zipalign = bt.join("zipalign.exe");

    let manifest = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="org.iks.{}"
    android:versionCode="1"
    android:versionName="1.0">
    <uses-sdk android:minSdkVersion="21" android:targetSdkVersion="35"/>
    <uses-feature android:glEsVersion="0x00020000" android:required="true"/>
    <application android:label="IKS" android:hasCode="false" android:extractNativeLibs="true">
        <activity android:name="android.app.NativeActivity"
            android:label="IKS"
            android:exported="true"
            android:configChanges="orientation|screenSize|keyboardHidden">
            <meta-data android:name="android.app.lib_name" android:value="{}"/>
            <intent-filter>
                <action android:name="android.intent.action.MAIN"/>
                <category android:name="android.intent.category.LAUNCHER"/>
            </intent-filter>
        </activity>
    </application>
</manifest>
"#,
        sanitize_pkg(name),
        name
    );
    fs::write("build/AndroidManifest.xml", &manifest).map_err(|e| e.to_string())?;

    // 1. aapt2 link -> binary manifest inside an (unsigned) APK.
    let base = "build/base-unsigned.apk";
    let _ = fs::remove_file(base);
    let status = Command::new(&aapt2)
        .arg("link")
        .arg("-o")
        .arg(base)
        .arg("-I")
        .arg(&jar)
        .arg("--manifest")
        .arg("build/AndroidManifest.xml")
        .arg("--min-sdk-version")
        .arg("21")
        .arg("--target-sdk-version")
        .arg("35")
        .status()
        .map_err(|e| format!("failed to run aapt2: {}", e))?;
    if !status.success() {
        return Err("aapt2 link failed".into());
    }

    // 2. pack the native library at lib/arm64-v8a/.
    let entry = format!("lib/arm64-v8a/lib{}.so", name);
    add_lib_to_apk(base, lib, &entry)?;

    // 3. zipalign.
    let aligned = "build/aligned.apk";
    let _ = fs::remove_file(aligned);
    let status = Command::new(&zipalign)
        .arg("-f")
        .arg("4")
        .arg(base)
        .arg(aligned)
        .status()
        .map_err(|e| format!("failed to run zipalign: {}", e))?;
    if !status.success() {
        return Err("zipalign failed".into());
    }

    // 4. sign with a debug keystore (apksigner.jar driven by java, avoiding .bat).
    let apk = format!("build/{}.apk", name);
    let _ = fs::remove_file(&apk);
    let ks = ensure_debug_keystore()?;
    let apksigner_jar = bt.join("lib").join("apksigner.jar");
    let status = Command::new(find_java())
        .arg("-cp")
        .arg(&apksigner_jar)
        .arg("com.android.apksigner.ApkSignerTool")
        .arg("sign")
        .arg("--ks")
        .arg(&ks)
        .arg("--ks-pass")
        .arg("pass:android")
        .arg("--key-pass")
        .arg("pass:android")
        .arg("--ks-key-alias")
        .arg("androiddebugkey")
        .arg("--out")
        .arg(&apk)
        .arg(aligned)
        .status()
        .map_err(|e| format!("failed to run apksigner: {}", e))?;
    if !status.success() {
        return Err("apksigner failed".into());
    }
    Ok(apk)
}

/// Full Android pipeline: C -> arm64-v8a .so -> signed APK.
fn build_android() {
    let c_path = compile_to_c_with_target(Target::Android, false);
    let name = Path::new(&c_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let tc = match detect_android_tc() {
        Some(t) => t,
        None => {
            eprintln!("ik: Android NDK not found for target 'android'.");
            eprintln!("   hint: set ANDROID_NDK_HOME to your NDK path, e.g.");
            eprintln!("         ANDROID_NDK_HOME=C:\\Android\\ndk\\28.2.13676358");
            eprintln!("         or install it via Android Studio > SDK Manager > NDK.");
            exit(1);
        }
    };
    let lib = match build_android_so(&c_path, &name, &tc) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("ik: {}", e);
            exit(1);
        }
    };
    match detect_android_sdk() {
        Some(sdk) => match build_android_apk(&lib, &name, &sdk) {
            Ok(apk) => {
                println!("[iks] built Android library: {}", lib);
                println!("[iks] build succeeded: {}", apk);
                println!("[iks] install on your phone: adb install -r {}", apk);
            }
            Err(e) => {
                eprintln!("ik: built {} but APK packaging failed: {}", lib, e);
                eprintln!("   (the .so is still at {})", lib);
                exit(1);
            }
        },
        None => {
            eprintln!("ik: built {} but no Android SDK found for packaging.", lib);
            eprintln!("   hint: set ANDROID_HOME to your SDK path (needs build-tools + platforms).");
            exit(1);
        }
    }
}

fn link_native(
    cc: &Cc,
    c_path: &str,
    name: &str,
    target: Target,
    opts: &BuildOpts,
) -> Result<String, String> {
    let exe = match &opts.out {
        Some(p) => {
            if let Some(dir) = Path::new(p).parent() {
                if !dir.as_os_str().is_empty() {
                    let _ = fs::create_dir_all(dir);
                }
            }
            if target == Target::Windows && !p.to_lowercase().ends_with(".exe") {
                format!("{}.exe", p)
            } else {
                p.clone()
            }
        }
        None => match target {
            Target::Windows => format!("build/{}.exe", name),
            Target::Android => format!("build/lib{}.so", name),
            _ => format!("build/{}", name),
        },
    };
    let mut args: Vec<String> = vec![opts.opt.clone()];
    match target {
        Target::Windows => args.push("-DIKS_PLATFORM_WINDOWS".to_string()),
        Target::Linux => args.push("-DIKS_PLATFORM_LINUX".to_string()),
        Target::Android => args.push("-DIKS_PLATFORM_ANDROID".to_string()),
        _ => {}
    }
    if let Some(icon) = &opts.icon {
        if target == Target::Windows {
            if let Some(obj) = embed_icon(icon)? {
                args.push(obj);
            }
        } else {
            eprintln!("warning: -icon only applies to the windows target; ignored here");
        }
    }
    if target == Target::Android {
        args.push("-shared".to_string());
        args.push("-fPIC".to_string());
        args.push("-o".to_string());
        args.push(exe.clone());
        args.push(c_path.to_string());
        args.push("-lm".to_string());
    } else {
        args.push("-o".to_string());
        args.push(exe.clone());
        args.push(c_path.to_string());
        args.push("-lm".to_string());
        if target == Target::Windows {
            for l in [
                "-lgdi32", "-luser32", "-lopengl32", "-ld3d9", "-ld3d10", "-ld3d11",
                "-ldxgi", "-ld3dcompiler", "-ld3d12", "-lpdh", "-ladvapi32",
            ] {
                args.push(l.to_string());
            }
            if opts.nocmd {
                args.push("-mwindows".to_string());
            }
            if let Some(sdk) = detect_vulkan_sdk() {
                println!("[iks] Vulkan SDK found: {}", sdk);
                args.push(format!("-I{}/Include", sdk));
                args.push(format!("{}/Lib/vulkan-1.lib", sdk));
                args.push("-DIKS_VK=1".to_string());
            }
        }
        if target == Target::Linux {
            args.push("-lX11".to_string());
            args.push("-lGL".to_string());
        }
    }

    let (prog, run_args) = match cc {
        Cc::Wsl(gcc) => {
            let cwd =
                env::current_dir().map_err(|_| "cannot determine working directory".to_string())?;
            let mut script = format!(
                "cd \"$(wslpath -u '{}')\" && {}",
                cwd.to_string_lossy(),
                gcc
            );
            for a in &args {
                script.push(' ');
                script.push_str(&wsl_shell_arg(a));
            }
            (
                "wsl".to_string(),
                vec![
                    "-e".to_string(),
                    "bash".to_string(),
                    "-lc".to_string(),
                    script,
                ],
            )
        }
        Cc::Plain(_) => (cc.name().to_string(), args),
    };
    if opts.verbose {
        print!("[iks] $ {}", prog);
        for a in &run_args {
            print!(" {}", a);
        }
        println!();
    }
    println!("[iks] linking with {} -> {}", prog, exe);
    let status = Command::new(&prog)
        .args(&run_args)
        .status()
        .map_err(|_| format!("could not run '{}'; is it installed and in PATH?", prog))?;
    if !status.success() {
        return Err(format!("{} failed; see the errors above", prog));
    }
    Ok(exe)
}

#[derive(Clone)]
struct BuildOpts {
    out: Option<String>,
    opt: String,
    verbose: bool,
    nocmd: bool,
    icon: Option<String>,
}

impl Default for BuildOpts {
    fn default() -> Self {
        BuildOpts {
            out: None,
            opt: "-O2".to_string(),
            verbose: false,
            nocmd: false,
            icon: None,
        }
    }
}

fn parse_cli(args: &[String]) -> (Target, BuildOpts, Vec<String>) {
    let mut target = Target::Native;
    let mut opts = BuildOpts::default();
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if let Some(t) = Target::from_str(a) {
            target = t;
        } else if let Some(v) = a.strip_prefix("--target=") {
            if let Some(t) = Target::from_str(v) {
                target = t;
            }
        } else if a == "--target" {
            if i + 1 < args.len() {
                if let Some(t) = Target::from_str(&args[i + 1]) {
                    target = t;
                }
                i += 2;
                continue;
            }
        } else if a == "-o" {
            if i + 1 < args.len() {
                opts.out = Some(args[i + 1].clone());
                i += 2;
                continue;
            }
        } else if a == "-icon" {
            if i + 1 < args.len() {
                opts.icon = Some(args[i + 1].clone());
                i += 2;
                continue;
            }
        } else if a == "-v" || a == "--verbose" {
            opts.verbose = true;
        } else if a == "-nocmd" {
            opts.nocmd = true;
        } else if a.len() == 3
            && a.starts_with("-O")
            && matches!(a.as_bytes()[2], b'0' | b'1' | b'2' | b's' | b'S')
        {
            opts.opt = a.to_string();
        } else {
            rest.push(a.to_string());
        }
        i += 1;
    }
    if target == Target::Native {
        target = host_target();
    }
    (target, opts, rest)
}

fn build_web() {
    let cfg = match config::load(Path::new("iks.toml")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ik: {}", e);
            exit(1);
        }
    };
    let mut comp = Compiler::new();
    for f in &cfg.files {
        let text = match fs::read_to_string(f) {
            Ok(t) => t,
            Err(_) => {
                eprintln!("ik: cannot read source file '{}'", f);
                exit(1);
            }
        };
        comp.add_file(f.clone(), text);
    }
    let res = comp.compile_wasm();
    let bytes = match res {
        Ok(b) => b,
        Err(d) => {
            report::report(comp.source_map(), &d.errors, &d.warnings, None, Path::new("."));
            exit(1);
        }
    };
    report::report(comp.source_map(), &[], comp.warnings(), Some("compilation succeeded"), Path::new("."));
    if let Err(e) = fs::create_dir_all("build") {
        eprintln!("ik: cannot create 'build' directory: {}", e);
        exit(1);
    }
    let name = sanitize_name(&cfg.name);
    let wasm_path = format!("build/{}.wasm", name);
    let html_path = format!("build/{}.html", name);
    if let Err(e) = fs::write(&wasm_path, &bytes) {
        eprintln!("ik: cannot write {}: {}", wasm_path, e);
        exit(1);
    }
    if let Err(e) = fs::write(&html_path, wasm::web_html(&name)) {
        eprintln!("ik: cannot write {}: {}", html_path, e);
        exit(1);
    }
    println!("[iks] generated WebAssembly: {} ({} bytes)", wasm_path, bytes.len());
    println!("[iks] open {} in a browser, or: ik run web", html_path);
    println!("[iks] build succeeded");
}

fn cmd_build(args: &[String]) {
    let (target, opts, _) = parse_cli(args);
    if target == Target::Web {
        build_web();
        return;
    }
    if target == Target::Android {
        build_android();
        return;
    }
    let c_path = compile_to_c_with_target(target, false);
    let name = Path::new(&c_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let cc = resolve_compiler(target);
    match link_native(&cc, &c_path, &name, target, &opts) {
        Ok(exe) => println!("[iks] build succeeded: {}", exe),
        Err(e) => {
            eprintln!("ik: {}", e);
            exit(1);
        }
    }
}

fn cmd_check() {
    let cfg = match config::load(Path::new("iks.toml")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ik: {}", e);
            exit(1);
        }
    };
    let mut comp = Compiler::new();
    for f in &cfg.files {
        let text = match fs::read_to_string(f) {
            Ok(t) => t,
            Err(_) => {
                eprintln!("ik: cannot read source file '{}'", f);
                exit(1);
            }
        };
        comp.add_file(f.clone(), text);
    }
    match comp.compile() {
        Ok(_) => {
            report::report(comp.source_map(), &[], comp.warnings(), Some("check succeeded: no errors found"), Path::new("."));
        }
        Err(d) => {
            report::report(comp.source_map(), &d.errors, &d.warnings, None, Path::new("."));
            exit(1);
        }
    }
}

fn cmd_run(args: &[String]) {
    let (target, opts, prog_args) = parse_cli(args);
    if target == Target::Web {
        run_web();
        return;
    }
    if target == Target::Android {
        build_android();
        return;
    }
    let c_path = compile_to_c_with_target(target, false);
    let name = Path::new(&c_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let cc = resolve_compiler(target);
    let exe = match link_native(&cc, &c_path, &name, target, &opts) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("ik: {}", e);
            exit(1);
        }
    };
    if target == Target::Android {
        println!("[iks] built Android library: {}", exe);
        println!("[iks] (wrap it in an APK with your NDK/Gradle project)");
        return;
    }
    println!("[iks] running {}", exe);
    match Command::new(&exe).args(&prog_args).status() {
        Ok(status) => {
            if let Some(code) = status.code() {
                exit(code);
            }
        }
        Err(e) => {
            eprintln!("ik: failed to run '{}': {}", exe, e);
            exit(1);
        }
    }
}

fn run_web() {
    let cfg = match config::load(Path::new("iks.toml")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ik: {}", e);
            exit(1);
        }
    };
    let mut comp = Compiler::new();
    for f in &cfg.files {
        let text = match fs::read_to_string(f) {
            Ok(t) => t,
            Err(_) => {
                eprintln!("ik: cannot read source file '{}'", f);
                exit(1);
            }
        };
        comp.add_file(f.clone(), text);
    }
    let bytes = match comp.compile_wasm() {
        Ok(b) => b,
        Err(d) => {
            report::report(comp.source_map(), &d.errors, &d.warnings, None, Path::new("."));
            exit(1);
        }
    };
    report::report(comp.source_map(), &[], comp.warnings(), Some("compilation succeeded"), Path::new("."));
    if let Err(e) = fs::create_dir_all("build") {
        eprintln!("ik: cannot create 'build' directory: {}", e);
        exit(1);
    }
    let name = sanitize_name(&cfg.name);
    let wasm_path = format!("build/{}.wasm", name);
    let html_path = format!("build/{}.html", name);
    let mjs_path = format!("build/{}.run.mjs", name);
    let _ = fs::write(&wasm_path, &bytes);
    let _ = fs::write(&html_path, wasm::web_html(&name));
    let _ = fs::write(&mjs_path, wasm::web_runner(&name));
    println!("[iks] running {} with Node.js", mjs_path);
    match Command::new("node").arg(&mjs_path).status() {
        Ok(status) => exit(status.code().unwrap_or(1)),
        Err(_) => {
            eprintln!("ik: could not run 'node'; install Node.js and add it to PATH");
            eprintln!("   (the browser page is still available in build/)");
            exit(1);
        }
    }
}

fn cmd_test() {
    let target = host_target();
    let c_path = compile_to_c_with_target(target, true);
    let name = Path::new(&c_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let cc = resolve_compiler(target);
    let exe = match link_native(&cc, &c_path, &name, target, &BuildOpts::default()) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("ik: {}", e);
            exit(1);
        }
    };
    println!("[iks] running tests: {}", exe);
    match Command::new(&exe).status() {
        Ok(status) => exit(status.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("ik: failed to run '{}': {}", exe, e);
            exit(1);
        }
    }
}

fn tool_first_line(cmd: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(cmd).args(args).output().ok()?;
    let s = String::from_utf8_lossy(&o.stdout);
    let line = s.lines().next().unwrap_or("").trim().to_string();
    if line.is_empty() {
        None
    } else {
        Some(line)
    }
}

fn doctor_row(ok: bool, label: &str, detail: &str, hint: &str) -> bool {
    if ok {
        println!("  \u{2713} {:<26} {}", label, detail);
    } else {
        println!(
            "  \u{2717} {:<26} {}",
            label,
            if detail.is_empty() { "not found" } else { detail }
        );
        if !hint.is_empty() {
            println!("      hint: {}", hint);
        }
    }
    ok
}

fn cmd_doctor() {
    println!("ik doctor - toolchain health check");
    println!();
    let host = host_target();
    let mut issues = 0;

    let gcc = tool_first_line("gcc", &["--version"]);
    if !doctor_row(
        gcc.is_some(),
        "C compiler (gcc)",
        gcc.as_deref().unwrap_or("not found"),
        "install GCC: MinGW-w64/MSYS2 on Windows, build-essential on Linux",
    ) {
        issues += 1;
    }

    let win_cross = tool_first_line("x86_64-w64-mingw32-gcc", &["--version"]).is_some();
    let win_ok = win_cross || (host == Target::Windows && gcc.is_some());
    if !doctor_row(
        win_ok,
        "windows target",
        if win_ok {
            if win_cross {
                "mingw cross-gcc"
            } else {
                "host gcc"
            }
        } else {
            "no cross-gcc"
        },
        "MSYS2: pacman -S mingw-w64-x86_64-gcc",
    ) {
        issues += 1;
    }

    let wsl_ok = wsl_gcc_available();
    let linux_ok;
    let linux_detail;
    if host == Target::Linux && gcc.is_some() {
        linux_ok = true;
        linux_detail = "native gcc".to_string();
    } else if let Some(v) = tool_first_line("x86_64-linux-gnu-gcc", &["--version"]) {
        linux_ok = true;
        linux_detail = v;
    } else if wsl_ok {
        linux_ok = true;
        linux_detail = "WSL gcc (real Linux ELF)".to_string();
    } else if gcc.is_some() {
        linux_ok = false;
        linux_detail = "host gcc only (emits PE, not ELF)".to_string();
    } else {
        linux_ok = false;
        linux_detail = "not found".to_string();
    }
    if !doctor_row(
        linux_ok,
        "linux target",
        &linux_detail,
        "install WSL: wsl --install   (or x86_64-linux-gnu-gcc)",
    ) {
        issues += 1;
    }

    if wsl_ok {
        let out = Command::new("wsl")
            .args([
                "-e",
                "bash",
                "-lc",
                "ls /usr/include/X11/Xlib.h /usr/include/GL/glx.h 2>/dev/null | wc -l",
            ])
            .output()
            .ok();
        let n = out
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<i32>().unwrap_or(0))
            .unwrap_or(0);
        if !doctor_row(
            n >= 2,
            "X11/GL headers (WSL)",
            if n >= 2 { "Xlib.h, glx.h" } else { "missing" },
            "sudo apt install libx11-dev libgl-dev",
        ) {
            issues += 1;
        }
    }

    let ndk = detect_android_tc();
    let ndk_detail = ndk
        .as_ref()
        .map(|t| t.ndk.to_string_lossy().to_string())
        .unwrap_or_else(|| "not found".to_string());
    if !doctor_row(
        ndk.is_some(),
        "Android NDK",
        &ndk_detail,
        "set ANDROID_NDK_HOME or install via Android Studio > SDK Manager",
    ) {
        issues += 1;
    }

    let (sdk_ok, sdk_detail) = match detect_android_sdk() {
        Some(s) => {
            let bt = find_build_tools(&s);
            let jar = find_android_jar(&s);
            let bts = bt
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "missing".to_string());
            (
                bt.is_some() && jar.is_some(),
                format!(
                    "build-tools {}, platform {}",
                    bts,
                    if jar.is_some() { "ok" } else { "missing" }
                ),
            )
        }
        None => (false, "not found".to_string()),
    };
    if !doctor_row(
        sdk_ok,
        "Android SDK",
        &sdk_detail,
        "set ANDROID_HOME (needs build-tools + platforms)",
    ) {
        issues += 1;
    }

    let kt = find_keytool();
    let kt_detail = kt
        .as_deref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "not found".to_string());
    if !doctor_row(
        kt.is_some(),
        "Java keytool (APK sign)",
        &kt_detail,
        "install a JDK (Android Studio bundles one)",
    ) {
        issues += 1;
    }

    let node = tool_first_line("node", &["--version"]);
    if !doctor_row(
        node.is_some(),
        "Node.js (web target)",
        node.as_deref().unwrap_or("not found"),
        "install Node.js and add it to PATH",
    ) {
        issues += 1;
    }

    let wr = tool_first_line("windres", &["--version"]);
    if !doctor_row(
        wr.is_some(),
        "windres (-icon)",
        wr.as_deref().unwrap_or("not found"),
        "install MinGW binutils (MSYS2: pacman -S mingw-w64-x86_64-binutils)",
    ) {
        issues += 1;
    }

    let vk = detect_vulkan_sdk();
    if !doctor_row(
        vk.is_some(),
        "Vulkan SDK (optional)",
        vk.as_deref().unwrap_or("not found"),
        "install the LunarG Vulkan SDK for vk_* graphics",
    ) {
        issues += 1;
    }

    println!();
    if issues == 0 {
        println!("  all checks passed");
    } else {
        println!("  {} issue(s) found", issues);
        exit(1);
    }
}

fn cmd_clean() {
    let mut removed = Vec::new();
    for dir in ["build", "LogsCompl"] {
        let p = Path::new(dir);
        if p.exists() {
            match fs::remove_dir_all(p) {
                Ok(()) => removed.push(dir),
                Err(e) => {
                    eprintln!("ik: cannot remove '{}': {}", dir, e);
                    exit(1);
                }
            }
        }
    }
    if removed.is_empty() {
        println!("[iks] nothing to clean");
    } else {
        for r in &removed {
            println!("[iks] removed {}/", r);
        }
        println!("[*]");
    }
}
