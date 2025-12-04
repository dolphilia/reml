//! MigrationPlan / MigrationStep の実験実装。
//! `docs/spec/3-7-core-config-data.md` §5 で定義された
//! API を段階的に Rust へ導入する。Phase 3 では
//! `experimental-migration` フィーチャ有効時のみ公開する。

use crate::data::schema::{Field, SchemaDataType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, time::Duration};

/// `effect {migration}` に対応するタグ名。
pub const MIGRATION_EFFECT_TAG: &str = "migration";

/// スキーマ変更の計画書。仕様では `MigrationPlan` として
/// 定義されており、影響度や停止時間の有無をまとめて保持する。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MigrationPlan {
    #[serde(default)]
    pub steps: Vec<MigrationStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_duration: Option<MigrationDuration>,
    #[serde(default)]
    pub requires_downtime: bool,
    #[serde(default)]
    pub data_loss_risk: MigrationRiskLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl MigrationPlan {
    /// 空の計画を作成する。
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            estimated_duration: None,
            requires_downtime: false,
            data_loss_risk: MigrationRiskLevel::None,
            notes: None,
        }
    }

    /// 計画へステップを追加する。
    pub fn push_step(&mut self, step: MigrationStep) {
        if step.breaking() {
            self.data_loss_risk = self.data_loss_risk.max(MigrationRiskLevel::High);
        }
        self.steps.push(step);
    }

    /// ステップを追加したバリアントを返す。
    pub fn with_step(mut self, step: MigrationStep) -> Self {
        self.push_step(step);
        self
    }

    /// 差分が破壊的であるかを返す。
    pub fn has_breaking_changes(&self) -> bool {
        self.steps.iter().any(MigrationStep::breaking)
    }

    /// 計画が空かどうか。
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

impl Default for MigrationPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// 推定所要時間を秒単位で保持する補助型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationDuration {
    pub seconds: u64,
}

impl MigrationDuration {
    pub fn from_seconds(seconds: u64) -> Self {
        Self { seconds }
    }

    pub fn as_std(&self) -> Duration {
        Duration::from_secs(self.seconds)
    }
}

impl From<Duration> for MigrationDuration {
    fn from(value: Duration) -> Self {
        Self {
            seconds: value.as_secs(),
        }
    }
}

impl From<MigrationDuration> for Duration {
    fn from(value: MigrationDuration) -> Self {
        Duration::from_secs(value.seconds)
    }
}

/// MigrationPlan の各ステップ。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MigrationStep {
    AddField {
        name: String,
        field: Field,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default_value: Option<Value>,
        #[serde(default)]
        breaking: bool,
    },
    RemoveField {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        backup_location: Option<PathBuf>,
        #[serde(default)]
        breaking: bool,
    },
    RenameField {
        old_name: String,
        new_name: String,
        #[serde(default)]
        breaking: bool,
    },
    ChangeType {
        name: String,
        old_type: SchemaDataType,
        new_type: SchemaDataType,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        converter: Option<TypeConversionPlan>,
        #[serde(default)]
        breaking: bool,
    },
    ReorganizeData {
        strategy: ReorganizationStrategy,
        #[serde(default)]
        breaking: bool,
    },
}

impl MigrationStep {
    pub fn breaking(&self) -> bool {
        match self {
            MigrationStep::AddField { breaking, .. }
            | MigrationStep::RemoveField { breaking, .. }
            | MigrationStep::RenameField { breaking, .. }
            | MigrationStep::ChangeType { breaking, .. }
            | MigrationStep::ReorganizeData { breaking, .. } => *breaking,
        }
    }
}

/// 型変換の補助情報。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeConversionPlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub converter_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub lossy: bool,
}

impl TypeConversionPlan {
    pub fn lossy(converter_name: impl Into<String>) -> Self {
        Self {
            converter_name: Some(converter_name.into()),
            description: None,
            lossy: true,
        }
    }
}

/// データ再編戦略のメタ情報。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReorganizationStrategy {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_entities: Vec<String>,
}

impl ReorganizationStrategy {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            affected_entities: Vec::new(),
        }
    }
}

/// 仕様に記載されたリスクレベル。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MigrationRiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl Default for MigrationRiskLevel {
    fn default() -> Self {
        MigrationRiskLevel::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_field() -> Field {
        Field::builder("new_column", SchemaDataType::String)
            .required(true)
            .description("新しい文字列列")
            .finish()
    }

    #[test]
    fn plan_serialization_roundtrip() {
        let mut plan = MigrationPlan::new();
        plan.estimated_duration = Some(MigrationDuration::from_seconds(3600));
        plan.requires_downtime = true;
        plan.data_loss_risk = MigrationRiskLevel::Medium;
        plan.push_step(MigrationStep::AddField {
            name: "new_column".into(),
            field: sample_field(),
            default_value: Some(json!("fallback")),
            breaking: false,
        });
        plan.push_step(MigrationStep::ChangeType {
            name: "score".into(),
            old_type: SchemaDataType::Integer,
            new_type: SchemaDataType::Number,
            converter: Some(TypeConversionPlan {
                converter_name: Some("score_to_float".into()),
                description: Some("整数→浮動小数へ安全に変換".into()),
                lossy: false,
            }),
            breaking: true,
        });

        let serialized = serde_json::to_string_pretty(&plan).expect("serialize plan");
        let roundtrip: MigrationPlan = serde_json::from_str(&serialized).expect("deserialize plan");
        assert!(roundtrip.has_breaking_changes());
        assert_eq!(roundtrip.steps.len(), 2);
        assert!(roundtrip.requires_downtime);
        assert_eq!(roundtrip.estimated_duration.unwrap().seconds, 3600);
        match &roundtrip.steps[1] {
            MigrationStep::ChangeType { converter, .. } => {
                assert_eq!(
                    converter.as_ref().and_then(|c| c.converter_name.as_deref()),
                    Some("score_to_float")
                );
            }
            other => panic!("expected change_type step, got {other:?}"),
        }
    }
}
