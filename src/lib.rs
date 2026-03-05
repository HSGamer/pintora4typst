use anyhow::{Context, Result};
use wasm_minimal_protocol::*;

initiate_protocol!();

const PINTORA_BYTECODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pintora.bc"));

thread_local! {
    static JS_ENV: (rquickjs::Runtime, rquickjs::Context) = {
        let rt = rquickjs::Runtime::new().expect("failed to create runtime");
        let ctx = rquickjs::Context::full(&rt).expect("failed to create context");

        ctx.with(|ctx| {
            // Load and evaluate the pre-compiled bytecode module
            let loaded_mod = unsafe { rquickjs::Module::load(ctx.clone(), PINTORA_BYTECODE) }
                .expect("failed to load pintora bytecode");
            loaded_mod.eval().expect("failed to evaluate pintora module");
        });

        (rt, ctx)
    };
}

/// Render a Pintora diagram to SVG.
///
/// Arguments:
/// - `src`: The Pintora diagram source code
/// - `style`: Theme style (e.g. "default", "larkLight", "larkDark", "dark")
/// - `font`: Font family to use
///
/// Returns the SVG string.
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
