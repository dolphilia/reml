# 2-7 → 2-8 ハンドオーバー

**作成日**: 2026-12-21  
**担当**: Phase 2-7 Diagnostics/Plugin/Effects チーム → Phase 2-8 仕様監査チーム

## 1. 概要
- Phase 2-7 では診断・監査パイプラインの残課題と技術的負債（ID22/23、Unicode 識別子、効果行統合、Streaming PoC）を解消し、Phase 2-8 の仕様完全性監査で利用するメトリクスとログを整備した。
- 完了報告書 (`docs/plans/bootstrap-roadmap/2-7-completion-report.md`) に達成状況と未移管課題を記録済み。Phase 2-8 は同報告書と本ノートを併用し、監査計画 (`docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`) の前提を確認すること。

## 2. 引き継ぎ成果物
| 項目 | 内容 | 参照 |
|------|------|------|
| 完了報告書 | Phase 2-7 の成果・未完了タスク・メトリクス | `docs/plans/bootstrap-roadmap/2-7-completion-report.md` |
| メトリクスログ | 診断・Streaming・効果行・Unicode 指標の最新値 | `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`, `reports/audit/dashboard/diagnostics.md`, `reports/audit/dashboard/streaming.md` |
| 監査データ | `collect-iterator-audit-metrics.py` の集計 JSON | `reports/audit/phase2-7/diagnostics-domain-20261221.json`, `reports/audit/phase2-7/macos-ffi-bridge.audit.jsonl`, `reports/audit/phase2-7/windows-ffi-bridge.audit.jsonl` |
| リスク登録 | 診断ドメイン KPI 逸脱時のエスカレーション | `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#diagnostic-domain-metrics` |
| 差分ログ | 仕様差分と脚注撤去条件の更新 | `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §5 |

## 3. 未完了タスク（Phase 2-8 対応）
| テーマ | 内容 | 追跡先 | 期限目安 |
|--------|------|--------|----------|
| 効果構文 Stage 監査 H-O3/H-O5 | CLI/LSP/監査ログで Stage 監査が整合しているか最終レビュー | `docs/notes/effect-system-tracking.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §8.2〜§8.3 | 2027-01-31 |
| Plugin 互換モード廃止 | `diagnostics.plugin_bundle_ratio` を 1.0 に引き上げるため、互換モードテストを更新 | `reports/audit/dashboard/diagnostics.md`, `docs/notes/dsl-plugin-roadmap.md` | 2027-02-15 |
| 仕様差分統合 | 差分リストと監査ログを Phase 2-8 仕様監査のベースに統合 | `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`, `docs/notes/spec-integrity-audit-checklist.md`（新設） | Phase 2-8 Week36 |

## 4. Phase 2-8 着手前チェックリスト
1. `collect-iterator-audit-metrics.py --section diagnostics --require-success --write-json reports/audit/phase2-8/diagnostics-domain-<date>.json` をリハーサル実行し、閾値逸脱時のエスカレーションが機能することを確認する。
2. `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` 前提節と本書 §2 を突き合わせ、仕様差分ログへの導線が切れていないことを確認する。
3. `reports/diagnostic-format-regression.md` のチェックリストを最新化し、Phase 2-8 で想定するフォーマット変更レビュー項目を追加する。
4. `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の 0.3.7b/0.3.7c を参照し、CI で必要な `--section effects` / `--section diagnostics` ゲートが有効になっていることを CI 設定ファイルで再確認する。

## 5. 参考資料
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`
- `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`
- `docs/plans/bootstrap-roadmap/0-4-risk-handling.md`
- `docs/notes/effect-system-tracking.md`
- `docs/notes/dsl-plugin-roadmap.md`
- `reports/audit/dashboard/diagnostics.md`, `reports/audit/dashboard/streaming.md`
