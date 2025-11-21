### Iterator Stage Audit サマリー (2025-11-21)

- 実行コマンド:
  - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml --test core_iter_pipeline`
  - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml --test core_iter_effects`
  - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml --test core_iter_collectors`
  - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml --test core_iter_terminators`
  - `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case pipeline --source compiler/ocaml/tests/golden/_actual/typeclass_iterator_stage_mismatch.actual.json --output reports/iterator-stage-metrics.json --require-success`
- KPI: `iterator.stage.audit_pass_rate`（schema v2.0.0-draft）。`reports/spec-audit/ch1/iter.json` を入力に `iterator.stage.audit_pass_rate = 1.0`、`collector.effect.mem = 0`、`collector.effect.mut = 0`、`TryCollectError::Collector = duplicate_key` のみであることを確認。
- Snapshots:
  - `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__core_iter_pipeline.snap`（6 ケース）
  - `compiler/rust/frontend/tests/snapshots/core_iter_effects__core_iter_effect_labels.snap`
  - `compiler/rust/frontend/tests/snapshots/core_iter_effects__core_iter_try_collect_errors.snap`
- 解析対象ケースと Stage/Effect 概要:
  - `list_roundtrip` / `from_iter_and_into_iter`: `ListCollector` 終端（`stage.actual = beta → stable`）。`EffectLabels` は `mem=false`/`predicate_calls=0` を維持。
  - `map_filter_vec`: `Iter::filter` が `iterator.effect.mut=true` / `predicate_calls=1` を露出し、`VecCollector` 終端では `collector.effect.mut=true` を保持。
  - `zip_collect_list`: 長さ調整のため `iterator.effect.mut=true` を報告。
  - `buffered_mem_case`: `Iter::buffered(2, DropOldest)` で `iterator.effect.mem=true` / `mem_bytes=2`。
  - `pure_effects` / `buffered_effects` / `try_unfold_effects`: `core_iter_effects__core_iter_effect_labels.snap` に `mut=false` / `predicate_calls=0` を追記し、`try_unfold` のみ `debug=true`。
  - `core_iter_effects__core_iter_try_collect_errors.snap`: `TryCollectError::Item("boom")` と `TryCollectError::Collector(MapCollector duplicate_key)` を JSON 化し、`collector.effect.predicate_calls=0` を監査メタデータへ追加。

#### Stage トレース検証
- pipeline 6 ケース + effect 3 ケースを `reports/spec-audit/ch1/iter.json` に集約し、`iterator.effect.*` に `predicate_calls` を追加。`collect-iterator-audit-metrics.py` は OCaml 由来の `typeclass.iterator.stage_mismatch` ダンプを参照して KPI を再計測し、必須フィールド欠落 0 を確認。
- `iterator.stage.audit_pass_rate = 1.0`、`collector.effect.mem = 0`（`VecCollector` の `mem=true` は KPI ログのみ）、`collector.error.invalid_encoding = 0`、`collector.error.duplicate_key = 1`（`TryCollectError::Collector` ケースのみ）。

#### Iter F3 KPI 連携
- `reports/spec-audit/ch1/iter.json` には `pipeline`/`effects` ケース配列と snapshot パス、`iterator.stage.audit_pass_rate`/`collector.effect.mem` 等の KPI 値を保持。`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `Iter` エントリと連携済み。
- `reports/spec-audit/ch0/links.md#iterator-f3` に今回の `cargo test` / `collect-iterator-audit-metrics.py` 実行ログを追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 記載と同期した。
