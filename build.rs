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

    eprintln!("Concatenating JS files...");
    get_js_source(js_dir, &js_path);

    // removed compile_to_bytecode
}

/// Get the JS source and write to file
fn get_js_source(js_dir: &Path, out_path: &Path) {
    let runtime_esm =
        fs::read_to_string(js_dir.join("runtime.esm.js")).expect("Failed to read runtime.esm.js");

    let runtime_patched = runtime_esm.replace("import.meta.url", "\"\"");
    let export_re = Regex::new(r"(?s)export\s*\{.*\}").unwrap();
    let runtime_patched = export_re.replace_all(&runtime_patched, "//EXPORTS aren't SUPPORTED");

    // ConsoleStub must be in the module source because render.js references it directly
    let console_stub =
        fs::read_to_string(js_dir.join("console.js")).expect("Failed to read console.js");
    let render_fn = fs::read_to_string(js_dir.join("render.js")).expect("Failed to read render.js");

    let full_js = format!("{}\n{}\n{}", console_stub, runtime_patched, render_fn);

    fs::write(out_path, full_js).expect("Failed to write pintora.js");

    println!("cargo:rerun-if-changed=js/runtime.esm.js");
    println!("cargo:rerun-if-changed=js/console.js");
    println!("cargo:rerun-if-changed=js/render.js");
}
