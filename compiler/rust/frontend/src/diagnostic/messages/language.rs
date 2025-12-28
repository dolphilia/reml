use crate::diagnostic::DiagnosticSeverity;

#[derive(Debug, Clone)]
pub struct LanguageDiagnosticMessage {
    pub code: &'static str,
    pub title: &'static str,
    pub message: &'static str,
    pub severity: DiagnosticSeverity,
}

pub fn find_language_message(code: &str) -> Option<&'static LanguageDiagnosticMessage> {
    language_messages().iter().find(|entry| entry.code == code)
}

pub fn language_messages() -> &'static [LanguageDiagnosticMessage] {
    static REGISTRY: &[LanguageDiagnosticMessage] = &[
        LanguageDiagnosticMessage {
            code: "parser.rec.invalid_form",
            title: "`rec` の形式が不正です",
            message: "`rec <ident>` の形式のみ受理されます。",
            severity: DiagnosticSeverity::Error,
        },
        LanguageDiagnosticMessage {
            code: "parser.rec.unsupported_position",
            title: "`rec` の位置が不正です",
            message: "`rec` は代入対象として使用できません。",
            severity: DiagnosticSeverity::Error,
        },
        LanguageDiagnosticMessage {
            code: "parser.lambda.param_missing",
            title: "ラムダ引数がありません",
            message: "ラムダ式には 1 つ以上の引数を指定してください。",
            severity: DiagnosticSeverity::Error,
        },
        LanguageDiagnosticMessage {
            code: "typeck.lambda.capture_unsupported",
            title: "キャプチャ付きラムダは未実装です",
            message: "ラムダが外側の束縛を参照しています。引数で明示してください。",
            severity: DiagnosticSeverity::Error,
        },
        LanguageDiagnosticMessage {
            code: "typeck.lambda.capture_mut_unsupported",
            title: "可変キャプチャは未実装です",
            message: "ラムダ内で外側の束縛を更新しています。引数で明示してください。",
            severity: DiagnosticSeverity::Error,
        },
        LanguageDiagnosticMessage {
            code: "typeck.rec.unresolved_ident",
            title: "`rec` 参照が未解決です",
            message: "`rec <ident>` の参照先が見つかりません。",
            severity: DiagnosticSeverity::Error,
        },
        LanguageDiagnosticMessage {
            code: "type.unresolved_ident",
            title: "型参照が未解決です",
            message: "型名に対応する `type` 宣言が見つかりません。",
            severity: DiagnosticSeverity::Error,
        },
        LanguageDiagnosticMessage {
            code: "type.alias.cycle",
            title: "型エイリアスが循環参照しています",
            message: "型エイリアスの循環参照が検出されました。",
            severity: DiagnosticSeverity::Error,
        },
        LanguageDiagnosticMessage {
            code: "type.alias.expansion_limit",
            title: "型エイリアスの展開上限に達しました",
            message: "型エイリアスの展開が上限に達しました。",
            severity: DiagnosticSeverity::Error,
        },
        LanguageDiagnosticMessage {
            code: "type.sum.constructor_arity_mismatch",
            title: "合成型コンストラクタの引数数が一致しません",
            message: "合成型コンストラクタへ渡した引数の数が期待と一致しません。",
            severity: DiagnosticSeverity::Error,
        },
    ];
    REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_language_codes() {
        assert!(find_language_message("parser.rec.invalid_form").is_some());
        assert!(find_language_message("parser.rec.unsupported_position").is_some());
        assert!(find_language_message("parser.lambda.param_missing").is_some());
        assert!(find_language_message("typeck.lambda.capture_unsupported").is_some());
        assert!(find_language_message("typeck.lambda.capture_mut_unsupported").is_some());
        assert!(find_language_message("typeck.rec.unresolved_ident").is_some());
        assert!(find_language_message("type.unresolved_ident").is_some());
        assert!(find_language_message("type.alias.cycle").is_some());
        assert!(find_language_message("type.alias.expansion_limit").is_some());
        assert!(find_language_message("type.sum.constructor_arity_mismatch").is_some());
    }
}
