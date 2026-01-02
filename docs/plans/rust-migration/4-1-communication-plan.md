# 4-1 コミュニケーション計画（Rust 移植 P4）

P4 フェーズでは最適化とハンドオーバーを同時進行させるため、Rust チーム・監査チーム・ドキュメントチームの連携を密に保つ必要がある。本計画は [`docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md`](../bootstrap-roadmap/2-7-to-2-8-handover.md) のハンドオーバーパターンを踏襲し、Phase 3 Self-Host 前提条件（[`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md`](../bootstrap-roadmap/3-0-phase3-self-host.md)）へスムーズに接続できる体制を定義する。

## 4-1.1 ステークホルダー
| チーム | 役割 | 主要責務 | 主な参照資料 |
|--------|------|----------|--------------|
| Rust 実装チーム | 実装・最適化 | LLVM/Runtime/Adapter 層の最適化、ベンチ計測、差分分析 | `2-0-llvm-backend-plan.md`, `2-1-runtime-integration.md`, `2-2-adapter-layer-guidelines.md` |
| 監査・CI チーム | 計測・ゲート管理 | `collect-iterator-audit-metrics.py` の閾値監視、CI マトリクス更新、監査ログ整備 | `3-0-ci-and-dual-write-strategy.md`, `3-1-observability-alignment.md`, `3-2-benchmark-baseline.md` |
| ドキュメントチーム | 仕様同期・脚注管理 | `docs/spec/`・`docs/guides/` などの更新とレビュー調整、`docs-migrations.log` 記録 | `4-2-documentation-sync.md`, `docs/spec/0-1-project-purpose.md` |
| Phase 3 受入チーム | ハンドオーバー検証 | Self-Host 前提条件確認、リスクレビュー、受け入れ判定 | `3-0-phase3-self-host.md`, `docs/notes/stdlib/core-library-outline.md` |

## 4-1.2 コミュニケーションカレンダー
| 名称 | 参加者 | 頻度 | 議題 | 成果物 |
|------|--------|------|------|--------|
| P4 Sync | 全チーム | 週次（月曜） | リスクレビュー、進捗確認、`0-3-audit-and-metrics.md` 更新状況 | `meeting-notes/p4-sync-YYYYMMDD.md`（`docs/notes/`） |
| Perf Deep Dive | Rust 実装 + 監査 | 隔週（水曜） | ベンチ結果分析、最適化計画、ロールバック判断 | `reports/audit/dashboard/perf.md` 更新記録 |
| Docs Alignment Check | ドキュメント + Phase 3 受入 | 週次（木曜） | 仕様・ガイド・ノートの差分確認、脚注レビュー | `docs/plans/rust-migration/4-2-documentation-sync.md` チェックリスト更新 |
| Release Gate Review | 監査 + Phase 3 受入 | マイルストーン到達時 | CI/監査ゲートの合格可否、リリース候補判定 | `meeting-notes/release-gate-YYYYMMDD.md`（`docs/notes/`） |

## 4-1.3 情報共有チャネル
- **成果物保管**: `docs/plans/rust-migration/` を一次保管ディレクトリとし、長期保管ログは `docs/notes/` に移管する。移管時は `docs-migrations.log` を更新。
- **差分通知**: PR では `unified-porting-principles` ラベルを付与し、レビュー依頼時に対象ドキュメントとメトリクスを明記する（例: 「`collect-iterator-audit-metrics.py --section effects --require-success` の結果を `reports/audit/dashboard/diagnostics.md` に反映済み」）。
- **意思決定ログ**: 重要な設計判断は `docs/notes/rust-migration-decisions.md`（新設予定）に 5W1H で記録し、`Docs Alignment Check` で共有する。

## 4-1.4 エスカレーションと承認フロー
1. `P4 Sync` でリスク状態を確認し、`Open` のまま期限が 7 日以内に迫る場合は担当リーダーがエスカレーションプランを提出する。
2. `Release Gate Review` で `stage_mismatch_count > 0` や `parse_throughput` 負荷逸脱が確認された場合、`Phase 3 受入チーム` の承認が得られるまで Rust リリース判定を保留する。
3. ドキュメント同期が 72 時間以上遅延した場合は、`Docs Alignment Check` で `Mitigating` → `Open` へ戻し、`4-0-risk-register.md` の `P4-R3` を更新する。

## 4-1.5 ハンドオーバー準備
- Phase 3 受入判定の 1 週間前に `Rust → OCaml` dual-write の比較結果、性能レポート、監査ログ、ドキュメント差分をまとめた「P4 サマリー」を作成し、`docs/plans/rust-migration/` 直下に配置する。
- 判定会議では以下を確認する：
  1. `collect-iterator-audit-metrics.py --section diagnostics/effects/streaming --require-success` の最新結果が `reports/audit/dashboard/` に反映済みか。
  2. `docs/spec/` の差分に脚注が挿入され、`docs-migrations.log` に記録されているか。
  3. `4-0-risk-register.md` の全リスクが `Resolved` もしくは Phase 3 のリスク台帳へ移管済みか。
- 会議後 24 時間以内に議事録を `docs/notes/phase3-handover/p4-summary-YYYYMMDD.md`（既存階層を再利用）として残し、Phase 3 チームへ通知する。

---

P4 作業期間中は本計画を参照し、会議頻度や成果物に変更が生じた場合は最新版へ必ず反映すること。
