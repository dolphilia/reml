use crate::diagnostic::DiagnosticSeverity;

#[derive(Debug, Clone)]
pub struct PatternDiagnosticMessage {
    pub code: &'static str,
    pub title: &'static str,
    pub message: &'static str,
    pub severity: DiagnosticSeverity,
}

pub fn find_pattern_message(code: &str) -> Option<&'static PatternDiagnosticMessage> {
    pattern_messages().iter().find(|entry| entry.code == code)
}

pub fn pattern_messages() -> &'static [PatternDiagnosticMessage] {
    static REGISTRY: &[PatternDiagnosticMessage] = &[
        PatternDiagnosticMessage {
            code: "pattern.active.return_contract_invalid",
            title: "Active Pattern の戻り値契約違反",
            message: "Active Pattern の戻り値は Option<T> または完全パターンの T に限定されます。",
            severity: DiagnosticSeverity::Error,
        },
        PatternDiagnosticMessage {
            code: "pattern.active.effect_violation",
            title: "純粋 Active Pattern での副作用",
            message: "`@pure` Active Pattern で副作用が検出されました。副作用を除去するか純粋でない関数へ移動してください。",
            severity: DiagnosticSeverity::Error,
        },
        PatternDiagnosticMessage {
            code: "pattern.active.name_conflict",
            title: "Active Pattern 名が衝突しています",
            message: "同一モジュール内で Active Pattern 名が別のシンボルと衝突しています。",
            severity: DiagnosticSeverity::Error,
        },
        PatternDiagnosticMessage {
            code: "pattern.exhaustiveness.missing",
            title: "match の網羅性が不足しています",
            message: "この match はすべての入力を網羅していません。未処理のケースを追加してください。",
            severity: DiagnosticSeverity::Warning,
        },
        PatternDiagnosticMessage {
            code: "pattern.unreachable_arm",
            title: "到達不能なパターンがあります",
            message: "前段のパターンによりこのアームは到達不能です。順序を見直すか冗長なアームを削除してください。",
            severity: DiagnosticSeverity::Warning,
        },
    ];
    REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_all_pattern_codes() {
        assert!(find_pattern_message("pattern.active.return_contract_invalid").is_some());
        assert!(find_pattern_message("pattern.active.effect_violation").is_some());
        assert!(find_pattern_message("pattern.exhaustiveness.missing").is_some());
        assert!(find_pattern_message("pattern.unreachable_arm").is_some());
        assert!(find_pattern_message("pattern.active.name_conflict").is_some());
    }
}
