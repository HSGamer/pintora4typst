use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    let out_dir = env::var("OUT_DIR").context("OUT_DIR not set")?;
    let out_dir_path = Path::new(&out_dir);
    let js_dir = Path::new("js");

    // 1. Prepare JS source
    let js_path = out_dir_path.join("pintora.js");
    let bc_path = out_dir_path.join("pintora.bc");

    eprintln!("Patching JS file...");
    let runtime_esm = fs::read_to_string(js_dir.join("runtime.esm.js"))
        .context("Failed to read runtime.esm.js")?;
    let runtime_patched = runtime_esm.replace("import.meta.url", "\"\"");
    fs::write(&js_path, runtime_patched).context("Failed to write pintora.js")?;
    println!("cargo:rerun-if-changed=js/runtime.esm.js");

    // 2. Compile to Bytecode using rquickjs
    eprintln!("Compiling to QuickJS bytecode natively...");
    compile_to_bytecode(&js_path, &bc_path)?;
    Ok(())
}

fn compile_to_bytecode(in_path: &Path, out_path: &Path) -> Result<()> {
    let source = fs::read_to_string(in_path).context("Failed to read concatenated pintora.js")?;

    let rt = rquickjs::Runtime::new().context("Failed to create QuickJS runtime")?;
    let ctx = rquickjs::Context::full(&rt).context("Failed to create QuickJS context")?;

    ctx.with(|ctx| -> Result<()> {
        let module = rquickjs::Module::declare(ctx.clone(), "pintora.js", source)
            .context("Failed to declare QuickJS module")?;
        let options = rquickjs::module::WriteOptions {
            strip_debug: true,
            strip_source: true,
            ..Default::default()
        };

        let bytecode = module
            .write(options)
            .context("Failed to compile module to bytecode")?;

        fs::write(out_path, bytecode).context("Failed to write pintora.bc")?;
        Ok(())
    })?;

    Ok(())
}
