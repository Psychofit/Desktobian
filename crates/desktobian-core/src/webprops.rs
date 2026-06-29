//! Parsing a Wallpaper Engine web project's user-configurable properties.
//!
//! Web wallpapers declare their tweakable settings (colours, sliders, combos,
//! toggles, …) in the `general.properties` block of their `project.json`. This
//! turns that block into a flat, ordered, typed model the wallpaper manager GUI
//! can render an editor from — the canonical counterpart to the QML plugin's
//! `we-properties.js`.
//!
//! The *values* the user chooses are stored elsewhere as a sparse override map
//! (`{ name: value }`); this module only describes the available properties and
//! their defaults.

use serde::Serialize;
use serde_json::Value;

/// One editable web wallpaper property, ready to serialise to the UI.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WebProperty {
    /// Property key (the name the wallpaper's JS expects in `applyUserProperties`).
    pub name: String,
    /// Normalised control type: `bool` | `slider` | `combo` | `color` |
    /// `textinput` (or the raw type for anything else, which the UI renders as
    /// a text field).
    #[serde(rename = "type")]
    pub kind: String,
    /// Human label shown next to the control.
    pub label: String,
    /// Sort order from `project.json` (lower comes first).
    pub order: f64,
    /// The project default value.
    pub default: Value,
    /// Slider bounds / step (only for `slider`). `step` of `0` means continuous.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// Choices (only for `combo`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<WebPropertyOption>>,
}

/// One choice of a `combo` property.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WebPropertyOption {
    pub label: String,
    pub value: Value,
}

