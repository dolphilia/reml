# 3.1 Core Prelude 実装課題リメディエーション計画

## 目的
- `Core.Prelude` / `Core.Iter` 実装で観測された欠落・不整合を列挙し、次スプリントで解消する具体的タスクと検証手段を定義する。
- `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の実装観点を補完し、Rust 実装・監査メトリクス・ドキュメント整合を同時に改善する。

## 前提資料
- 仕様: [docs/spec/3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md)
- 実装: `compiler/runtime/ffi/src/core_prelude/*.rs`, `compiler/runtime/src/prelude/**/*`
- テスト: `compiler/frontend/tests/core_prelude_option_result.rs`, `core_iter_*.rs`
- 監査資産: `reports/spec-audit/ch0/links.md`, `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`

## 課題一覧と対応計画

### 課題A: `Iter::drain_into_collector` が iterator 側エラーを握りつぶす
- **現状**: `compiler/runtime/src/prelude/iter/mod.rs:297-305` で `IterStep::Error(_)` を受け取っても `Ok(collector.finish())` を返しているため、`collect` 系 API が iterator 内部の失敗を観測できない。
- **影響**: `Iter.try_collect` 以外の終端操作が `IterError` を診断化できず、仕様 §3.4 の「Collector が item/driver エラーを区別して報告する」要件を満たさない。
- **対応ステップ**:
  1. `Iter::drain_into_collector` に `IterError` を `Collector::Error` へラップして返す経路を追加。`TryCollectError::Iter` 相当の型がない場合は `CollectError` へ `collector.iter_error` メタデータを添付。
  2. `core_iter_pipeline.rs` に「iterator error が `collect_vec` から伝播する」テストケースを追加し、snapshot に `collector.iter_error` を固定。
  3. `tooling/ci/collect-iterator-audit-metrics.py` の対象に新テストを追加し、`reports/spec-audit/ch1/core_iter_pipeline.json`へ `iterator.error.iter_driver` 指標を追記。
- **出口条件**: `total` エラー数が `reports/iterator-collector-summary.md` に記録され、`collect_vec` で `IterError::Buffer` が `Collector` レイヤへ伝播していることを確認。

### 課題B: API インベントリの実装ステータス・ファイル参照が陳腐化
- **現状**:
  - `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml:142-158` で `Result.ensure` が `rust_status = "pending"` のまま。
  - 同ファイルの `collect_*` 項目が存在しない `iter/terminators.rs` を参照しており、最新の `iter/mod.rs` / `collectors/*.rs` へ追従していない。
- **影響**: `cargo xtask prelude-audit --strict` で false positive が発生し、運用チームが誤って未実装と判断するリスクがある。
- **対応ステップ**:
  1. `Result.ensure` 相当の Guard 実装 (`compiler/runtime/src/prelude/ensure.rs`) をインベントリへ反映し、`rust_status` を `implemented` に更新。
  2. すべての `collect_*` ノートで実際のソースパス (`iter/mod.rs` / `collectors/*.rs`) とテスト (`core_iter_collectors.rs`, `core_iter_pipeline.rs`) を引用。
  3. インベントリ更新後に `scripts/validate-diagnostic-json.sh` と `cargo xtask prelude-audit --strict` を実行し、`core_prelude.missing_api = 0` を `reports/spec-audit/ch0/links.md` に追記。
- **出口条件**: `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `last_updated` をリフレッシュし、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI が最新ステータスを参照。

**対応状況（2027-03-19）**: `prelude_api_inventory.toml` を更新し、`Result.ensure` を `rust_status=implemented`、`collect_*` 系のソース/テスト参照を `iter/mod.rs`・`collectors/string.rs`・`core_iter_pipeline.rs`・`core_iter_collectors.rs` へ刷新。`cargo run --manifest-path compiler/xtask/Cargo.toml -- prelude-audit --section Result --strict` のログを `reports/spec-audit/ch0/links.md` に記録済み。

### 課題C: `Iter::empty`/`Iter::once` など基本 API のテストカバレッジ不足
- **現状**: `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml:192-200` にも記載のとおり、`Iter::empty`/`once` は Rust 実装済みだが専用テストが存在しない。`core_iter_generators.rs` では間接利用のみ。
- **影響**: 将来の最適化で `Iter::empty` が Stage/Effect 情報を落とした場合に検知できない。`WBS 3.1` のベンチ/KPI でギャップが残る。
- **対応ステップ**:
  1. `compiler/frontend/tests/core_iter_generators.rs` に `iter_empty_returns_none`、`iter_once_delivers_single_value` を追加し、`stage_snapshot`/`effect_labels` を snapshot で固定。
  2. `reports/spec-audit/ch1/core_iter_generators.json` に新ケースを登録し、`collect-iterator-audit-metrics.py --case generators` へフィード。
  3. `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の該当 `notes` を更新し、テスト参照を「直接検証あり」と明記。
- **出口条件**: `core_iter_generators` snapshot 更新が `reports/spec-audit/diffs/README.md` に記録され、`iterator.effect.mem_bytes` 等の KPI が `0`（空）/`PURE` と一致する。

**対応状況（2027-03-19）**: `compiler/frontend/tests/core_iter_generators.rs` に `empty_iter_reports_pure_stage` / `once_iter_emits_single_value_and_stage` を追加し、`cargo test --manifest-path compiler/frontend/Cargo.toml core_iter_generators` で通過を確認。`prelude_api_inventory.toml` の `collect_*` ノート更新により新テストを参照済み。`reports/spec-audit/ch1/` 側の追加登録は Phase 3-2 KPI 更新タスクで実施予定。

## マイルストーンと責任
| WBS | タスク | 担当 | 期日 | 依存 |
| --- | --- | --- | --- | --- |
| 3.1c-Fix-A | 課題A 実装＋テスト | Runtime チーム | W38 木 | `core_iter_pipeline` snapshot 更新 |
| 3.1c-Fix-B | インベントリ更新＋検証 | Docs/Audit | W38 金 | 課題Aのメトリクス差分反映 |
| 3.1c-Fix-C | 追加テスト＋レポート | Frontend QA | W38 金 | 無し |

## リスクとフォローアップ
- **監査ログ差分**: `Iter::drain_into_collector` の修正で `collector.effect.*` が変動する可能性があるため、`reports/iterator-collector-summary.md` に結果を追記し Phase 3-2 に共有する。
- **ドキュメント更新漏れ**: インベントリ更新に併せて `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の完了チェックリストも見直す。必要に応じて `docs-migrations.log` へ作業ログを追加。
