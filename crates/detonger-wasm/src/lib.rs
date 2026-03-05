use detonger_protocol::{PaperType, PrintOptions, PrinterCaps};
use js_sys::{Array, Uint8Array};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PaperTypeInput {
    Continuous,
    Gap,
}

impl PaperTypeInput {
    fn to_protocol(&self) -> PaperType {
        match self {
            Self::Continuous => PaperType::Continuous,
            Self::Gap => PaperType::Gap,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncodeOptionsInput {
    threshold: Option<u8>,
    x_offset_dots: Option<i16>,
    print_width_dots: Option<u16>,
    paper_type: Option<PaperTypeInput>,
}

impl EncodeOptionsInput {
    fn to_protocol(&self) -> (PrinterCaps, PrintOptions) {
        let caps = PrinterCaps {
            print_width_dots: self
                .print_width_dots
                .unwrap_or(PrinterCaps::default().print_width_dots),
            ..PrinterCaps::default()
        };
        let opts = PrintOptions {
            threshold: self.threshold.unwrap_or(PrintOptions::default().threshold),
            x_offset_dots: self
                .x_offset_dots
                .unwrap_or(PrintOptions::default().x_offset_dots),
            paper_type: self
                .paper_type
                .as_ref()
                .map(PaperTypeInput::to_protocol)
                .unwrap_or(PrintOptions::default().paper_type),
        };
        (caps, opts)
    }
}

#[wasm_bindgen(js_name = "encodePngJobMessages")]
pub fn encode_png_job_messages(png: &[u8], options: JsValue) -> Result<Array, JsValue> {
    let options: EncodeOptionsInput = serde_wasm_bindgen::from_value(options)
        .map_err(|err| JsValue::from_str(&format!("invalid options: {err}")))?;
    let (caps, opts) = options.to_protocol();

    let messages = detonger_protocol::encode_png_job_messages(png, &caps, &opts)
        .map_err(|err| JsValue::from_str(&err.to_string()))?;

    Ok(messages_to_js(messages))
}

#[wasm_bindgen(js_name = "encodeWidthTestMessages")]
pub fn encode_width_test_messages(options: JsValue) -> Result<Array, JsValue> {
    let options: EncodeOptionsInput = serde_wasm_bindgen::from_value(options)
        .map_err(|err| JsValue::from_str(&format!("invalid options: {err}")))?;
    let (caps, opts) = options.to_protocol();

    let messages = detonger_protocol::encode_width_test_job_messages(&caps, &opts)
        .map_err(|err| JsValue::from_str(&err.to_string()))?;

    Ok(messages_to_js(messages))
}

#[wasm_bindgen(js_name = "renderWidthTestPng")]
pub fn render_width_test_png(options: JsValue, scale: u32) -> Result<Uint8Array, JsValue> {
    let options: EncodeOptionsInput = serde_wasm_bindgen::from_value(options)
        .map_err(|err| JsValue::from_str(&format!("invalid options: {err}")))?;
    let (caps, opts) = options.to_protocol();

    let png = detonger_protocol::render_width_test_png(&caps, &opts, scale)
        .map_err(|err| JsValue::from_str(&err.to_string()))?;
    Ok(Uint8Array::from(png.as_slice()))
}

#[wasm_bindgen(js_name = "protocolVersion")]
pub fn protocol_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn messages_to_js(messages: Vec<Vec<u8>>) -> Array {
    let out = Array::new();
    for msg in messages {
        out.push(&Uint8Array::from(msg.as_slice()));
    }
    out
}
