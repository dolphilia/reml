# Phase 2-4 診断・監査パイプライン 完了報告書

> 作成日: 2025-10-24  
> 担当: 診断・監査パイプラインチーム（Phase 2-4）

## 1. サマリー
- `Diagnostic`/`AuditEnvelope` 共通シリアライズ層の導入と CLI/LSP への組み込みにより、フェーズ後半の仕様改訂へ備える基盤を整備した。
- JSON スキーマ検証と AJV ベースの互換テストを CI に追加し、`ffi_bridge.audit_pass_rate` の評価に必要なフィールド欠落を検知できる状態を整えた。
- 仕様差分レビューに向けて `reports/diagnostic-format-regression.md` を作成し、フォーマット変更時のレビュー手順を定義した。
- CLI テキスト出力刷新や Windows/macOS 監査ゲートなどの残タスクは Phase 2-7「診断パイプライン残課題・技術的負債整理計画」へ移管した。

## 2. 達成事項
1. **共通シリアライズ層の実装**
   - `compiler/ocaml/src/diagnostic_serialization.{ml,mli}` を新設し、`Diagnostic.t` / `AuditEnvelope.t` から `SerializedDiagnostic` を生成するパスを整備。
   - `compiler/ocaml/src/cli/json_formatter.ml` と `tooling/lsp/diagnostic_transport.ml` を新レイヤ経由に置き換え、CLI/LSP 両チャネルで同一フィールド集合を扱えるようにした。
2. **JSON スキーマ検証フローの構築**
   - `scripts/validate-diagnostic-json.sh` を追加し、`tooling/lsp/tests/client_compat/validate-diagnostic-json.mjs` と連携して AJV 検証を自動化。
   - Linux/Windows/macOS 向け CI ワークフローにスキーマ検証ステップを追加し、欠落フィールドを `ffi_bridge.audit_pass_rate` などのメトリクスに反映可能な状態へ更新。
3. **レビュー指針と差分記録の整備**
   - `reports/diagnostic-format-regression.md` を作成し、フォーマット差分レビューのチェックリスト・比較手順・エビデンス保存先を定義。
   - `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` に進捗サマリーを追記し、完了済み・着手中・移管済みタスクを明文化。
4. **LSP V2 互換レイヤの分離と試験環境の準備**
   - `tooling/lsp/lsp_transport.mli` と `tooling/lsp/compat/diagnostic_v1.ml` を追加し、V1/V2 のトランスポート切り替え基盤を確立。
   - AJV 検証を `npm run ci` へ組み込み、FFI サンプルフィクスチャ（`diagnostic-v2-ffi-sample.json`）を追加。

## 3. 未完了タスク（Phase 2-7 へ移管）
| ID | 内容 | 参照先 | 備考 |
|----|------|--------|------|
| 22 | Windows Stage 自動検証不足 | `compiler/ocaml/docs/technical-debt.md` | Windows CI で `ffi_bridge.audit_pass_rate`/`iterator.stage.audit_pass_rate` をゲート化。 |
| 23 | macOS FFI サンプル自動検証不足 | 同上 | `ffi_dispatch_async.reml` 等の監査ゲートを macOS CI に導入。 |
| - | CLI テキストフォーマット刷新 | `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §2 | `SerializedDiagnostic` ベースのテキスト出力、`--json-mode` 集約。 |
| - | LSP V2 テスト・CI 強化 | 同上 §3 | フィクスチャ拡充、`lsp-contract` ジョブ追加、互換エラー記録の整理。 |
| - | 監査メトリクスのダッシュボード統合 | 同上 §5 | Phase 2-8 のベースラインとして `reports/audit/dashboard/` を更新。 |

## 4. メトリクス
| 指標 | 現状 | 備考 |
|------|------|------|
| `diagnostic_schema.validation_pass` | 1.0（Linux/macOS/Windows） | AJV 検証を CI に追加、欠落フィールド検出可。 |
| `ffi_bridge.audit_pass_rate` | 0.0（Windows/macOS CI） | スキーマ整合待ち。ID 22/23 で解消予定。 |
| `iterator.stage.audit_pass_rate` | 0.0（Windows CI） | Windows 監査ゲート未導入。Phase 2-7 対応。 |
| `lsp_ci.schema_validation` | 1.0（`npm run ci` 手動実行） | フィクスチャは最低限。追加ケースは 2-7 で実装。 |

## 5. フォローアップ
1. Phase 2-7 にて CLI テキスト出力刷新と Windows/macOS 監査ゲートを完了させ、Phase 2-5 の差分補正で参照する診断ログを安定化させる。
2. LSP V2 互換テストを拡充し、`docs/guides/dsl/plugin-authoring.md` に V2 連携手順を追記した上で `lsp-contract` CI ジョブを導入する。
3. `tooling/ci/collect-iterator-audit-metrics.py` の集計結果を `reports/audit/dashboard/` に反映し、Phase 2-8 の仕様監査に備えてベースラインを確定する。

## 6. 添付・参照
- `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md`
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`
- `scripts/validate-diagnostic-json.sh`
- `tooling/lsp/tests/client_compat/validate-diagnostic-json.mjs`
- `reports/diagnostic-format-regression.md`
- `compiler/ocaml/docs/technical-debt.md`（ID 22, 23）
