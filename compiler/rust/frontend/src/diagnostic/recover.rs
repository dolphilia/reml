//! Recover 系診断で期待トークン列を整列・要約するユーティリティ。
//!
//! OCaml 版 `parser_expectation.ml` の `dedup_and_sort`／`humanize` をベースに、
//! `Keyword` → `Token` → `Class` → `Rule` → その他の優先順位で直列化する。

use std::cmp::Ordering;

use super::{EXPECTED_EMPTY_HUMANIZED, PARSE_EXPECTED_EMPTY_KEY, PARSE_EXPECTED_KEY};

/// Recover で提示する期待トークンの分類。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExpectedToken {
    Keyword(String),
    Token(String),
    Class(String),
    Rule(String),
    Eof,
    Not(String),
    Custom(String),
    TypeExpected(String),
    TraitBound(String),
}

impl ExpectedToken {
    pub fn keyword(value: impl Into<String>) -> Self {
        Self::Keyword(value.into())
    }

    pub fn token(value: impl Into<String>) -> Self {
        Self::Token(value.into())
    }

    pub fn class(value: impl Into<String>) -> Self {
        Self::Class(value.into())
    }

    pub fn rule(value: impl Into<String>) -> Self {
        Self::Rule(value.into())
    }

    pub fn custom(value: impl Into<String>) -> Self {
        Self::Custom(value.into())
    }

    pub fn eof() -> Self {
        Self::Eof
    }

    fn priority(&self) -> u8 {
        match self {
            ExpectedToken::Keyword(_) => 0,
            ExpectedToken::Token(_) | ExpectedToken::Eof => 1,
            ExpectedToken::Class(_) | ExpectedToken::TypeExpected(_) => 2,
            ExpectedToken::Rule(_) | ExpectedToken::TraitBound(_) => 3,
            ExpectedToken::Not(_) => 4,
            ExpectedToken::Custom(_) => 5,
        }
    }

    fn raw_label(&self) -> &str {
        match self {
            ExpectedToken::Keyword(value)
            | ExpectedToken::Token(value)
            | ExpectedToken::Class(value)
            | ExpectedToken::Rule(value)
            | ExpectedToken::Not(value)
            | ExpectedToken::Custom(value)
            | ExpectedToken::TypeExpected(value)
            | ExpectedToken::TraitBound(value) => value.as_str(),
            ExpectedToken::Eof => "EOF",
        }
    }

    fn quoted_label(&self) -> String {
        match self {
            ExpectedToken::Keyword(value) | ExpectedToken::Token(value) => {
                format!("`{value}`")
            }
            ExpectedToken::Eof => "入力終端".to_string(),
            ExpectedToken::Class(value) | ExpectedToken::Rule(value) => value.clone(),
            ExpectedToken::Not(value) => format!("{value}以外"),
            ExpectedToken::Custom(value) => value.clone(),
            ExpectedToken::TypeExpected(ty) => format!("型 {ty}"),
            ExpectedToken::TraitBound(trait_bound) => format!("{trait_bound} 境界"),
        }
    }

    fn cmp_for_sort(&self, other: &Self) -> Ordering {
        match self.priority().cmp(&other.priority()) {
            Ordering::Equal => self.raw_label().cmp(other.raw_label()),
            ordering => ordering,
        }
    }
}

/// Menhir 互換の期待トークン列を整列・重複排除するコレクタ。
#[derive(Debug, Default, Clone)]
pub struct ExpectedTokenCollector {
    entries: Vec<ExpectedToken>,
}

