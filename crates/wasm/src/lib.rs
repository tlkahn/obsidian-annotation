use wasm_bindgen::prelude::*;
use annotation_core::parser;

#[wasm_bindgen]
pub fn parse_annotations(content: &str) -> String {
    serde_json::to_string(&parser::parse_annotations(content)).unwrap_or_default()
}
