use crate::run_config::RunConfigExtensionValue;
use indexmap::IndexMap;
use serde_json::{json, Value};
use std::fmt;

/// DSL で使用する fixity シンボル。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixitySymbol {
    Prefix,
    Postfix,
    InfixLeft,
    InfixRight,
    InfixNonassoc,
    Ternary,
}

impl FixitySymbol {
    /// `:infix_left` など DSL に登場するシンボル表記。
    pub fn keyword(self) -> &'static str {
        match self {
            FixitySymbol::Prefix => ":prefix",
            FixitySymbol::Postfix => ":postfix",
            FixitySymbol::InfixLeft => ":infix_left",
            FixitySymbol::InfixRight => ":infix_right",
            FixitySymbol::InfixNonassoc => ":infix_nonassoc",
            FixitySymbol::Ternary => ":ternary",
        }
    }

    fn label(self) -> &'static str {
        match self {
            FixitySymbol::Prefix => "prefix",
            FixitySymbol::Postfix => "postfix",
            FixitySymbol::InfixLeft => "infix_left",
            FixitySymbol::InfixRight => "infix_right",
            FixitySymbol::InfixNonassoc => "infix_nonassoc",
            FixitySymbol::Ternary => "ternary",
        }
    }
}

/// OpBuilder DSL で `builder.level(...)` を積み上げるためのビルダー。
#[derive(Debug, Clone, Default)]
pub struct OpBuilder {
    levels: IndexMap<i64, LevelDefinition>,
}

impl OpBuilder {
    /// 新しいビルダーを作成する。
    pub fn new() -> Self {
        Self::default()
    }

    /// `builder.level(priority, :fixity, ["+"])` 形式の宣言を登録する。
    pub fn level<T, S>(
        &mut self,
        priority: i64,
        fixity: FixitySymbol,
        tokens: T,
    ) -> Result<&mut Self, OpBuilderError>
    where
        T: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let values = tokens.into_iter().map(Into::into).collect::<Vec<String>>();
        match self.levels.entry(priority) {
            indexmap::map::Entry::Occupied(mut entry) => {
                let existing = entry.get_mut();
                if existing.fixity != fixity {
                    return Err(OpBuilderErrorKind::LevelConflict {
                        priority,
                        existing: existing.fixity,
                        incoming: fixity,
                    }
                    .into());
                }
                existing.add_tokens(fixity, values)?;
            }
            indexmap::map::Entry::Vacant(entry) => {
                entry.insert(LevelDefinition::new(fixity, values)?);
            }
        }
        Ok(self)
    }

    /// 優先度レベルの集合を確定し、実行時に利用するテーブルへ変換する。
    pub fn build(self) -> Result<OpTable, OpBuilderError> {
        let mut entries = self
            .levels
            .into_iter()
            .enumerate()
            .map(|(order, (priority, level))| LevelWithOrder {
                priority,
                order,
                level,
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.order.cmp(&b.order)));
        let levels = entries
            .into_iter()
            .map(|entry| entry.level.into_level(entry.priority))
            .collect();
        Ok(OpTable { levels })
    }
}

struct LevelWithOrder {
    priority: i64,
    order: usize,
    level: LevelDefinition,
}

#[derive(Debug, Clone)]
struct LevelDefinition {
    fixity: FixitySymbol,
    operators: OperatorCollection,
}

impl LevelDefinition {
    fn new(fixity: FixitySymbol, tokens: Vec<String>) -> Result<Self, OpBuilderError> {
        Ok(Self {
            fixity,
            operators: OperatorCollection::new(fixity, tokens)?,
        })
    }

    fn add_tokens(
        &mut self,
        fixity: FixitySymbol,
        tokens: Vec<String>,
    ) -> Result<(), OpBuilderError> {
        self.operators.extend(fixity, tokens)
    }

    fn into_level(self, priority: i64) -> OpLevel {
        OpLevel {
            priority,
            fixity: self.fixity,
            operators: self.operators.into_specs(),
        }
    }
}

