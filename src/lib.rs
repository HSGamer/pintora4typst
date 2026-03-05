use anyhow::{Context as _, Result};
use base64::{engine::general_purpose, Engine as _};
use rquickjs::class::Trace;
use rquickjs::{
    function::Rest, prelude::*, Class, Ctx, Function, JsLifetime, Object, TypedArray, Value,
};
use wasm_minimal_protocol::*;

initiate_protocol!();

const PINTORA_BYTECODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pintora.bc"));

// ─── Native Polyfills via Functions ──────────────────────────────────────────

#[derive(Trace, JsLifetime)]
#[rquickjs::class]
pub struct TextEncoder {}

#[rquickjs::methods]
impl TextEncoder {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[qjs(get)]
    pub fn encoding(&self) -> &'static str {
        "utf-8"
    }

    pub fn encode<'js>(
        &self,
        ctx: Ctx<'js>,
        string: Opt<rquickjs::Coerced<String>>,
    ) -> rquickjs::Result<Value<'js>> {
        let bytes = string.0.map(|s| s.0.into_bytes()).unwrap_or_default();
        TypedArray::new(ctx.clone(), bytes).map(|m| m.into_value())
    }
}

#[derive(Trace, JsLifetime)]
#[rquickjs::class]
pub struct TextDecoder {
    encoding: String,
}

#[rquickjs::methods]
impl TextDecoder {
    #[qjs(constructor)]
    pub fn new(label: Opt<rquickjs::Coerced<String>>) -> Self {
        let encoding = label
            .0
            .map(|s| s.0)
            .unwrap_or_else(|| "utf-8".to_string())
            .to_lowercase();
        Self { encoding }
    }

    #[qjs(get)]
    pub fn encoding(&self) -> String {
        self.encoding.clone()
    }

    pub fn decode<'js>(&self, ctx: Ctx<'js>, bytes: Opt<Value<'js>>) -> rquickjs::Result<String> {
        let Some(bytes_val) = bytes.0 else {
            return Ok(String::new());
        };

        let typed_array_res = TypedArray::<u8>::from_value(bytes_val.clone());
        let typed_array: TypedArray<'js, u8> = match typed_array_res {
            Ok(t) => t,
            Err(_) => {
                let uint8_array_ctor: rquickjs::Function = ctx.globals().get("Uint8Array")?;
                uint8_array_ctor.call((bytes_val,))?
            }
        };

        let bytes_slice = typed_array.as_bytes().unwrap_or(&[]);

        let enc_str = &self.encoding;
        if enc_str.eq_ignore_ascii_case("ascii") || enc_str.eq_ignore_ascii_case("us-ascii") {
            return Ok(bytes_slice.iter().map(|&b| (b & 0x7F) as char).collect());
        }

        Ok(String::from_utf8_lossy(bytes_slice).into_owned())
    }
}

fn format_console_args(args: Rest<rquickjs::Coerced<String>>) -> String {
    let mut out = String::new();
    for (i, arg) in args.0.into_iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&arg.0);
    }
    out
}

fn native_console_log(_args: Rest<rquickjs::Coerced<String>>) {
    // Suppressed to prevent wasi-stub fd_write panics
}

fn native_console_error(_args: Rest<rquickjs::Coerced<String>>) {
    // Suppressed to prevent wasi-stub fd_write panics
}

fn native_console_warn<'js>(
    ctx: Ctx<'js>,
    args: Rest<rquickjs::Coerced<String>>,
) -> rquickjs::Result<()> {
    let msg = format_console_args(args);
    ctx.globals().set("_pintoraLastWarning", msg)?;
    Ok(())
}

fn native_uint8array_from_base64<'js>(
    ctx: Ctx<'js>,
    b64: String,
) -> rquickjs::Result<rquickjs::TypedArray<'js, u8>> {
    let bytes = general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| rquickjs::Error::Unknown)?;
    TypedArray::new(ctx, bytes)
}

// ─── Main WASM plugin ────────────────────────────────────────────────────────

thread_local! {
    static JS_ENV: (rquickjs::Runtime, rquickjs::Context) = {
        let rt = rquickjs::Runtime::new().expect("failed to create runtime");
        // Increase GC threshold to 32MB to avoid constant GC pauses during immense AST generations in Pintora parsing
        rt.set_gc_threshold(32 * 1024 * 1024);
        let ctx = rquickjs::Context::full(&rt).expect("failed to create context");

        ctx.with(|ctx| {
            // 1. Bind Rust functions to global context
            let globals = ctx.globals();

            Class::<TextEncoder>::define(&globals).expect("failed to define TextEncoder");
            Class::<TextDecoder>::define(&globals).expect("failed to define TextDecoder");

            let console = Object::new(ctx.clone()).unwrap();
            console.set("log", Function::new(ctx.clone(), native_console_log).unwrap()).unwrap();
            console.set("warn", Function::new(ctx.clone(), native_console_warn).unwrap()).unwrap();
            console.set("error", Function::new(ctx.clone(), native_console_error).unwrap()).unwrap();
            globals.set("console", console).unwrap();

            // 1.5. Polyfill Uint8Array.fromBase64
            let uint8array: Object = globals.get("Uint8Array").expect("failed to get Uint8Array");
            uint8array.set("fromBase64", Function::new(ctx.clone(), native_uint8array_from_base64).unwrap()).unwrap();

            // 2. Load and evaluate the pre-compiled bytecode module
            let loaded_mod = unsafe { rquickjs::Module::load(ctx.clone(), PINTORA_BYTECODE) }
                .expect("failed to load pintora bytecode");

            loaded_mod.eval().expect("failed to evaluate pintora bytecode");

            let _ = globals.get::<_, Function>("PintoraRender").expect("failed to get PintoraRender function");
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

            let result_res: rquickjs::Result<String> =
                render_fn.call((src_str, style_str, font_str));
            let result = match result_res {
                Ok(r) => r,
                Err(e) => {
                    let mut msg = format!("failed to call PintoraRender: {:?}", e);
                    if let Some(js_error) = ctx.catch().into_exception() {
                        msg = format!(
                            "JS Exception in PintoraRender: {} \nStack: {}",
                            js_error.message().unwrap_or_default(),
                            js_error.stack().unwrap_or_default()
                        );
                    }
                    return Err(anyhow::anyhow!(msg));
                }
            };

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

            let console = Object::new(ctx.clone()).unwrap();
            console
                .set(
                    "log",
                    Function::new(ctx.clone(), native_console_log).unwrap(),
                )
                .unwrap();
            console
                .set(
                    "warn",
                    Function::new(ctx.clone(), native_console_warn).unwrap(),
                )
                .unwrap();
            console
                .set(
                    "error",
                    Function::new(ctx.clone(), native_console_error).unwrap(),
                )
                .unwrap();
            globals.set("console", console).unwrap();

            let uint8array: Object = globals.get("Uint8Array").expect("failed to get Uint8Array");
            uint8array
                .set(
                    "fromBase64",
                    Function::new(ctx.clone(), native_uint8array_from_base64).unwrap(),
                )
                .unwrap();

            Class::<TextEncoder>::define(&globals).expect("failed to define TextEncoder");
            Class::<TextDecoder>::define(&globals).expect("failed to define TextDecoder");

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

            loaded_mod
                .eval()
                .expect("failed to evaluate pintora bytecode");

            let pr: Function = globals
                .get("PintoraRender")
                .expect("PintoraRender not found");
            assert!(pr.as_value().is_function());
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
