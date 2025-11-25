# Text & Unicode 性能調査メモ

## 目的
正規化・書記素分割・TextBuilder などホットパスの性能を計測し、`0-3-audit-and-metrics.md` の KPI 達成度を追跡する。

## 計測項目
| ID | ワークロード | 目標値 | 実測値 (最新) | 計測手段 | 備考 |
| --- | --- | --- | --- | --- | --- |
| PERF-01 | NFC on 10MB UTF-8 | 150 MB/s 以上 | - | `cargo bench text::normalization` | `NormalizationTest` 使用 |
| PERF-02 | Grapheme segmentation 10MB | 110 MB/s 以上 | - | `cargo bench text::grapheme` | キャッシュ命中率 70% を確認 |
| PERF-03 | TextBuilder streaming append | 50 MB/s 以上 | - | `cargo bench text::builder --quick` | `effect {mem}` を記録 |

## 分析ログ
- 2025-11-21: 指標テンプレ作成。`reports/benchmarks/core_text/` に結果を保存予定。

## TODO
- [ ] OCaml 実装との比較値を追加し、±15% の差分目標を設定。
- [ ] SIMD 最適化（AVX2/WASM）候補を列挙し、Phase 4 移行計画と同期。
