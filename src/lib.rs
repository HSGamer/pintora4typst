use anyhow::{Context, Result};
use quickjs_wasm_rs::{to_qjs_value, JSContextRef, JSValue};
use wasm_minimal_protocol::*;

initiate_protocol!();

const PINTORA_BYTECODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pintora.bc"));

thread_local! {
    static JS_CTX: JSContextRef = {
        let context = JSContextRef::default();
        context
            .eval_binary(PINTORA_BYTECODE)
            .expect("failed to load pintora bytecode");
        context
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
    JS_CTX.with(|context| {
        // Get the PintoraRender function from global scope
        let global_this = context
            .global_object()
            .context("failed to get global object")?;
        let function = global_this
            .get_property("PintoraRender")
            .context("failed to get PintoraRender function")?;

        // Convert arguments to JS values
        let src = std::str::from_utf8(src).context("src is not valid utf8")?;
        let style = std::str::from_utf8(style).context("style is not valid utf8")?;
        let font = std::str::from_utf8(font).context("font is not valid utf8")?;

        let js_src = to_qjs_value(context, &JSValue::String(src.to_string()))
            .context("failed to convert src to JSValue")?;
        let js_style = to_qjs_value(context, &JSValue::String(style.to_string()))
            .context("failed to convert style to JSValue")?;
        let js_font = to_qjs_value(context, &JSValue::String(font.to_string()))
            .context("failed to convert font to JSValue")?;

        // Call PintoraRender(src, style, font) natively
        let result = function
            .call(&global_this, &[js_src, js_style, js_font])
            .context("failed to call PintoraRender")?;

        let svg = result
            .as_str()
            .context("PintoraRender did not return a string")?;

        Ok(svg.as_bytes().to_vec())
    })
}
