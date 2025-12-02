use serde_json::{json, Value};

/// 診断メッセージのローカライズキーとロケール情報。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalizationKey {
    key: Option<String>,
    locale: Option<String>,
    args: Vec<String>,
}

impl LocalizationKey {
    pub fn new(key: Option<String>, locale: Option<String>, args: Vec<String>) -> Self {
        Self { key, locale, args }
    }

    /// JSON 診断 (`FrontendDiagnostic` の直列化結果) からローカライズ情報を抽出する。
    pub fn from_diagnostic(value: &Value) -> Self {
        let expected = value.get("expected");
        let key = extract_string(value.get("message_key"))
            .or_else(|| expected.and_then(|map| extract_string(map.get("message_key"))));
        let locale = extract_string(value.get("locale"));
        let args = extract_string_array(value.get("locale_args"))
            .or_else(|| expected.and_then(|map| extract_string_array(map.get("locale_args"))))
            .unwrap_or_default();
        Self { key, locale, args }
    }

    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    pub fn locale(&self) -> Option<&str> {
        self.locale.as_deref()
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn is_empty(&self) -> bool {
        self.key.is_none() && self.locale.is_none() && self.args.is_empty()
    }

    /// JSON へ埋め込みやすい形へ変換する。空であれば `Value::Null` を返す。
    pub fn to_value(&self) -> Value {
        if self.is_empty() {
            Value::Null
        } else {
            let mut map = serde_json::Map::new();
            if let Some(key) = &self.key {
                map.insert("message_key".to_string(), json!(key));
            }
            if let Some(locale) = &self.locale {
                map.insert("locale".to_string(), json!(locale));
            }
            if !self.args.is_empty() {
                map.insert("locale_args".to_string(), json!(self.args));
            }
            Value::Object(map)
        }
    }

    /// 人間可読な表示用ラベル。Human 出力でのデバッグ用途を想定する。
    pub fn display_label(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        if let Some(key) = &self.key {
            parts.push(format!("key={key}"));
        }
        if let Some(locale) = &self.locale {
            parts.push(format!("locale={locale}"));
        }
        if !self.args.is_empty() {
            parts.push(format!("args=[{}]", self.args.join(", ")));
        }
        Some(parts.join(", "))
    }
}

fn extract_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn extract_string_array(value: Option<&Value>) -> Option<Vec<String>> {
    let array = value?.as_array()?;
    let mut result = Vec::with_capacity(array.len());
    for entry in array {
        if let Some(text) = entry.as_str() {
            result.push(text.to_string());
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::LocalizationKey;
    use serde_json::json;

    #[test]
    fn extracts_top_level_message_key() {
        let value = json!({
            "message_key": "parse.expected",
            "locale": "ja-JP",
            "locale_args": ["fn"]
        });
        let key = LocalizationKey::from_diagnostic(&value);
        assert_eq!(key.key(), Some("parse.expected"));
        assert_eq!(key.locale(), Some("ja-JP"));
        assert_eq!(key.args(), &["fn".to_string()]);
        assert!(!key.is_empty());
    }

    #[test]
    fn falls_back_to_expected_summary() {
        let value = json!({
            "expected": {
                "message_key": "parse.expected",
                "locale_args": ["fn", "identifier"]
            }
        });
        let key = LocalizationKey::from_diagnostic(&value);
        assert_eq!(key.key(), Some("parse.expected"));
        assert!(key.locale().is_none());
        assert_eq!(key.args(), &["fn".to_string(), "identifier".to_string()]);
    }

    #[test]
    fn display_label_is_none_when_empty() {
        let value = json!({ "message": "example" });
        let key = LocalizationKey::from_diagnostic(&value);
        assert!(key.display_label().is_none());
    }
}
