use regex::Regex;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir_path = Path::new(&out_dir);
    let js_dir = Path::new("js");

    // 1. Prepare JS source
    let js_path = out_dir_path.join("pintora.js");
    let bc_path = out_dir_path.join("pintora.bc");

    eprintln!("Concatenating JS files...");
    get_js_source(js_dir, &js_path);

    // 2. Compile to Bytecode using rquickjs
    eprintln!("Compiling to QuickJS bytecode natively...");
    compile_to_bytecode(&js_path, &bc_path);
}

fn compile_to_bytecode(in_path: &Path, out_path: &Path) {
    let source = fs::read_to_string(in_path).expect("Failed to read concatenated pintora.js");

    let rt = rquickjs::Runtime::new().expect("Failed to create QuickJS runtime");
    let ctx = rquickjs::Context::full(&rt).expect("Failed to create QuickJS context");

    ctx.with(|ctx| {
        let module = rquickjs::Module::declare(ctx.clone(), "pintora.js", source)
            .expect("Failed to declare QuickJS module");
        let mut options = rquickjs::module::WriteOptions::default();
        options.strip_debug = true;
        options.strip_source = true;

        let bytecode = module
            .write(options)
            .expect("Failed to compile module to bytecode");

        fs::write(out_path, bytecode).expect("Failed to write pintora.bc");
    });
}

/// Get the JS source and write to file
fn get_js_source(js_dir: &Path, out_path: &Path) {
    let runtime_esm =
        fs::read_to_string(js_dir.join("runtime.esm.js")).expect("Failed to read runtime.esm.js");

    let runtime_patched = runtime_esm.replace("import.meta.url", "\"\"");
    let export_re = Regex::new(r"(?s)export\s*\{.*\}").unwrap();
    let runtime_patched = export_re.replace_all(&runtime_patched, "//EXPORTS aren't SUPPORTED");

    let render_fn = fs::read_to_string(js_dir.join("render.js")).expect("Failed to read render.js");

    let full_js = format!("{}\n{}", runtime_patched, render_fn);

    fs::write(out_path, full_js).expect("Failed to write pintora.js");

    println!("cargo:rerun-if-changed=js/runtime.esm.js");
    println!("cargo:rerun-if-changed=js/render.js");
}
