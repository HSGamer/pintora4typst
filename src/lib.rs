use anyhow::{Context, Result};
use rquickjs::prelude::{Opt, Rest};
use rquickjs::{Ctx, Function, TypedArray, Value};
use wasm_minimal_protocol::*;

initiate_protocol!();

const PINTORA_BYTECODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pintora.bc"));

// ─── Native Polyfills via Functions ──────────────────────────────────────────

fn native_encode<'js>(ctx: Ctx<'js>, string: Opt<Value<'js>>) -> rquickjs::Result<Value<'js>> {
    let mut bytes = Vec::new();
    if let Some(val) = string.0 {
        if let Ok(string_fn) = ctx.globals().get::<_, rquickjs::Function>("String") {
            if let Ok(s) = string_fn.call::<_, rquickjs::String>((val,)) {
                if let Ok(rust_str) = s.to_string() {
                    bytes.extend_from_slice(rust_str.as_bytes());
                }
            }
        }
    }
    TypedArray::new(ctx.clone(), bytes).map(|m| m.into_value())
}

fn native_decode<'js>(
    bytes: Opt<TypedArray<'js, u8>>,
    encoding: Opt<String>,
) -> rquickjs::Result<String> {
    let bytes = match bytes.0 {
        Some(b) => b,
        None => return Ok(String::new()),
    };
    let encoding = encoding
        .0
        .unwrap_or_else(|| "utf-8".to_string())
        .to_lowercase();

    let bytes_slice = bytes.as_bytes().unwrap_or(&[]);

    if encoding == "ascii" || encoding == "us-ascii" {
        return Ok(bytes_slice.iter().map(|&b| (b & 0x7F) as char).collect());
    }

    String::from_utf8(bytes_slice.to_vec())
        .map_err(|e| rquickjs::Error::new_from_js_message("bytes", "string", &e.to_string()))
}

fn format_console_args<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> rquickjs::Result<String> {
    let mut out = String::new();
    let globals = ctx.globals();
    let string_fn: rquickjs::Function = globals.get("String")?;
    for (i, arg) in args.0.into_iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let s: rquickjs::String = string_fn.call((arg,))?;
        out.push_str(&s.to_string()?);
    }
    Ok(out)
}

fn native_console_log<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> rquickjs::Result<()> {
    let msg = format_console_args(ctx, args)?;
    println!("{}", msg);
    Ok(())
}

fn native_console_error<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> rquickjs::Result<()> {
    let msg = format_console_args(ctx, args)?;
    eprintln!("{}", msg);
    Ok(())
}

fn native_console_warn<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> rquickjs::Result<()> {
    let msg = format_console_args(ctx.clone(), args)?;
    eprintln!("{}", msg);
    ctx.globals().set("_pintoraLastWarning", msg)?;
    Ok(())
}

const JS_POLYFILLS: &str = r#"
class TextEncoder {
    get encoding() { return "utf-8"; }
    encode(str) { return _RustTextEncoder_encode(str); }
}

class TextDecoder {
    constructor(label) {
        this._encoding = (label || "utf-8").toLowerCase();
    }
    get encoding() { return this._encoding; }
    decode(bytes) { 
        if (!bytes) return "";
        var arr = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
        return _RustTextDecoder_decode(arr, this._encoding); 
    }
}

globalThis.TextEncoder = TextEncoder;
globalThis.TextDecoder = TextDecoder;

// Expose native Rust implementations for the console
globalThis.console = {
    log: function(...args) { _RustConsole_log(...args); },
    error: function(...args) { _RustConsole_error(...args); },
    warn: function(...args) { _RustConsole_warn(...args); }
};
"#;

// ─── Main WASM plugin ────────────────────────────────────────────────────────

