use wasm_bindgen::prelude::*;
use annotation_core::parser;
use annotation_core::types::Scope;
use annotation_core::scope_resolver::{self, ResolutionMode};

#[wasm_bindgen]
pub fn parse_annotations(content: &str) -> String {
    serde_json::to_string(&parser::parse_annotations(content)).unwrap_or_default()
}

#[wasm_bindgen]
pub fn resolve_scope_range(
    content: &str,
    char_start: usize,
    char_end: usize,
    scope_json: &str,
    lang: &str,
    mode: &str,
) -> String {
    let scope: Scope = match serde_json::from_str(scope_json) {
        Ok(s) => s,
        Err(_) => return "null".to_string(),
    };
    let mode = ResolutionMode::from_str(mode);
    match scope_resolver::resolve_scope_range(content, char_start, char_end, &scope, lang, mode) {
        Some((start, end)) => format!("{{\"start\":{},\"end\":{}}}", start, end),
        None => "null".to_string(),
    }
}
