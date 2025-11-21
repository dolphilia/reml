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

#### <a id="iter-adapters"></a><a id="flat-map"></a>Iter Adapter G2 (flat_map / zip)
- 実行コマンド:
  - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_adapters -- --include-ignored flat_map_vec zip_mismatch`
  - `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case flat_map --case zip --output reports/iterator-flatmap-metrics.json --secondary-output reports/iterator-zip-metrics.json --require-success`
  - `scripts/validate-diagnostic-json.sh --pattern iterator.flat_map --pattern iterator.zip reports/spec-audit/ch1/core_iter_adapters.json`
- KPI: `iterator.flat_map.mem_reservation = 3 byte`（`EffectLabels.mem=true`）、`iterator.zip.shorter_error_rate = 1.0`（`iterator.error.zip_shorter = 1/1`）。いずれも Stage 要件 (`Exact(beta)` / `Exact(stable)`) を満たし、`iterator.stage.audit_pass_rate = 1.0` を維持。
- Snapshots: `compiler/rust/frontend/tests/snapshots/core_iter_adapters__core_iter_adapters.snap`（`flat_map_vec` / `zip_mismatch`）。
- 連携資料: `reports/spec-audit/ch0/links.md#iter-adapters`, `reports/iterator-flatmap-metrics.json`, `reports/iterator-zip-metrics.json`, `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` §4.a, `docs/notes/core-library-outline.md#iter-g2-flat-zip`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#iterator-adapter-esc`.

#### <a id="iter-buffered"></a>Iter Adapter G3 (buffered/backpressure)
- 実行コマンド:
  - `cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_adapters -- --include-ignored buffered_window`
  - `cargo bench -p compiler-rust-frontend iter_buffered -- warmup-time 3 --measurement-time 10`
  - `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case buffered --output reports/iterator-buffered-metrics.json --require-success`
  - `scripts/validate-diagnostic-json.sh --pattern iterator.buffered reports/spec-audit/ch1/core_iter_adapters.json`
- KPI: `iterator.mem.window.bytes = 2`、`iterator.mem.window.backpressure = 0.33`（`reports/iterator-buffered-metrics.json`）、`windows_per_sec = 1.89e6` / `delta_pct = +0.038`（`reports/benchmarks/iter_buffered-2027-02-22.json`）。`StageRequirement = Exact("experimental")` を満たし、`iterator.stage.audit_pass_rate = 1.0` を維持。
- Snapshots: `compiler/rust/frontend/tests/snapshots/core_iter_adapters__core_iter_adapters.snap`（`buffered_window`） / `reports/spec-audit/ch1/iterator.buffered.diagnostics.json`。
- 連携資料: `reports/spec-audit/ch0/links.md#iter-buffered`, `reports/iterator-buffered-metrics.json`, `reports/benchmarks/iter_buffered-2027-02-22.json`, `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` §4.a, `docs/notes/core-library-outline.md#iter-g3-buffered-backpressure`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#iterator-adapter-esc`.