thread_local! {
    static JS_ENV: (rquickjs::Runtime, rquickjs::Context) = {
        let rt = rquickjs::Runtime::new().expect("failed to create runtime");
        let ctx = rquickjs::Context::full(&rt).expect("failed to create context");

        ctx.with(|ctx| {
            // 1. Bind Rust functions to global context
            let globals = ctx.globals();

            let encode_fn = Function::new(ctx.clone(), native_encode)
                .expect("failed to create encode function");
            globals.set("_RustTextEncoder_encode", encode_fn)
                .expect("failed to set encode function");

            let decode_fn = Function::new(ctx.clone(), native_decode)
                .expect("failed to create decode function");
            globals.set("_RustTextDecoder_decode", decode_fn)
                .expect("failed to set decode function");

            let console_log_fn = Function::new(ctx.clone(), native_console_log).unwrap();
            globals.set("_RustConsole_log", console_log_fn).unwrap();
            let console_warn_fn = Function::new(ctx.clone(), native_console_warn).unwrap();
            globals.set("_RustConsole_warn", console_warn_fn).unwrap();
            let console_error_fn = Function::new(ctx.clone(), native_console_error).unwrap();
            globals.set("_RustConsole_error", console_error_fn).unwrap();

            // Evaluate the JS polyfills
            let _: () = ctx.eval(JS_POLYFILLS).expect("failed to eval polyfills");

            // 2. Load and evaluate the pre-compiled bytecode module
            let loaded_mod = unsafe { rquickjs::Module::load(ctx.clone(), PINTORA_BYTECODE) }
                .expect("failed to load pintora bytecode");
            let _ = loaded_mod.eval();
        });

        (rt, ctx)
    };
}

/// Render a Pintora diagram to SVG.
#[wasm_func]
fn render(src: &[u8], style: &[u8], font: &[u8]) -> Result<Vec<u8>> {
    let src_str = std::str::from_utf8(src).context("src is not valid utf8")?;
    let style_str = std::str::from_utf8(style).context("style is not valid utf8")?;
    let font_str = std::str::from_utf8(font).context("font is not valid utf8")?;

    JS_ENV.with(|(_, ctx)| {
        ctx.with(|ctx| {
            let globals = ctx.globals();
            let render_fn: rquickjs::Function = globals
                .get("PintoraRender")
                .context("failed to get PintoraRender function")?;

            let result: String = render_fn
                .call((src_str, style_str, font_str))
                .context("failed to call PintoraRender")?;

            Ok(result.into_bytes())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_module() {
        let rt = rquickjs::Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let globals = ctx.globals();

            let console_log_fn = Function::new(ctx.clone(), native_console_log).unwrap();
            globals.set("_RustConsole_log", console_log_fn).unwrap();
            let console_warn_fn = Function::new(ctx.clone(), native_console_warn).unwrap();
            globals.set("_RustConsole_warn", console_warn_fn).unwrap();
            let console_error_fn = Function::new(ctx.clone(), native_console_error).unwrap();
            globals.set("_RustConsole_error", console_error_fn).unwrap();

            let encode_fn = Function::new(ctx.clone(), native_encode).unwrap();
            globals.set("_RustTextEncoder_encode", encode_fn).unwrap();

            let decode_fn = Function::new(ctx.clone(), native_decode).unwrap();
            globals.set("_RustTextDecoder_decode", decode_fn).unwrap();

            ctx.eval::<(), _>(JS_POLYFILLS).unwrap();

            // Sanity test module evaluation errors
            let throw_mod = rquickjs::Module::declare(
                ctx.clone(),
                "throw.js",
                "throw new Error('test error');",
            )
            .unwrap();
            let _ = throw_mod.eval();
            let err = ctx.catch();
            if err.is_exception() {
                println!(
                    "throw_mod correctly threw: {:?}",
                    err.as_exception().unwrap().message()
                );
            } else {
                println!("throw_mod did NOT throw!");
            }

            let loaded_mod =
                unsafe { rquickjs::Module::load(ctx.clone(), PINTORA_BYTECODE) }.unwrap();

            let _ = loaded_mod.eval();
        });

        loop {
            // execute_pending_job returns Result<bool>
            match rt.execute_pending_job() {
                Ok(false) => break,
                Ok(true) => continue,
                Err(e) => {
                    println!("Pending jobs threw an error: {:?}", e);
                    break;
                }
            }
        }

        ctx.with(|ctx| {
            let globals = ctx.globals();
            println!("Eval succeeded!");
            let _render_fn: rquickjs::Function = globals
                .get("PintoraRender")
                .expect("Failed to get PintoraRender in test");
            println!("Successfully got PintoraRender function!");
        });
    }

    #[test]
    fn test_render_utf8() {
        let src = r#"
sequenceDiagram
  participant ユーザー
  participant サーバー
  ユーザー->>サーバー: 🚀 こんにちは!
  サーバー-->>ユーザー: サーバーからの応答
  @note left of ユーザー: 多言語サポート
"#;
        let style = "default";
        let font = "sans-serif";

        println!("Calling render with UTF-8...");
        let result = render(src.as_bytes(), style.as_bytes(), font.as_bytes()).unwrap();
        println!("Render result: {}", String::from_utf8_lossy(&result));
    }
}
