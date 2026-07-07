use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

/// A custom mark definition from `.lit/marks.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkDefinition {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// CSS property → value map from the `[code.style]` table
    #[serde(default)]
    pub style: BTreeMap<String, String>,
}

/// Parse a `.lit/marks.toml` document into mark definitions keyed by code.
///
/// Returns `None` for invalid TOML. Lenient per entry: a missing label
/// defaults to the code, non-string style values are skipped, and codes the
/// annotation parser could never match (anything but ASCII lowercase
/// letters) are dropped.
pub fn parse_marks_toml(input: &str) -> Option<BTreeMap<String, MarkDefinition>> {
    let value: toml::Value = input.parse().ok()?;
    let table = value.as_table()?;

    let mut defs = BTreeMap::new();
    for (code, entry) in table {
        if code.is_empty() || !code.chars().all(|c| c.is_ascii_lowercase()) {
            continue;
        }
        let Some(entry) = entry.as_table() else {
            continue;
        };
        let label = entry
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or(code)
            .to_string();
        let icon = entry
            .get("icon")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let style = entry
            .get("style")
            .and_then(|v| v.as_table())
            .map(|t| {
                t.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        defs.insert(code.clone(), MarkDefinition { label, icon, style });
    }
    Some(defs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_spec_example() {
        let input = r#"
[mymark]
label = "my custom mark"
icon = "M"
[mymark.style]
color = "purple"
font-weight = "bold"
"#;
        let defs = parse_marks_toml(input).unwrap();
        let def = defs.get("mymark").unwrap();
        assert_eq!(def.label, "my custom mark");
        assert_eq!(def.icon, Some("M".to_string()));
        assert_eq!(def.style.get("color"), Some(&"purple".to_string()));
        assert_eq!(def.style.get("font-weight"), Some(&"bold".to_string()));
    }

    #[test]
    fn parses_multiple_marks() {
        let input = r#"
[alpha]
label = "alpha mark"

[beta]
label = "beta mark"
[beta.style]
color = "red"
"#;
        let defs = parse_marks_toml(input).unwrap();
        assert_eq!(defs.len(), 2);
        assert_eq!(defs.get("alpha").unwrap().label, "alpha mark");
        assert!(defs.get("alpha").unwrap().style.is_empty());
        assert_eq!(defs.get("beta").unwrap().style.get("color"), Some(&"red".to_string()));
    }

    #[test]
    fn invalid_toml_returns_none() {
        assert_eq!(parse_marks_toml("[unclosed"), None);
        assert_eq!(parse_marks_toml("not = = toml"), None);
    }

    #[test]
    fn missing_label_defaults_to_code() {
        let input = "[nolabel]\n[nolabel.style]\ncolor = \"blue\"\n";
        let defs = parse_marks_toml(input).unwrap();
        assert_eq!(defs.get("nolabel").unwrap().label, "nolabel");
    }

    #[test]
    fn non_lowercase_codes_skipped() {
        // The parser only matches ascii-lowercase tokens in the type slot,
        // so codes that could never match are rejected at load time
        let input = "[MyMark]\nlabel = \"x\"\n\n[ok]\nlabel = \"y\"\n\n[with_underscore]\nlabel = \"z\"\n";
        let defs = parse_marks_toml(input).unwrap();
        assert!(defs.get("MyMark").is_none());
        assert!(defs.get("with_underscore").is_none());
        assert!(defs.get("ok").is_some());
    }

    #[test]
    fn non_string_style_values_skipped() {
        let input = "[m]\nlabel = \"x\"\n[m.style]\ncolor = \"red\"\nweight = 700\n";
        let defs = parse_marks_toml(input).unwrap();
        let style = &defs.get("m").unwrap().style;
        assert_eq!(style.get("color"), Some(&"red".to_string()));
        assert!(style.get("weight").is_none());
    }

    #[test]
    fn empty_input_is_empty_map() {
        let defs = parse_marks_toml("").unwrap();
        assert!(defs.is_empty());
    }
}
