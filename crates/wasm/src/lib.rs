use wasm_bindgen::prelude::*;
use annotation_core::marks;
use annotation_core::parser;
use annotation_core::types::Scope;
use annotation_core::scope_resolver::{self, ResolutionMode};

/// Parse annotations. `custom_marks_json` is a JSON array of custom mark
/// codes (from `.lit/marks.toml`); pass "[]" when none are defined.
#[wasm_bindgen]
pub fn parse_annotations(content: &str, custom_marks_json: &str) -> String {
    let custom: Vec<String> = serde_json::from_str(custom_marks_json).unwrap_or_default();
    serde_json::to_string(&parser::parse_annotations_with_marks(content, &custom))
        .unwrap_or_default()
}

/// Parse a `.lit/marks.toml` document. Returns a JSON object keyed by mark
/// code ({label, icon?, style}), or "null" for invalid TOML.
#[wasm_bindgen]
pub fn parse_marks_toml(input: &str) -> String {
    match marks::parse_marks_toml(input) {
        Some(defs) => serde_json::to_string(&defs).unwrap_or_else(|_| "null".to_string()),
        None => "null".to_string(),
    }
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
    let Some(mode) = ResolutionMode::from_str(mode) else {
        return "null".to_string();
    };
    match scope_resolver::resolve_scope_range(content, char_start, char_end, &scope, lang, mode) {
        Some((start, end)) => format!("{{\"start\":{},\"end\":{}}}", start, end),
        None => "null".to_string(),
    }
}
