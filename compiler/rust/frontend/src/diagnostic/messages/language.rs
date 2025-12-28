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
    }
}