impl ExpectedTokenCollector {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, token: ExpectedToken) {
        self.entries.push(token);
    }

    pub fn push_keyword(&mut self, keyword: impl Into<String>) {
        self.push(ExpectedToken::keyword(keyword));
    }

    pub fn push_token(&mut self, token: impl Into<String>) {
        self.push(ExpectedToken::token(token));
    }

    pub fn push_class(&mut self, class_name: impl Into<String>) {
        self.push(ExpectedToken::class(class_name));
    }

    pub fn push_rule(&mut self, rule_name: impl Into<String>) {
        self.push(ExpectedToken::rule(rule_name));
    }

    pub fn push_custom(&mut self, text: impl Into<String>) {
        self.push(ExpectedToken::custom(text));
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = ExpectedToken>,
    {
        self.entries.extend(iter);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn summarize(&self) -> ExpectedTokensSummary {
        self.summarize_with_context(None)
    }

    pub fn summarize_with_context(&self, context_note: Option<String>) -> ExpectedTokensSummary {
        let normalized = self.normalized();
        let locale_args = normalized
            .iter()
            .map(|token| token.raw_label().to_string())
            .collect::<Vec<_>>();
        let (message_key, humanized) = if normalized.is_empty() {
            (
                Some(PARSE_EXPECTED_EMPTY_KEY.to_string()),
                Some(EXPECTED_EMPTY_HUMANIZED.to_string()),
            )
        } else {
            (Some(PARSE_EXPECTED_KEY.to_string()), humanize(&normalized))
        };

        ExpectedTokensSummary {
            message_key,
            locale_args,
            humanized,
            context_note,
            alternatives: normalized,
        }
    }

    fn normalized(&self) -> Vec<ExpectedToken> {
        let mut normalized = self.entries.clone();
        normalized.sort_by(|a, b| a.cmp_for_sort(b));
        normalized.dedup();
        normalized
    }
}

/// `Diagnostic.expectation` へ格納する直列化結果。
#[derive(Debug, Clone)]
pub struct ExpectedTokensSummary {
    pub message_key: Option<String>,
    pub locale_args: Vec<String>,
    pub humanized: Option<String>,
    pub context_note: Option<String>,
    pub alternatives: Vec<ExpectedToken>,
}

impl ExpectedTokensSummary {
    pub fn tokens(&self) -> Vec<String> {
        self.alternatives
            .iter()
            .map(|token| token.raw_label().to_string())
            .collect()
    }

    pub fn has_alternatives(&self) -> bool {
        !self.alternatives.is_empty()
    }
}

fn humanize(expectations: &[ExpectedToken]) -> Option<String> {
    match expectations {
        [] => None,
        [single] => Some(format!("ここで{}が必要です", single.quoted_label())),
        _ => {
            let mut labels: Vec<String> = expectations
                .iter()
                .map(ExpectedToken::quoted_label)
                .collect();
            if let Some(last) = labels.pop() {
                if labels.is_empty() {
                    Some(format!("ここで{last}のいずれかが必要です"))
                } else {
                    let body = format!("{} または {last}", labels.join("、"));
                    Some(format!("ここで{body}のいずれかが必要です"))
                }
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExpectedToken, ExpectedTokenCollector};

    #[test]
    fn dedup_and_sort_respects_priority() {
        let mut collector = ExpectedTokenCollector::new();
        collector.push_rule("expression");
        collector.push_keyword("let");
        collector.push_token(";");
        collector.push_class("identifier");
        collector.push_keyword("fn");
        collector.push_keyword("fn");

        let summary = collector.summarize();
        let tokens = summary.tokens();

        assert_eq!(
            tokens,
            vec![
                "fn".to_string(),
                "let".to_string(),
                ";".to_string(),
                "identifier".to_string(),
                "expression".to_string()
            ]
        );
    }

    #[test]
    fn humanize_handles_single_and_multiple() {
        let mut single = ExpectedTokenCollector::new();
        single.push(ExpectedToken::keyword("fn"));
        let summary_single = single.summarize();
        assert_eq!(
            summary_single.humanized,
            Some("ここで`fn`が必要です".to_string())
        );

        let mut multiple = ExpectedTokenCollector::new();
        multiple.push(ExpectedToken::keyword("fn"));
        multiple.push(ExpectedToken::class("identifier"));
        multiple.push(ExpectedToken::rule("expression"));
        let summary_multiple = multiple.summarize();
        assert_eq!(
            summary_multiple.humanized,
            Some("ここで`fn`、identifier または expressionのいずれかが必要です".to_string())
        );
    }

    #[test]
    fn empty_summary_uses_placeholder_messages() {
        let collector = ExpectedTokenCollector::new();
        let summary = collector.summarize();
        assert_eq!(summary.tokens(), Vec::<String>::new());
        assert_eq!(
            summary.message_key.as_deref(),
            Some(super::PARSE_EXPECTED_EMPTY_KEY)
        );
        assert_eq!(
            summary.humanized.as_deref(),
            Some(super::EXPECTED_EMPTY_HUMANIZED)
        );
    }
}
