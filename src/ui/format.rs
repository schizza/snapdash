use crate::ha::EntityState;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct FormattedValue {
    pub title: Option<String>,  // friendly name
    pub main: String,           // big number
    pub detail: Option<String>, // small line main
}

fn attr_str(st: &EntityState, key: &str) -> Option<String> {
    st.attributes.get(key).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    })
}

fn attr_f64(st: &EntityState, key: &str) -> Option<f64> {
    st.attributes.get(key).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    })
}

fn parse_f64(s: &str) -> Option<f64> {
    s.parse::<f64>().ok()
}

fn domain(entity_id: &str) -> &str {
    entity_id.split('.').next().unwrap_or("")
}

fn on_off(state: &str) -> Option<&'static str> {
    match state {
        "on" => Some("On"),
        "off" => Some("Off"),
        _ => None,
    }
}

fn human_number(n: f64) -> String {
    if (n.fract()).abs() < 0.0001 {
        format!("{:.0}", n)
    } else if (n * 10.0).fract().abs() < 0.0001 {
        format!("{:.1}", n)
    } else {
        format!("{:.2}", n)
    }
}

pub fn format_entity_value(st: &EntityState) -> FormattedValue {
    let title = attr_str(st, "friendly_name");
    let dom = domain(&st.entity_id);
    let raw = st.state.as_str();

    if let Some(v) = on_off(raw) {
        let mut detail_bits: Vec<String> = Vec::new();

        if dom == "light" {
            if let Some(b) = attr_f64(st, "brightness") {
                let pct = (b / 255.0 * 100.0).round();
                detail_bits.push(format!("Brightness {}%", pct as i64));
            }
            if let Some(ct) = attr_f64(st, "color_temp_kelvin") {
                detail_bits.push(format!("{}K", ct.round() as i64))
            } else if let Some(ct) = attr_f64(st, "color_temp") {
                let kelvin = (1_000_000.0 / ct).round();
                detail_bits.push(format!("{}K", kelvin as i64));
            }
        }

        if dom == "fan" {
            if let Some(p) = attr_str(st, "percentage") {
                detail_bits.push(format!("{}%", p));
            }
            if let Some(p) = attr_str(st, "preset_mode") {
                detail_bits.push(p);
            }
        }

        return FormattedValue {
            title,
            main: v.into(),
            detail: if detail_bits.is_empty() {
                None
            } else {
                Some(detail_bits.join(" • "))
            },
        };
    }

    if dom == "sensor" || dom == "number" || dom == "input_number" {
        let unit = attr_str(st, "unit_of_measurement");
        if let Some(n) = parse_f64(raw) {
            let main = match unit.as_deref() {
                Some("°C") | Some("°F") | Some("%") | Some("W") | Some("kW") | Some("V")
                | Some("A") => {
                    format!("{} {}", human_number(n), unit.unwrap())
                }
                Some(u) => format!("{} {}", human_number(n), u),
                None => human_number(n),
            };

            // detail: state_class / device_class (když je)
            let device_class = attr_str(st, "device_class");
            let state_class = attr_str(st, "state_class");
            let mut detail = Vec::new();
            if let Some(dc) = device_class {
                detail.push(dc);
            }
            if let Some(sc) = state_class {
                detail.push(sc);
            }

            return FormattedValue {
                title,
                main,
                detail: if detail.is_empty() {
                    None
                } else {
                    Some(detail.join(" • "))
                },
            };
        }
    }

    if dom == "climate" {
        let hvac = attr_str(st, "hvac_action").or_else(|| attr_str(st, "hvac_mode"));
        let temp = attr_f64(st, "current_temperature").or_else(|| attr_f64(st, "temperature"));

        let main = match (temp, hvac.as_deref()) {
            (Some(t), Some(h)) => format!("{}° • {}", human_number(t), h),
            (Some(t), None) => format!("{}°", human_number(t)),
            (None, Some(h)) => h.to_string(),
            _ => raw.to_string(),
        };

        let mut detail_bits = Vec::new();
        if let Some(h) = attr_f64(st, "humidity") {
            detail_bits.push(format!("{}%", h.round() as i64));
        }

        return FormattedValue {
            title,
            main,
            detail: if detail_bits.is_empty() {
                None
            } else {
                Some(detail_bits.join(" • "))
            },
        };
    }

    FormattedValue {
        title,
        main: raw.to_string(),
        detail: None,
    }
}
