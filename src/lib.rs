use anyhow::{Context, Result};
use rquickjs::prelude::{Opt, Rest};
use rquickjs::{Ctx, Function, TypedArray, Value};
use wasm_minimal_protocol::*;

initiate_protocol!();

const PINTORA_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/pintora.js"));

// ─── Native Polyfills via Functions ──────────────────────────────────────────

fn native_encode<'js>(ctx: Ctx<'js>, string: Opt<Value<'js>>) -> rquickjs::Result<Value<'js>> {
    if let Some(string) = string.0 {
        if let Some(string) = string.as_string() {
            let string = string.to_string()?;
            return TypedArray::new(ctx.clone(), string.as_bytes())
                .map(|m: TypedArray<'_, u8>| m.into_value());
        }
    }
    TypedArray::new(ctx.clone(), []).map(|m: TypedArray<'_, u8>| m.into_value())
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

// Override console to print to stdout so we can see any JS errors
globalThis.console = {
    log: function(...args) { std_print("LOG:", ...args); },
    error: function(...args) { std_print("ERROR:", ...args); },
    warn: function(...args) { std_print("WARN:", ...args); }
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

            // Evaluate the JS polyfills
            let _: () = ctx.eval(JS_POLYFILLS).expect("failed to eval polyfills");

            // 2. Evaluate the concatenated JS script
            let _: () = ctx.eval(PINTORA_JS).expect("failed to evaluate pintora js");
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

            let print_fn = Function::new(ctx.clone(), |args: Rest<Value<'_>>| {
                for arg in args.0 {
                    if let Some(s) = arg.as_string() {
                        print!("{} ", s.to_string().unwrap_or_default());
                    } else if arg.is_error() {
                        let err = arg.as_object().unwrap();
                        let msg: String = err.get::<_, String>("message").unwrap_or_default();
                        let stack: String = err.get::<_, String>("stack").unwrap_or_default();
                        print!("Error: {}\nStack: {}", msg, stack);
                    } else {
                        print!("{:?} ", arg);
                    }
                }
                println!();
            })
            .unwrap();
            globals.set("std_print", print_fn).unwrap();

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

            let script = format!(
                "try {{\n{}\n}} catch (e) {{ std_print('CAUGHT: ', e, e.stack); throw e; }}",
                PINTORA_JS
            );
            match ctx.eval::<(), _>(script.as_str()) {
                Ok(_) => println!("Pintora JS evaluated successfully!"),
                Err(e) => {
                    println!("Eval failed with Error: {:?}", e);
                    panic!("Evaluation failed");
                }
            }
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
}
