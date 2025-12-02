# GDPR/Privacy 監査ログ

このディレクトリでは GDPR/Privacy 監査タスクの実行ログと Run ID を保存し、
`docs/plans/bootstrap-roadmap/0-4-risk-handling.md#0.4.6-現在のリスク登録` に記録した
リスクの根拠データとして参照できるようにする。

## 20290705-privacy-redaction
- Run ID: `20290705-privacy-redaction`
- 概要: `reml_frontend --audit-policy anonymize_pii=on` で生成される `privacy.*` メタデータを検証する
- コマンド:
  - `scripts/validate-diagnostic-json.sh --suite audit --require-privacy reports/audit/privacy/20290705-privacy-redaction.jsonl`
  - `cargo test -p reml_runtime audit_snapshot`
- 期待値: `AuditEnvelope.metadata["privacy.redacted"] = true` により PII 除去済みの監査ログを識別できること
- 成果物: `20290705-privacy-redaction.jsonl`（GDPR テストケース）、`compiler/rust/runtime/tests/audit_snapshot.rs`
