# 2-4 → 2-5 ハンドオーバー

**作成日**: 2025-10-24  
**担当**: 診断・監査パイプラインチーム → 仕様差分補正チーム

## 1. 概要
- Phase 2-4 では診断・監査データ構造の共通化と JSON スキーマ検証フローを整備し、Phase 2-5 の仕様差分補正に必要な基盤を提供した。
- CLI/LSP 出力の差分レビュー手順を `reports/diagnostic-format-regression.md` にまとめ、仕様更新時のエビデンスを残せる状態にした。
- Windows/macOS 監査ゲートや CLI テキストフォーマット刷新など未完タスクは Phase 2-7 に移管済みである。2-5 着手時は差分リストを 2-7/2-8 と共有しつつ作業を進めること。

## 2. 引き継ぎ成果物
- **完了報告書**: `docs/plans/bootstrap-roadmap/2-4-completion-report.md`
- **計画書更新**: `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md`（現状整理・タスク移管を反映）
- **シリアライズレイヤ**: `compiler/ocaml/src/diagnostic_serialization.{ml,mli}`
- **JSON 検証スクリプト**: `scripts/validate-diagnostic-json.sh`, `tooling/lsp/tests/client_compat/validate-diagnostic-json.mjs`
- **レビュー手順**: `reports/diagnostic-format-regression.md`
- **LSP 互換レイヤ**: `tooling/lsp/lsp_transport.mli`, `tooling/lsp/compat/diagnostic_v1.ml`
- **技術的負債一覧**: `compiler/ocaml/docs/technical-debt.md`（ID 22/23 継続）

## 3. 未完了タスク（Phase 2-7 へ移管）
| ID | 項目 | 内容 | 追跡先 |
|----|------|------|--------|
| 22 | Windows Stage 自動検証不足 | Windows CI で `ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` を 1.0 でゲート化する。 | `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §1 |
| 23 | macOS FFI サンプル自動検証不足 | `ffi_dispatch_async.reml` 等を CI 監査に組み込み、`bridge.platform = macos-arm64` の pass_rate を保証。 | 同上 §1 |
| - | CLI テキスト出力刷新 | `SerializedDiagnostic` ベースのテキストフォーマッタ、`--json-mode` 集約、`--format text --no-snippet`。 | 同上 §2 |
| - | LSP V2 テスト強化 | フィクスチャ拡充、`lsp-contract` CI ジョブ導入、互換エラー記録の整理。 | 同上 §3 |
| - | 監査ダッシュボード更新 | `collect-iterator-audit-metrics.py` の出力を `reports/audit/dashboard/` に反映し、Phase 2-8 のベースラインを準備。 | 同上 §5 |

> **補足**: 上記タスクが完了するまで、仕様差分補正で参照する診断ログには欠落フィールドが残る可能性がある。差分レビュー時は `diagnostic_schema.validation_pass` の結果を確認し、欠落フィールドがある場合は 2-7 チームと連携すること。

## 4. 2-5 着手前チェックリスト
1. `docs/plans/bootstrap-roadmap/2-4-completion-report.md` と `2-4-diagnostics-audit-pipeline.md` の現状整理を確認し、差分リストの初期入力を共有する。
2. `scripts/validate-diagnostic-json.sh` をローカルで実行し、差分補正対象の診断サンプルがスキーマ検証を通過するか確認する。
3. `reports/diagnostic-format-regression.md` に従い、Phase 2-5 で更新予定のフォーマットに対するレビュー観点を洗い出す。
4. 技術的負債リスト（ID 22/23）と 2-7 計画書で扱うタスクとの差分を把握し、仕様差分補正に直接関係する項目を `docs/notes/spec-integrity-audit-checklist.md`（2-8 で作成予定）へメモしておく。

## 5. 参考リンク
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`
- `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `reports/diagnostic-format-regression.md`
- `compiler/ocaml/docs/technical-debt.md`

## 6. 備考
- 2-5 では仕様差分の抽出と修正案作成が主目的であり、診断ログの完全性は 2-7 の進捗に依存する。差分レビュー時はスキーマ検証結果と照合して作業順を調整すること。
- 2-7 で CLI テキスト出力の刷新が完了した際は、差分補正中のサンプルにも影響するため、`reports/diagnostic-format-regression.md` を経由してレビュアに共有する運用を徹底する。
