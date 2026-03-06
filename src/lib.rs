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

#[derive(Trace, JsLifetime, Default)]
#[rquickjs::class]
pub struct TextEncoder {}

#[rquickjs::methods]
impl TextEncoder {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self::default()
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

            let console = Object::new(ctx.clone()).expect("failed to create console object");
            console.set("log", Function::new(ctx.clone(), native_console_log).expect("failed to bind console.log")).expect("failed to set console.log");
            console.set("warn", Function::new(ctx.clone(), native_console_warn).expect("failed to bind console.warn")).expect("failed to set console.warn");
            console.set("error", Function::new(ctx.clone(), native_console_error).expect("failed to bind console.error")).expect("failed to set console.error");
            globals.set("console", console).expect("failed to set global console");

            // 1.5. Polyfill Uint8Array.fromBase64
            let uint8array: Object = globals.get("Uint8Array").expect("failed to get Uint8Array");
            uint8array.set("fromBase64", Function::new(ctx.clone(), native_uint8array_from_base64).expect("failed to bind fromBase64")).expect("failed to set fromBase64");

            let loaded_mod = unsafe { rquickjs::Module::load(ctx.clone(), PINTORA_BYTECODE) }
                .expect("failed to load pintora bytecode");

            let eval_res = loaded_mod.eval().expect("failed to evaluate pintora bytecode");
            let namespace = eval_res.0.namespace().expect("failed to get module namespace");

            let render_fn: Function = namespace.get("render").expect("Failed to find render export");
            globals.set("render", render_fn).expect("failed to set global render");
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
                .get("render")
                .context("failed to get render function")?;

            let opts =
                rquickjs::Object::new(ctx.clone()).context("failed to create opts object")?;
            opts.set("code", src_str).context("failed to set code")?;

            if !style_str.is_empty() || !font_str.is_empty() {
                let theme_variables = rquickjs::Object::new(ctx.clone())
                    .context("failed to create theme Variables")?;
                if !style_str.is_empty() {
                    let json_parse: rquickjs::Function = globals
                        .get::<_, rquickjs::Object>("JSON")
                        .context("failed to get JSON object")?
                        .get("parse")
                        .context("failed to get JSON.parse")?;
                    if let Ok(style_obj) = json_parse.call::<_, rquickjs::Object>((style_str,)) {
                        let object_assign: rquickjs::Function = globals
                            .get::<_, rquickjs::Object>("Object")
                            .context("failed to get Object object")?
                            .get("assign")
                            .context("failed to get Object.assign")?;
                        let _ = object_assign.call::<_, ()>((theme_variables.clone(), style_obj));
                    }
                }
                if !font_str.is_empty() {
                    theme_variables
                        .set("fontFamily", font_str)
                        .context("failed to set fontFamily")?;
                }
                let theme_config =
                    rquickjs::Object::new(ctx.clone()).context("failed to create themeConfig")?;
                theme_config
                    .set("themeVariables", theme_variables)
                    .context("failed to set themeVariables")?;
                let pintora_config =
                    rquickjs::Object::new(ctx.clone()).context("failed to create pintoraConfig")?;
                pintora_config
                    .set("themeConfig", theme_config)
                    .context("failed to set themeConfig")?;

                opts.set("pintoraConfig", pintora_config)
                    .context("failed to set pintoraConfig to opts")?;
            }

            let promise: rquickjs::Promise = render_fn.call((opts,)).map_err(|e| {
                let mut msg = format!("failed to call render_fn: {:?}", e);
                if let Some(js_error) = ctx.catch().into_exception() {
                    msg = format!(
                        "JS Exception: {} \nStack: {}",
                        js_error.message().unwrap_or_default(),
                        js_error.stack().unwrap_or_default()
                    );
                }
                anyhow::anyhow!(msg)
            })?;

            let res_obj: rquickjs::Object = promise.finish().map_err(|e| {
                let mut msg = format!("failed to finish promise: {:?}", e);
                if let Some(js_error) = ctx.catch().into_exception() {
                    msg = format!(
                        "JS Exception in Promise: {} \nStack: {}",
                        js_error.message().unwrap_or_default(),
                        js_error.stack().unwrap_or_default()
                    );
                }
                anyhow::anyhow!(msg)
            })?;

            let data: String = res_obj
                .get("data")
                .map_err(|_| anyhow::anyhow!("render result missing 'data' field"))?;
            Ok(data.into_bytes())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_module() {
        let rt = rquickjs::Runtime::new().expect("failed to init test runtime");
        let ctx = rquickjs::Context::full(&rt).expect("failed to init test context");

        ctx.with(|ctx| {
            let globals = ctx.globals();

            let console = Object::new(ctx.clone()).expect("failed to create console object");
            console
                .set(
                    "log",
                    Function::new(ctx.clone(), native_console_log)
                        .expect("failed to bind console.log"),
                )
                .expect("failed to set console.log");
            console
                .set(
                    "warn",
                    Function::new(ctx.clone(), native_console_warn)
                        .expect("failed to bind console.warn"),
                )
                .expect("failed to set console.warn");
            console
                .set(
                    "error",
                    Function::new(ctx.clone(), native_console_error)
                        .expect("failed to bind console.error"),
                )
                .expect("failed to set console.error");
            globals
                .set("console", console)
                .expect("failed to set global console");

            let uint8array: Object = globals.get("Uint8Array").expect("failed to get Uint8Array");
            uint8array
                .set(
                    "fromBase64",
                    Function::new(ctx.clone(), native_uint8array_from_base64)
                        .expect("failed to bind fromBase64"),
                )
                .expect("failed to set fromBase64");

            Class::<TextEncoder>::define(&globals).expect("failed to define TextEncoder");
            Class::<TextDecoder>::define(&globals).expect("failed to define TextDecoder");

            // Sanity test module evaluation errors
            let throw_mod = rquickjs::Module::declare(
                ctx.clone(),
                "throw.js",
                "throw new Error('test error');",
            )
            .expect("failed to declare module");
            let _ = throw_mod.eval();
            let err = ctx.catch();
            if err.is_exception() {
                println!(
                    "throw_mod correctly threw: {:?}",
                    err.as_exception()
                        .expect("failed to get exception")
                        .message()
                );
            } else {
                println!("throw_mod did NOT throw!");
            }

            let loaded_mod = unsafe { rquickjs::Module::load(ctx.clone(), PINTORA_BYTECODE) }
                .expect("failed to load bytecode");

            let eval_res = loaded_mod
                .eval()
                .expect("failed to evaluate pintora bytecode");
            let namespace = eval_res
                .0
                .namespace()
                .expect("failed to extract export namespace");

            let render_fn: Function = namespace
                .get("render")
                .expect("Failed to find render export");

            globals
                .set("render", render_fn)
                .expect("failed to set global render");

            let pr: Function = globals.get("render").expect("render not found");
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
        let result = render(src.as_bytes(), style.as_bytes(), font.as_bytes())
            .expect("Expected UTF-8 render to complete");
        println!("Render result: {}", String::from_utf8_lossy(&result));
    }
}