/// Parse the editable properties out of a `project.json` document.
///
/// Returns an empty list on any problem (not valid JSON, no `general.properties`,
/// etc.) so callers can treat "no editor" uniformly.
pub fn parse_properties(project_json: &str) -> Vec<WebProperty> {
    let root: Value = match serde_json::from_str(project_json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let props = match root.get("general").and_then(|g| g.get("properties")) {
        Some(Value::Object(map)) => map,
        _ => return Vec::new(),
    };

    let mut list: Vec<WebProperty> = Vec::new();
    for (name, def) in props {
        // Skip group headers / labels: entries without a value aren't editable.
        let Some(default) = def.get("value") else {
            continue;
        };

        let raw_type = def
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let label = def
            .get("text")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or(name)
            .to_string();
        let order = def.get("order").and_then(Value::as_f64).unwrap_or(1000.0);

        let mut prop = WebProperty {
            name: name.clone(),
            kind: raw_type.clone(),
            label,
            order,
            default: default.clone(),
            min: None,
            max: None,
            step: None,
            options: None,
        };

        match raw_type.as_str() {
            // WE's "int" is a whole-number slider.
            "int" => {
                prop.kind = "slider".to_string();
                prop.min = Some(def.get("min").and_then(Value::as_f64).unwrap_or(0.0));
                prop.max = Some(def.get("max").and_then(Value::as_f64).unwrap_or(100.0));
                prop.step = Some(
                    def.get("step")
                        .and_then(Value::as_f64)
                        .filter(|s| *s > 0.0)
                        .unwrap_or(1.0),
                );
            }
            "slider" => {
                prop.min = Some(def.get("min").and_then(Value::as_f64).unwrap_or(0.0));
                prop.max = Some(def.get("max").and_then(Value::as_f64).unwrap_or(1.0));
                // 0 (or absent) => continuous.
                prop.step = Some(
                    def.get("step")
                        .and_then(Value::as_f64)
                        .filter(|s| *s > 0.0)
                        .unwrap_or(0.0),
                );
            }
            "combo" => {
                let options = def
                    .get("options")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .map(|o| {
                                let value = o.get("value").cloned().unwrap_or(Value::Null);
                                let label = o
                                    .get("label")
                                    .and_then(Value::as_str)
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| value_to_label(&value));
                                WebPropertyOption { label, value }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                prop.options = Some(options);
            }
            _ => {}
        }

        list.push(prop);
    }

    list.sort_by(|a, b| {
        a.order
            .partial_cmp(&b.order)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });
    list
}

/// A reasonable display string for an option value that has no explicit label.
fn value_to_label(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PROJECT: &str = r#"{
        "type": "web",
        "general": {
            "properties": {
                "schemecolor": { "type": "color", "text": "Colour", "value": "0.2 0.4 0.8", "order": 2 },
                "speed": { "type": "slider", "text": "Speed", "value": 1.5, "min": 0, "max": 5, "step": 0.1, "order": 1 },
                "count": { "type": "int", "text": "Count", "value": 3, "min": 1, "max": 10, "order": 0 },
                "darkmode": { "type": "bool", "text": "Dark", "value": true, "order": 3 },
                "align": {
                    "type": "combo", "text": "Align", "value": "center", "order": 4,
                    "options": [
                        { "label": "Left", "value": "left" },
                        { "label": "Center", "value": "center" }
                    ]
                },
                "header": { "type": "text", "text": "A header", "order": 5 }
            }
        }
    }"#;

    #[test]
    fn parses_and_orders_properties() {
        let props = parse_properties(PROJECT);
        // The value-less "text" header is skipped.
        assert_eq!(props.len(), 5);
        let names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["count", "speed", "schemecolor", "darkmode", "align"]);
    }

    #[test]
    fn slider_bounds_parsed() {
        let props = parse_properties(PROJECT);
        let speed = props.iter().find(|p| p.name == "speed").unwrap();
        assert_eq!(speed.kind, "slider");
        assert_eq!(speed.min, Some(0.0));
        assert_eq!(speed.max, Some(5.0));
        assert_eq!(speed.step, Some(0.1));
    }

    #[test]
    fn int_becomes_stepped_slider() {
        let props = parse_properties(PROJECT);
        let count = props.iter().find(|p| p.name == "count").unwrap();
        assert_eq!(count.kind, "slider");
        assert_eq!(count.step, Some(1.0));
        assert_eq!(count.default, json!(3));
    }

    #[test]
    fn combo_options_parsed() {
        let props = parse_properties(PROJECT);
        let align = props.iter().find(|p| p.name == "align").unwrap();
        assert_eq!(align.kind, "combo");
        let opts = align.options.as_ref().unwrap();
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[1].label, "Center");
        assert_eq!(opts[1].value, json!("center"));
    }

    #[test]
    fn color_and_bool_defaults_kept() {
        let props = parse_properties(PROJECT);
        let color = props.iter().find(|p| p.name == "schemecolor").unwrap();
        assert_eq!(color.kind, "color");
        assert_eq!(color.default, json!("0.2 0.4 0.8"));
        let dark = props.iter().find(|p| p.name == "darkmode").unwrap();
        assert_eq!(dark.kind, "bool");
        assert_eq!(dark.default, json!(true));
    }

    #[test]
    fn label_falls_back_to_name() {
        let p = parse_properties(
            r#"{"general":{"properties":{"foo":{"type":"bool","value":false}}}}"#,
        );
        assert_eq!(p[0].label, "foo");
    }

    #[test]
    fn serialises_with_type_key_and_skips_none() {
        let props = parse_properties(PROJECT);
        let dark = props.iter().find(|p| p.name == "darkmode").unwrap();
        let v = serde_json::to_value(dark).unwrap();
        assert_eq!(v["type"], json!("bool"));
        // Non-slider/combo: optional fields omitted.
        assert!(v.get("min").is_none());
        assert!(v.get("options").is_none());
    }

    #[test]
    fn bad_or_empty_input_yields_empty() {
        assert!(parse_properties("not json").is_empty());
        assert!(parse_properties(r#"{"type":"web"}"#).is_empty());
        assert!(parse_properties(r#"{"general":{}}"#).is_empty());
    }
}
