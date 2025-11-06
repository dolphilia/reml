# 2025-11-06 OCaml 診断メトリクス再収集メモ

- ブランチ: `git rev-parse --short HEAD` → `412bf15`
- 出力ディレクトリ: `tmp/diagnostics-output/`
- 参考ログ:
  - `reports/dual-write/20251106-validate-diagnostic-json.md`
  - `reports/dual-write/20251106-collect-iterator-metrics.json`

## 1. `scripts/validate-diagnostic-json.sh`

- コマンド: `bash scripts/validate-diagnostic-json.sh tmp/diagnostics-output/`
- 結果: 未解決のスキーマ準拠エラーにより終了コード 1。
- 主な指摘事項:
  - `effects.required_capabilities` / `effects.actual_capabilities` が JSON 拡張および `audit_metadata` / `audit.metadata` に含まれていない。
  - `parser.core.rule.*` をはじめとするコアメタデータがドット区切りキーで提供されており、バリデータが期待するネスト表現と不整合。
  - FFI・診断サンプルも同様に `effect.required_capabilities` 系が欠落。
- 備考: 現行 OCaml CLI の JSON 生成が Phase 2-5 のフィールド追加に追随していない。CLI 側での構造刷新またはハーネス側の変換レイヤーが必要。

## 2. `collect-iterator-audit-metrics.py --require-success`

- コマンド:
  ```bash
  python3 tooling/ci/collect-iterator-audit-metrics.py \
    --require-success \
    --source tmp/diagnostics-output/parser-runconfig-packrat.json \
    --source tmp/diagnostics-output/streaming-outcome.json \
    --source tmp/diagnostics-output/expected-summary.json \
    --source tmp/diagnostics-output/typeclass-iterator-stage-mismatch.json \
    --source tmp/diagnostics-output/typeclass-dictionary-resolved.json \
    --source tmp/diagnostics-output/effects-residual-leak.json \
    --source tmp/diagnostics-output/effects-stage-resolution.json \
    --source tmp/diagnostics-output/ffi-unsupported-abi.json \
    --source tmp/diagnostics-output/severity-info-hint.json \
    --audit-source tmp/diagnostics-output/audit/cli-ffi-bridge-windows.jsonl \
    --audit-source tmp/diagnostics-output/audit/effects-stage.json \
    --audit-source tmp/diagnostics-output/audit/effects-residual.jsonl
  ```
- 結果: `--require-success` 判定により終了コード 1。
- 成功メトリクス:
  - `diagnostic.audit_presence_rate`: 1.0
  - `parser.expected_summary_presence`: 1.0（関連メトリクス含め成功）
  - `parser.runconfig_*`: 1.0（RunConfig スイッチ/拡張の網羅）
- 未達成メトリクスと原因:
  - `lexer.shared_profile_pass_rate` / `lexer.identifier_profile_unicode`: `streaming-outcome.json` の `run_config.extensions.lex` 欠如、および CLI 既定の `lex.profile=unicode` 生成未対応。
  - `typeclass.metadata_pass_rate`: `typeclass_dictionary_resolved` 系診断に `typeclass.*` 拡張が部分欠落。
  - `ffi_bridge.audit_pass_rate`: 付随する監査ログの `bridge` セクション不足。
- 付随情報:
  - 集計結果の完全な JSON 出力を `reports/dual-write/20251106-collect-iterator-metrics.json` に保存。
  - 失敗要因はいずれも診断 JSON または監査ログのフィールド欠落であり、Rust 版へ移行する前に OCaml CLI 側でのメタデータ補完が必要。

## 3. 今後の TODO

1. OCaml CLI の診断シリアライザで `effect.required_capabilities` / `effect.actual_capabilities` を拡張・監査両方のメタデータに付与。
2. Parser 系メタデータをドット区切りからネスト構造へ移行する（`scripts/validate-diagnostic-json.sh` の期待値と整合）。
3. `streaming-outcome` 系 RunConfig に `lex` プロファイルを付与し、lexer メトリクスを満たす CLI フラグを文書化。
4. FFI 監査ログの `bridge` メタデータを `collect-iterator-audit-metrics.py` の必須キーと一致させる。
5. `parser.core.rule.*` の監査メタデータをネスト形式に統一しつつ、既存のドット表記と併存させる。
