use serde::Serialize;
use smol_str::SmolStr;

use super::super::{
    env::{StageId, StageRequirement},
    types::Type,
};

/// `Iterator` 制約を解決した結果を表す辞書情報。
#[derive(Debug, Clone, Serialize)]
pub struct IteratorDictInfo {
    pub source_type: Type,
    pub element_type: Type,
    pub kind: IteratorKind,
    pub stage_profile: IteratorStageProfile,
}

impl IteratorDictInfo {
    pub fn new(source_type: Type, element_type: Type, kind: IteratorKind) -> Self {
        let stage_profile = IteratorStageProfile::for_kind(kind.clone());
        Self {
            source_type,
            element_type,
            kind,
            stage_profile,
        }
    }

    /// 診断／監査に書き出すためのスナップショットを生成する。
    pub fn stage_snapshot(&self) -> IteratorStageSnapshot {
        self.stage_profile.snapshot(self.source_type.label())
    }
}

/// イテレータの種類。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum IteratorKind {
    ArrayLike,
    CoreIter,
    OptionLike,
    ResultLike,
    Custom(SmolStr),
}

impl IteratorKind {
    fn capability_label(&self) -> Option<&'static str> {
        match self {
            IteratorKind::ArrayLike => Some("core.iter.array"),
            IteratorKind::CoreIter => Some("core.iter.core"),
            IteratorKind::OptionLike => Some("core.iter.option"),
            IteratorKind::ResultLike => Some("core.iter.result"),
            IteratorKind::Custom(_) => None,
        }
    }

    fn default_requirement(&self) -> StageRequirement {
        match self {
            IteratorKind::ArrayLike => StageRequirement::Exact(StageId::stable()),
            _ => StageRequirement::AtLeast(StageId::beta()),
        }
    }

    fn default_actual(&self) -> StageId {
        match self {
            IteratorKind::ArrayLike => StageId::stable(),
            IteratorKind::CoreIter | IteratorKind::OptionLike | IteratorKind::ResultLike => {
                StageId::beta()
            }
            IteratorKind::Custom(label) => StageId::new(label.clone()),
        }
    }

    fn label(&self) -> SmolStr {
        match self {
            IteratorKind::ArrayLike => SmolStr::new("array_like"),
            IteratorKind::CoreIter => SmolStr::new("core_iter"),
            IteratorKind::OptionLike => SmolStr::new("option_like"),
            IteratorKind::ResultLike => SmolStr::new("result_like"),
            IteratorKind::Custom(label) => SmolStr::new(format!("custom:{label}")),
        }
    }
}

/// Stage/Capability 情報。
#[derive(Debug, Clone, Serialize)]
pub struct IteratorStageProfile {
    pub required: StageRequirement,
    pub actual: StageId,
    pub capability: Option<SmolStr>,
    pub kind: IteratorKind,
}

impl IteratorStageProfile {
    pub fn for_kind(kind: IteratorKind) -> Self {
        let required = kind.default_requirement();
        let actual = kind.default_actual();
        let capability = kind.capability_label().map(SmolStr::new);
        Self {
            required,
            actual,
            capability,
            kind,
        }
    }

    pub fn snapshot(&self, source: String) -> IteratorStageSnapshot {
        IteratorStageSnapshot {
            required: self.required.clone(),
            actual: self.actual.clone(),
            capability: self.capability.clone(),
            kind: self.kind.label(),
            source,
        }
    }
}

/// `effect.stage.iterator.*` に書き出す内容。
#[derive(Debug, Clone, Serialize)]
pub struct IteratorStageSnapshot {
    pub required: StageRequirement,
    pub actual: StageId,
    pub capability: Option<SmolStr>,
    pub kind: SmolStr,
    pub source: String,
}

/// `Iterator` 制約を解決する。
pub fn solve_iterator(source_ty: &Type) -> Option<IteratorDictInfo> {
    match source_ty {
        Type::Slice { element } => Some(new_dict(
            source_ty,
            element.as_ref(),
            IteratorKind::ArrayLike,
        )),
        Type::App {
            constructor,
            arguments,
        } if constructor == "Array" && arguments.len() == 1 => {
            Some(new_dict(source_ty, &arguments[0], IteratorKind::ArrayLike))
        }
        Type::App {
            constructor,
            arguments,
        } if constructor == "Slice" && arguments.len() == 1 => {
            Some(new_dict(source_ty, &arguments[0], IteratorKind::ArrayLike))
        }
        Type::App {
            constructor,
            arguments,
        } if constructor == "Iter" && arguments.len() == 1 => {
            Some(new_dict(source_ty, &arguments[0], IteratorKind::CoreIter))
        }
        Type::App {
            constructor,
            arguments,
        } if constructor == "IteratorState" && arguments.len() == 1 => {
            Some(new_dict(source_ty, &arguments[0], IteratorKind::CoreIter))
        }
        Type::App {
            constructor,
            arguments,
        } if constructor == "Option" && arguments.len() == 1 => {
            Some(new_dict(source_ty, &arguments[0], IteratorKind::OptionLike))
        }
        Type::App {
            constructor,
            arguments,
        } if constructor == "Result" && arguments.len() >= 1 => {
            Some(new_dict(source_ty, &arguments[0], IteratorKind::ResultLike))
        }
        _ => None,
    }
}

fn new_dict(source: &Type, element: &Type, kind: IteratorKind) -> IteratorDictInfo {
    IteratorDictInfo::new(source.clone(), element.clone(), kind)
}
