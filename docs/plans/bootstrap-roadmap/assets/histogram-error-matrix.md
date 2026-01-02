# Histogram エラーマトリクス

`Core.Numeric.histogram`（`docs/spec/3-4-core-numeric-time.md` §2）で検証すべき入力条件と、それぞれが `StatisticsError`/`Diagnostic` にどう反映されるかを整理する。`Core.Config`/`Core.Data` 章（`docs/spec/3-7-core-config-data.md` §4.8）では `ColumnStats.histogram` の重複検証と `Diagnostic.code = "data.stats.invalid_bucket"` を要求しているため、本マトリクスで Runtime 実装と仕様参照を一元管理する。

| ID | 検証内容 | StatisticsErrorKind | 推奨 Diagnostic.code | 参照 |
| --- | --- | --- | --- | --- |
| H-01 | `buckets` が空の場合は統計処理を継続できないため即時エラーにする | InvalidParameter | `data.stats.invalid_bucket` | 3-4 §2 `histogram`、3-7 §columnStats.count |
| H-02 | 各 `HistogramBucket` は `min`/`max` を必須とし、`min < max`（かつ双方が有限値）でなければならない | InvalidParameter | `data.stats.invalid_bucket` | 3-4 §2 `HistogramBucket` 定義、3-7 §columnStats.histogram |
| H-03 | バケット集合は `min` 昇順で重複しないこと（`prev.max <= next.min` を満たさない場合は重複扱い） | InvalidParameter | `data.stats.invalid_bucket` | 3-7 §columnStats.histogram (`重複バケット` の注記) |
| H-04 | ラベル (`bucket.label`) または `(min, max)` のタプルが完全一致するバケットは重複とみなし拒否する | InvalidParameter | `data.stats.invalid_bucket` | 3-7 §columnStats.histogram (`Diagnostic.code = "data.stats.invalid_bucket"`) |
| H-05 | 値の分類中に `NaN` / 無限大を検出した場合は計算を停止し `NumericalInstability` を返す | NumericalInstability | `core.numeric.statistics.numerical_instability` | 3-4 §2 (`数値的不安定性` の注記) |
| H-06 | いずれのバケットにも含まれない値が存在する場合は `InvalidParameter`（対象列の事前スキャン不足）として報告する | InvalidParameter | `core.numeric.statistics.out_of_range` | 3-4 §2 `histogram`, 3-7 §4.8 `update_stats` |
| H-07 | 入力が 1 件未満の場合は正規化できないため `InsufficientData` を返し、`mean`/`percentile` 計算と整合させる | InsufficientData | `core.numeric.statistics.insufficient_data` | 3-4 §2 `StatisticsErrorKind` |

## 運用メモ

- `InvalidParameter` 系の診断は `Diagnostic.extensions["numeric.histogram"]` に `bucket_index` / `bucket_label` / `violated_rule` を記録する。3-7 §4.8 `update_stats` → `Diagnostic.code` の要件を満たすため、コードは `data.stats.invalid_bucket` を優先し、個別エラーコード（`core.numeric.statistics.*`）は `extensions["numeric.error_code"]` で補完する。
- `NumericalInstability` は `value`, `bucket_index`, `source = "histogram"` を `metadata.numeric.*` に出力し、`docs/notes/runtime/core-numeric-time-gap-log.md` で再現データを追跡する。
- `Out of range` の扱いは `ColumnStats` 側の前処理（`min`/`max` 推定）とリンクするため、`docs/spec/3-7-core-config-data.md` §4.8 の `update_stats` 実装メモにも同じ ID を追記する。
