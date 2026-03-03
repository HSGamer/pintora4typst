use regex::Regex;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir_path = Path::new(&out_dir);
    let js_dir = Path::new("js");

    // Step 1: Get the JS source
    let js_path = out_dir_path.join("pintora.js");
    get_js_source(js_dir, &js_path);

    // Step 2: Build the native QuickJS bytecode compiler
    let js2bc_path = build_js2bc(&out_dir);

    // Step 3: Compile JS to bytecode
    let bc_path = out_dir_path.join("pintora.bc");
    eprintln!("Compiling JS to QuickJS bytecode...");
    let output = Command::new(&js2bc_path)
        .arg(&js_path)
        .arg(&bc_path)
        .output()
        .expect("Failed to run js2bc");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("js2bc failed: {}", stderr);
    }
}

/// Build the native js2bc helper from QuickJS C sources.
fn build_js2bc(out_dir: &str) -> PathBuf {
    let quickjs_dir = find_quickjs_sources();

    eprintln!(
        "Building native js2bc from QuickJS sources at: {}",
        quickjs_dir.display()
    );

    let mut build = cc::Build::new();
    build
        .file("js2bc.c")
        .file(quickjs_dir.join("quickjs.c"))
        .file(quickjs_dir.join("cutils.c"))
        .file(quickjs_dir.join("libbf.c"))
        .file(quickjs_dir.join("libregexp.c"))
        .file(quickjs_dir.join("libunicode.c"))
        .include(&quickjs_dir)
        .define("_GNU_SOURCE", None)
        .define("CONFIG_VERSION", "\"2021-03-27\"")
        .define("CONFIG_BIGNUM", None)
        .opt_level(2)
        .warnings(false)
        .target(&env::var("HOST").unwrap());

    let objects = build.compile_intermediates();

    let js2bc_path = Path::new(out_dir).join("js2bc");
    let mut link_cmd = Command::new(
        env::var("HOST_CC")
            .or_else(|_| env::var("CC_host"))
            .unwrap_or_else(|_| "cc".to_string()),
    );
    for obj in &objects {
        link_cmd.arg(obj);
    }
    link_cmd.arg("-o").arg(&js2bc_path).arg("-lm");

    let output = link_cmd.output().expect("Failed to link js2bc");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Failed to link js2bc: {}", stderr);
    }

    println!("cargo:rerun-if-changed=js2bc.c");
    js2bc_path
}

/// Find the QuickJS C sources from the quickjs-wasm-sys crate.
fn find_quickjs_sources() -> PathBuf {
    let home = env::var("CARGO_HOME")
        .or_else(|_| env::var("HOME").map(|h| format!("{}/.cargo", h)))
        .expect("Cannot determine CARGO_HOME");

    let registry = Path::new(&home).join("registry/src");

    for entry in fs::read_dir(&registry).expect("Cannot read cargo registry") {
        let entry = entry.unwrap();
        let index_dir = entry.path();

        if index_dir.is_dir() {
            for crate_entry in fs::read_dir(&index_dir).expect("Cannot read index dir") {
                let crate_entry = crate_entry.unwrap();
                let crate_dir = crate_entry.path();
                if crate_dir.is_dir() {
                    if let Some(name) = crate_dir.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("quickjs-wasm-sys-") {
                            let qjs_dir = crate_dir.join("quickjs");
                            if qjs_dir.exists() {
                                return qjs_dir;
                            }
                        }
                    }
                }
            }
        }
    }

    panic!("Cannot find quickjs-wasm-sys sources in cargo registry.");
}

/// Get the JS source and write to file
fn get_js_source(js_dir: &Path, out_path: &Path) {
    let encoding_indexes = fs::read_to_string(js_dir.join("encoding-indexes.js"))
        .expect("Failed to read encoding-indexes.js");
    let encoding =
        fs::read_to_string(js_dir.join("encoding.js")).expect("Failed to read encoding.js");
    let runtime_esm =
        fs::read_to_string(js_dir.join("runtime.esm.js")).expect("Failed to read runtime.esm.js");

    let runtime_patched = runtime_esm.replace("import.meta.url", "\"\"");
    let export_re = Regex::new(r"(?s)export\s*\{.*\}").unwrap();
    let runtime_patched = export_re.replace_all(&runtime_patched, "//EXPORTS aren't SUPPORTED");

    let console_stub =
        fs::read_to_string(js_dir.join("console.js")).expect("Failed to read console.js");
    let render_fn = fs::read_to_string(js_dir.join("render.js")).expect("Failed to read render.js");

    let full_js = format!(
        "{}\n{}\n{}\n{}\n{}",
        console_stub, encoding_indexes, encoding, runtime_patched, render_fn
    );

    fs::write(out_path, full_js).expect("Failed to write pintora.js");

    println!("cargo:rerun-if-changed=js/encoding-indexes.js");
    println!("cargo:rerun-if-changed=js/encoding.js");
    println!("cargo:rerun-if-changed=js/runtime.esm.js");
    println!("cargo:rerun-if-changed=js/console.js");
    println!("cargo:rerun-if-changed=js/render.js");
}