#[derive(Debug, Clone)]
enum OperatorCollection {
    /// prefix/postfix/infix のトークン一覧。
    Operators(Vec<String>),
    /// ternary 用の head/mid ペア。
    Ternary(Vec<TernaryToken>),
}

impl OperatorCollection {
    fn new(fixity: FixitySymbol, tokens: Vec<String>) -> Result<Self, OpBuilderError> {
        match fixity {
            FixitySymbol::Ternary => Ok(Self::Ternary(vec![TernaryToken::try_from(tokens)?])),
            _ => {
                if tokens.is_empty() {
                    Err(OpBuilderErrorKind::EmptyTokenList { fixity }.into())
                } else {
                    Ok(Self::Operators(tokens))
                }
            }
        }
    }

    fn extend(&mut self, fixity: FixitySymbol, tokens: Vec<String>) -> Result<(), OpBuilderError> {
        match (fixity, self) {
            (FixitySymbol::Ternary, OperatorCollection::Ternary(existing)) => {
                existing.push(TernaryToken::try_from(tokens)?);
            }
            (FixitySymbol::Ternary, OperatorCollection::Operators(_)) => {
                unreachable!("OpBuilder level fixity mismatch should be caught earlier");
            }
            (_, OperatorCollection::Operators(existing)) => {
                if tokens.is_empty() {
                    return Err(OpBuilderErrorKind::EmptyTokenList { fixity }.into());
                }
                existing.extend(tokens);
            }
            (_, OperatorCollection::Ternary(_)) => {
                unreachable!("OpBuilder level fixity mismatch should be caught earlier");
            }
        }
        Ok(())
    }

