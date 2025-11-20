### Iterator Stage Audit サマリー (2025-11-20)

- 実行コマンド:
  - `RUSTFLAGS="-Zpanic-abort-tests" cargo +nightly test --manifest-path compiler/rust/frontend/Cargo.toml --test core_iter_pipeline -- core_iter_pipeline_snapshot`
  - `RUSTFLAGS="-Zpanic-abort-tests" cargo +nightly test --manifest-path compiler/rust/frontend/Cargo.toml --test core_iter_effects -- core_iter_effect_labels_snapshot core_iter_try_collect_errors_snapshot`
  - `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case pipeline --source reports/spec-audit/ch1/iter.json --output reports/iterator-stage-metrics.json`
  - `scripts/validate-diagnostic-json.sh --pattern iterator --pattern collector`
- KPI: `iterator.stage.audit_pass_rate`（schema v2.0.0-draft）。`reports/spec-audit/ch1/iter.json` を入力に `iterator.stage.audit_pass_rate = 1.0`、`collector.effect.mem = 0`、`collector.effect.mut = 0`、`TryCollectError::Collector = duplicate_key` のみであることを確認。
- Snapshots:
  - `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__core_iter_pipeline.snap`（6 ケース）
  - `compiler/rust/frontend/tests/snapshots/core_iter_effects__core_iter_effect_labels.snap`
  - `compiler/rust/frontend/tests/snapshots/core_iter_effects__core_iter_try_collect_errors.snap`
- 解析対象ケースと Stage/Effect 概要:
  - `list_roundtrip` / `from_iter_and_into_iter`: `ListCollector` 終端、`stage.actual = beta → stable`、`effects.mem=false`。
  - `map_filter_vec` / `zip_collect_list` / `buffered_mem_case` / `try_collect_success`: `VecCollector` 終端、`stage.actual = beta`、`collector.effect.mem=true` のみで `iterator.effect.mem_bytes` は `buffered_mem_case=2` のみ。
  - `pure_effects` / `buffered_effects` / `try_unfold_effects`: `EffectLabels` (`mem`, `debug`) と `IteratorStageProfile` を個別に snapshot し、`core_iter_effects__core_iter_effect_labels.snap` で `async_pending=false` を固定。
  - `core_iter_effects__core_iter_try_collect_errors.snap`: `TryCollectError::Item("boom")`、`TryCollectError::Collector(MapCollector duplicate_key)` を JSON で保存。

#### Stage トレース検証
- pipeline 6 ケース + effect 3 ケースを `reports/spec-audit/ch1/iter.json` に集約し、`collect-iterator-audit-metrics.py` の stage trace / audit metadata 必須キーをすべて充足（欠落 0）。
- `iterator.stage.audit_pass_rate = 1.0`、`collector.effect.mem = 0`（`VecCollector` の `mem=true` は KPI 例外としてロギングのみ）、`collector.error.invalid_encoding = 0`、`collector.error.duplicate_key` は `TryCollectError::Collector` case のみ。

#### Iter F3 KPI 連携
- `reports/spec-audit/ch1/iter.json` には `pipeline`/`effects` のケース配列、対応する snapshot パス、`iterator.stage.audit_pass_rate`/`collector.effect.mem`/`iterator.effect.debug` の KPI 値を格納。`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `Iter` エントリとリンクさせ、API 棚卸しから証跡へ辿れる構成にした。
- `reports/spec-audit/ch0/links.md#iterator-f3` に `cargo +nightly test`・`collect-iterator-audit-metrics.py --section iterator --case pipeline --source reports/spec-audit/ch1/iter.json`・`scripts/validate-diagnostic-json.sh --pattern iterator --pattern collector` の実行ログを追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI と同期している。
