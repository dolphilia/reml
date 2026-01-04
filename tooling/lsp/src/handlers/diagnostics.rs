//! LSP 診断ハンドラで `LocalizationKey` を扱うための補助ユーティリティ（Rust PoC）。
//!
//! Rust フロントエンドと同一のローカライズ情報を扱うための
//! サンプル実装として本モジュールを配置している。`serde_json::Value` を直接操作し、
//! `data.localization` ブロックへ `{message_key, locale, locale_args}` を注入する。

use serde_json::{json, Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationKey {
    key: Option<String>,
    locale: Option<String>,
    args: Vec<String>,
}

impl LocalizationKey {
    pub fn from_diagnostic(diag: &Value) -> Self {
        let expected = diag.get("expected");
        let key = diag
            .get("message_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                expected
                    .and_then(|exp| exp.get("message_key"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });
        let locale = diag
            .get("locale")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let args = diag
            .get("locale_args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .or_else(|| {
                expected.and_then(|exp| {
                    exp.get("locale_args").and_then(|v| {
                        v.as_array()
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
                    })
                })
            })
            .unwrap_or_default();
        Self { key, locale, args }
    }

    pub fn is_empty(&self) -> bool {
        self.key.is_none() && self.locale.is_none() && self.args.is_empty()
    }

    pub fn to_value(&self) -> Value {
        if self.is_empty() {
            Value::Null
        } else {
            let mut map = Map::new();
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
}

pub fn inject_localization(data: &mut Map<String, Value>, diag: &Value) {
    let localization = LocalizationKey::from_diagnostic(diag);
    if !localization.is_empty() {
        data.insert("localization".to_string(), localization.to_value());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_localization_payload() {
        let diag = json!({
            "message_key": "parse.expected",
            "locale": "en-US",
            "locale_args": ["identifier"]
        });
        let mut data = Map::new();
        inject_localization(&mut data, &diag);
        assert_eq!(
            data.get("localization").unwrap()["message_key"],
            json!("parse.expected")
        );
    }
}