    fn into_specs(self) -> Vec<OperatorSpec> {
        match self {
            OperatorCollection::Operators(tokens) => {
                tokens.into_iter().map(OperatorSpec::token).collect()
            }
            OperatorCollection::Ternary(tokens) => tokens
                .into_iter()
                .map(|token| OperatorSpec::Ternary {
                    head: token.head,
                    mid: token.mid,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
struct TernaryToken {
    head: String,
    mid: String,
}

impl TryFrom<Vec<String>> for TernaryToken {
    type Error = OpBuilderError;

    fn try_from(tokens: Vec<String>) -> Result<Self, Self::Error> {
        if tokens.len() != 2 {
            return Err(OpBuilderErrorKind::InvalidTernaryTokenCount {
                actual: tokens.len(),
            }
            .into());
        }
        let mut iter = tokens.into_iter();
        let head = iter.next().unwrap();
        let mid = iter.next().unwrap();
        Ok(Self { head, mid })
    }
}

/// `builder.level` から得られる優先度レベル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpLevel {
    pub priority: i64,
    pub fixity: FixitySymbol,
    pub operators: Vec<OperatorSpec>,
}

/// `OpBuilder.build()` の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpTable {
    levels: Vec<OpLevel>,
}

impl OpTable {
    /// 優先度レベルの一覧を取得する（高い優先度から降順）。
    pub fn levels(&self) -> &[OpLevel] {
        &self.levels
    }

    /// RunConfig.extensions["parse"] へ埋め込むための JSON 互換値を生成する。
    pub fn to_run_config_extension(&self) -> RunConfigExtensionValue {
        let levels = self
            .levels
            .iter()
            .map(|lvl| {
                let mut obj = serde_json::Map::new();
                obj.insert("priority".into(), json!(lvl.priority));
                obj.insert(
                    "fixity".into(),
                    json!(match lvl.fixity {
                        FixitySymbol::Prefix => "prefix",
                        FixitySymbol::Postfix => "postfix",
                        FixitySymbol::InfixLeft => "infix_left",
                        FixitySymbol::InfixRight => "infix_right",
                        FixitySymbol::InfixNonassoc => "infix_nonassoc",
                        FixitySymbol::Ternary => "ternary",
                    }),
                );
                let operators = lvl
                    .operators
                    .iter()
                    .map(|op| match op {
                        OperatorSpec::Token(t) => Value::String(t.clone()),
                        OperatorSpec::Ternary { head, mid } => Value::Array(vec![
                            Value::String(head.clone()),
                            Value::String(mid.clone()),
                        ]),
                    })
                    .collect();
                obj.insert("operators".into(), Value::Array(operators));
                Value::Object(obj)
            })
            .collect();
        let mut ext = RunConfigExtensionValue::new();
        ext.insert("operator_table".into(), Value::Array(levels));
        ext
    }
}

/// 演算子トークンの種類。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorSpec {
    Token(String),
    Ternary { head: String, mid: String },
}

impl OperatorSpec {
    fn token(value: String) -> Self {
        Self::Token(value)
    }
}

/// OpBuilder DSL に起因するエラー。
#[derive(Debug, Clone)]
pub struct OpBuilderError {
    kind: OpBuilderErrorKind,
}

impl OpBuilderError {
    pub fn kind(&self) -> &OpBuilderErrorKind {
        &self.kind
    }
}

impl fmt::Display for OpBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for OpBuilderError {}

impl From<OpBuilderErrorKind> for OpBuilderError {
    fn from(kind: OpBuilderErrorKind) -> Self {
        Self { kind }
    }
}

/// エラー種別。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpBuilderErrorKind {
    LevelConflict {
        priority: i64,
        existing: FixitySymbol,
        incoming: FixitySymbol,
    },
    EmptyTokenList {
        fixity: FixitySymbol,
    },
    InvalidTernaryTokenCount {
        actual: usize,
    },
}

impl fmt::Display for OpBuilderErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpBuilderErrorKind::LevelConflict {
                priority,
                existing,
                incoming,
            } => write!(
                f,
                "priority {} already uses `{}` fixity and cannot be redeclared as `{}`",
                priority,
                existing.label(),
                incoming.label()
            ),
            OpBuilderErrorKind::EmptyTokenList { fixity } => write!(
                f,
                "`{}` fixity requires at least one operator token",
                fixity.keyword()
            ),
            OpBuilderErrorKind::InvalidTernaryTokenCount { actual } => write!(
                f,
                "`:ternary` fixity requires exactly two tokens (head/mid), got {}",
                actual
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_tokens_with_same_fixity() {
        let mut builder = OpBuilder::new();
        builder
            .level(70, FixitySymbol::InfixLeft, ["+", "-"])
            .unwrap();
        builder.level(70, FixitySymbol::InfixLeft, ["*"]).unwrap();
        let table = builder.build().unwrap();
        assert_eq!(table.levels().len(), 1);
        let level = &table.levels()[0];
        assert_eq!(level.priority, 70);
        assert_eq!(level.fixity, FixitySymbol::InfixLeft);
        assert_eq!(
            level.operators,
            vec![
                OperatorSpec::Token("+".into()),
                OperatorSpec::Token("-".into()),
                OperatorSpec::Token("*".into())
            ]
        );
    }

    #[test]
    fn detects_conflicting_fixity() {
        let mut builder = OpBuilder::new();
        builder.level(10, FixitySymbol::Prefix, ["-"]).unwrap();
        let err = builder.level(10, FixitySymbol::Postfix, ["!"]).unwrap_err();
        assert!(matches!(
            err.kind(),
            OpBuilderErrorKind::LevelConflict { priority: 10, .. }
        ));
    }

    #[test]
    fn enforces_ternary_pair() {
        let mut builder = OpBuilder::new();
        let err = builder.level(5, FixitySymbol::Ternary, ["?"]).unwrap_err();
        assert!(matches!(
            err.kind(),
            OpBuilderErrorKind::InvalidTernaryTokenCount { actual: 1 }
        ));
    }

    #[test]
    fn build_orders_by_priority_desc() {
        let mut builder = OpBuilder::new();
        builder
            .level(40, FixitySymbol::InfixLeft, ["+", "-"])
            .unwrap()
            .level(80, FixitySymbol::Prefix, ["-"])
            .unwrap()
            .level(60, FixitySymbol::InfixLeft, ["*", "/"])
            .unwrap();
        let table = builder.build().unwrap();
        let priorities = table
            .levels()
            .iter()
            .map(|level| level.priority)
            .collect::<Vec<_>>();
        assert_eq!(priorities, vec![80, 60, 40]);
    }
}
