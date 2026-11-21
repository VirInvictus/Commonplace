//! The hand-rolled JSON envelope writer (spec §6): no serde, one
//! dependency-free writer for the fixed verb shapes. Strings escape
//! `\\`, `"`, and the control range below 0x20 (`\n`, `\t`, and
//! `\u00XX` for the rest); UTF-8 passes through.

#[derive(Debug)]
pub enum Json {
    Str(String),
    Int(i64),
    Bool(bool),
    Arr(Vec<Json>),
    /// Object fields in emission order (the shapes are pinned).
    Obj(Vec<(&'static str, Json)>),
}

impl Json {
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Str(s) => write_string(s, out),
            Json::Int(n) => out.push_str(&n.to_string()),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                out.push('{');
                for (i, (key, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// The fixed envelope head every verb shares: `verb`, `ok: true`, and
/// the verb's payload fields.
pub fn envelope(verb: &str, payload: Vec<(&'static str, Json)>) -> Json {
    let mut fields = vec![
        ("verb", Json::Str(verb.to_owned())),
        ("ok", Json::Bool(true)),
    ];
    fields.extend(payload);
    Json::Obj(fields)
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_shapes() {
        assert_eq!(Json::Int(3).render(), "3");
        assert_eq!(Json::Bool(true).render(), "true");
        assert_eq!(Json::Str("a\"b\\c".to_owned()).render(), "\"a\\\"b\\\\c\"");
        assert_eq!(
            Json::Arr(vec![Json::Int(1), Json::Int(2)]).render(),
            "[1,2]"
        );
        assert_eq!(
            Json::Obj(vec![("a", Json::Int(1)), ("b", Json::Str("x".to_owned()))]).render(),
            "{\"a\":1,\"b\":\"x\"}"
        );
    }

    #[test]
    fn escapes_control_characters() {
        assert_eq!(Json::Str("a\u{1}b".to_owned()).render(), "\"a\\u0001b\"");
        assert_eq!(Json::Str("tab\there".to_owned()).render(), "\"tab\\there\"");
    }

    #[test]
    fn utf8_passes_through() {
        assert_eq!(Json::Str("café".to_owned()).render(), "\"café\"");
    }
}
