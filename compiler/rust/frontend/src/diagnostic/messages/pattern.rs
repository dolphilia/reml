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
            message: "Active Pattern の戻り値は Option<T>（部分）または T（完全）のみ許可されます。Result など別の型は使用できません。",
            severity: DiagnosticSeverity::Error,
        },
        PatternDiagnosticMessage {
            code: "pattern.active.effect_violation",
            title: "純粋 Active Pattern での副作用",
            message: "`@pure` Active Pattern で副作用が検出されました。副作用を除去するか純粋でない関数へ移動してください。",
            severity: DiagnosticSeverity::Error,
        },
        PatternDiagnosticMessage {
            code: "pattern.guard.if_deprecated",
            title: "`if` ガードは非推奨です",
            message: "`match` のガードは正規形の `when` を使用してください（`if` は互換目的のエイリアスです）。",
            severity: DiagnosticSeverity::Warning,
        },
        PatternDiagnosticMessage {
            code: "pattern.binding.duplicate_name",
            title: "パターン束縛が重複しています",
            message: "`as` や `@` で同じ名前を複数回束縛しています。",
            severity: DiagnosticSeverity::Error,
        },
        PatternDiagnosticMessage {
            code: "pattern.regex.invalid_syntax",
            title: "正規表現パターンの構文が不正です",
            message: "r\"...\" 形式の正規表現パターンが無効です。表記を見直してください。",
            severity: DiagnosticSeverity::Error,
        },
        PatternDiagnosticMessage {
            code: "pattern.regex.unsupported_target",
            title: "正規表現パターンの適用対象が不正です",
            message: "正規表現パターンは文字列またはバイト列にのみ適用できます。",
            severity: DiagnosticSeverity::Error,
        },
        PatternDiagnosticMessage {
            code: "pattern.range.type_mismatch",
            title: "範囲パターンの型が一致しません",
            message: "範囲パターンの境界と対象の型は同じ比較可能な型である必要があります。",
            severity: DiagnosticSeverity::Error,
        },
        PatternDiagnosticMessage {
            code: "pattern.range.bound_inverted",
            title: "範囲パターンの上下限が逆転しています",
            message: "開始境界が終了境界より大きくなっています。境界の順序を見直してください。",
            severity: DiagnosticSeverity::Error,
        },
        PatternDiagnosticMessage {
            code: "pattern.slice.type_mismatch",
            title: "スライスパターンの適用対象が不正です",
            message: "スライスパターンは Array など反復可能な型にのみ適用できます。",
            severity: DiagnosticSeverity::Error,
        },
        PatternDiagnosticMessage {
            code: "pattern.slice.multiple_rest",
            title: "スライスパターンで `..` が多重に指定されています",
            message: "`..` は 1 回のみ使用できます。パターンを 1 つにまとめてください。",
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
            severity: DiagnosticSeverity::Error,
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
        assert!(find_pattern_message("pattern.guard.if_deprecated").is_some());
        assert!(find_pattern_message("pattern.binding.duplicate_name").is_some());
        assert!(find_pattern_message("pattern.regex.invalid_syntax").is_some());
        assert!(find_pattern_message("pattern.regex.unsupported_target").is_some());
        assert!(find_pattern_message("pattern.range.type_mismatch").is_some());
        assert!(find_pattern_message("pattern.range.bound_inverted").is_some());
        assert!(find_pattern_message("pattern.slice.type_mismatch").is_some());
        assert!(find_pattern_message("pattern.slice.multiple_rest").is_some());
        assert!(find_pattern_message("pattern.exhaustiveness.missing").is_some());
        assert!(find_pattern_message("pattern.unreachable_arm").is_some());
        assert!(find_pattern_message("pattern.active.name_conflict").is_some());
    }
}
