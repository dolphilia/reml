# 4-0 リスク登録（Rust 移植 P4）

P4 フェーズでは最適化とハンドオーバーを進めながら、Phase 3 以降に影響するリスクを可視化して制御する必要がある。本書は [`docs/plans/bootstrap-roadmap/0-4-risk-handling.md`](../bootstrap-roadmap/0-4-risk-handling.md) の登録フォーマットを継承し、Rust 版コンパイラの仕上げ作業に特化したリスク台帳を提供する。

## 4-0.1 運用方針
- フェーズ横断で共有される基準値・測定手順は [`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`](../bootstrap-roadmap/0-3-audit-and-metrics.md) に従う。
- リスク更新は週次レビュー（`P4 Sync` ミーティング、詳細は [`4-1-communication-plan.md`](4-1-communication-plan.md)）で確認し、状態変更時は関連計画書へ脚注を追記する。
- OCaml 実装との差分がブロッカーになった場合は dual-write を復活させ、`Resolved` へ移行するまで Rust 実装によるリリースを停止する。

## 4-0.2 リスクサマリー
| ID | カテゴリ | 状態 | 概要 | 関連フェーズ |
|----|----------|------|------|--------------|
| P4-R1 | 互換性 | Open | Rust 版の性能最適化により OCaml 基準からの乖離が 10% を超える恐れ | P4（最適化） |
| P4-R2 | 安全性 | Mitigating | Capability Stage 監査の自動化範囲が Rust 最適化で欠落する可能性 | P4 → Phase 3 |
| P4-R3 | エコシステム | Open | ドキュメント更新が遅延し、Phase 3/4 のチームへ知識が伝達されないリスク | P4 → Phase 3/4 |

## 4-0.3 リスク詳細

### P4-R1 Rust ↔ OCaml 性能乖離
- 登録日: 2027-02-18
- カテゴリ: 互換性
- 詳細: P4 の最適化タスクで Rust 実装に固有のメモリ戦略やベクトル化を導入する際、OCaml 実装を基準にした `parse_throughput`・`memory_peak_ratio` が ±10% を超えて逸脱する恐れがある。逸脱を許容すると Phase 3 の Core Library ベンチマーク比較が成立せず、[`docs/plans/bootstrap-roadmap/3-2-benchmark-baseline.md`](3-2-benchmark-baseline.md) で定義したベースラインが無効化される。
- 対応案: `scripts/validate-diagnostic-json.sh` 実行後に `collect-iterator-audit-metrics.py --section bench --require-success` を追加実行し、Rust/OCaml の並列ベンチ結果を `reports/audit/dashboard/perf.md`（新規作成予定）へ記録する。差分が 8% を超えた時点で `Mitigating` に移行し、原因調査と巻き戻し計画を [`2-0-llvm-backend-plan.md`](2-0-llvm-backend-plan.md) §2.4 に反映する。
- 期限: 2027-04-05
- 状態: Open
- 関連フェーズ: P4 最適化ウィンドウ
- 参照: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` §0.3.1, `reports/diagnostic-format-regression.md` §2（差分報告手順）

### P4-R2 Capability Stage 監査の欠落
- 登録日: 2027-02-18
- カテゴリ: 安全性
- 詳細: Rust 側で `RuntimeBridgeRegistry` や `verify_capability_stage` を最適化する際に監査ログのキー（`effect.stage.required` など）が欠落する恐れがあり、Phase 2-7 で整備した監査メトリクスが再び警告を出す懸念がある。特に Windows `gnu` ターゲットの差分 CI を Rust 導入後も維持しなければ、[`docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md`](../bootstrap-roadmap/2-7-to-2-8-handover.md) で引き継いだ監査基準を満たせなくなる。
- 対応案: `collect-iterator-audit-metrics.py --section effects --require-success` と `--section streaming` を Rust CI マトリクスの `windows-msvc` / `windows-gnu` / `linux-gnu` で週次実行し、結果を [`3-1-observability-alignment.md`](3-1-observability-alignment.md) の監査ダッシュボード反映手順に従って共有する。監査キー欠落が検出された場合は 24 時間以内に `Mitigating` とし、`runtime_bridge` コンポーネントにロールバックパッチを適用して再検証する。
- 期限: 2027-03-24
- 状態: Mitigating
- 関連フェーズ: P4 最適化 → Phase 3 移行判定
- 参照: `docs/spec/3-6-core-diagnostics-audit.md` §1, `docs/spec/3-8-core-runtime-capability.md` §10, `compiler/ocaml/docs/technical-debt.md` §Backpressure

### P4-R3 ドキュメント同期遅延
- 登録日: 2027-02-18
- カテゴリ: エコシステム
- 詳細: 最終調整期間に仕様・ガイド・ノートを更新し損ねると、Phase 3 Self-Host 前提条件（[`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md`](../bootstrap-roadmap/3-0-phase3-self-host.md)）の整合が崩れる。Rust 実装の差分が `docs/spec/1-x` や `docs/guides/runtime-bridges.md` に反映されないまま Phase 3 へ進むと、セルフホスト計画が旧設計を参照したまま進行する恐れがある。
- 対応案: `4-2-documentation-sync.md` で定義するチェックリストに沿って、PR クローズ前に `docs/spec/`・`docs/guides/`・`docs/notes/` の該当セクションを更新し、`docs-migrations.log` に操作履歴を残す。同期漏れが発覚した場合は `Mitigating` に移行し、72 時間以内に該当文書へ脚注を追記して差分理由を共有する。
- 期限: 2027-02-28
- 状態: Open
- 関連フェーズ: P4 ハンドオーバー準備
- 参照: `4-1-communication-plan.md` §4, `4-2-documentation-sync.md`, `docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md` §3

---

本リスク台帳は P4 作業期間中に継続的に更新し、完了条件を満たした後は Phase 3 のリスク管理へ引き継ぐ。未解決リスクが残る場合は Phase 3 着手前レビューで必ず状態確認を行うこと。
