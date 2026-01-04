# Core.Numeric 数値安定性メモ

Core.Numeric/Time 章の計画 (`docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §2.3) に従い、統計ユーティリティで採用する数値安定化手法と再現手順をまとめる。仕様 `docs/spec/3-4-core-numeric-time.md` §2 が要求する「数値的不安定性 (`StatisticsErrorKind::NumericalInstability`) を自動検出する」条件を満たすための根拠資料として扱う。

## 1. 採用アルゴリズムと理由

| 項目 | 採用手法 | 理由 | 参照 |
| --- | --- | --- | --- |
| 累積和 (`IterNumericExt` に今後追加する `sum`/`rolling_average` 等) | Kahan summation（改良型、PoC 設計済み） | 大小混在データ（1e-9 ～ 1e12）の和を 1ULP 以内で保持し、`f64` 基本演算のみで実装できる。`Iter::try_fold`＋補償加算でメモリ確保を発生させない。 | `compiler/runtime/src/numeric/mod.rs`（Kahan ガード導入 TODO） |
| 平均・分散 | Welford 法 | 1 パスで平均・二乗偏差を同時更新でき、`NaN` の早期検出が容易。標本数 1 の場合に `None` を返す仕様とも整合。 | 同上 |
| 回帰・比率推定（今後の `linear_regression` 実装） | Horvitz-Thompson 推定量 | 標本重み付けに線形性があり、欠損率の高いデータでもバイアスの少ない推定値を得られる。必要な再重みは `Iter` の遅延処理と相性が良い。 | 今後追加予定 (`docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §3) |

- `Numeric`/`OrderedFloat` は `Iter` を 1 パスで処理し、`EffectSet` へ `effect {mem}` を追加しないことを優先した。
- `mean`/`variance` は更新量が非有限 (`!is_finite()`) になった時点でループを打ち切り、`StatisticsErrorKind::NumericalInstability` を返すガード（`Histogram` と共通）を別途導入予定。

## 2. 再現ベンチマーク

- ベンチファイル: `compiler/runtime/benches/bench_numeric_statistics.rs`
- フレームワーク: `criterion` (`cargo bench --manifest-path compiler/runtime/Cargo.toml --features core-numeric --bench bench_numeric_statistics -- --noplot`)
- 計測シナリオ:
  - `mean_large_drift`: 小数と 1e12 オーダーを混在させた 100k 件の平均（Kahan + Welford）。
  - `variance_random_walk`: 擬似 random walk に対する Welford 法。
  - `percentile_heavy_tail`: ヘビーテール分布で 0.5/0.95 百分位を評価。
- 最新結果は `reports/benchmarks/numeric-phase3/phase3-baseline-2025-12-04.json` に保存し、`docs/plans/rust-migration/3-2-benchmark-baseline.md` のスイート表へ追記済み。

## 3. NumericalInstability 診断サンプル

- 入力ファイル: `tests/data/numeric/instability/histogram_non_finite.json`
- 再現手順: 非有限値 (`NaN`) を含むヒストグラム入力を `histogram` に渡すと、`StatisticsErrorKind::NumericalInstability` が `GuardDiagnostic` へ変換され、`numeric.statistics.kind = "numerical_instability"`/`rule = "H-05"` を記録する。
- ギャップログ: `docs/notes/runtime/core-numeric-time-gap-log.md` 2025-12-04 行に診断サンプルとベンチファイルのリンクを登録した。

## 4. 今後の TODO

- `mean`/`variance`/`percentile` に数値安定性ガードを組み込み、`StatisticsError` を返す API へ拡張する。
- `collect-iterator-audit-metrics.py --section numeric_time --scenario statistics_stability` を追加し、`reports/benchmarks/numeric-phase3/*.json` の中央値が ±15% を超えた場合に CI を失敗させる。
- `linear_regression` 実装に Horvitz-Thompson 推定を適用し、ベンチ/診断双方のサンプルを追加する。
