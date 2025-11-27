# Phase3 Core.Text ベンチマーク基準値

`benchmarks/` クレートの `cargo bench text::*` で収集する 3 つのシナリオ（正規化・グラフェム分割・TextBuilder）をまとめた表。`最新値` は直近で計測した `criterion` の平均（`estimates.mean.point_estimate`）を記録し、`目標` 列には Phase 2 ベンチ比 ±15% の枠（暫定目安）を記す。

| シナリオ | メトリクス | 最新値 | 目標/閾値 | 備考 |
| --- | --- | --- | --- | --- |
| 正規化 (NFC/NFD/NFKC/NFKD) | `MB/s` | 未計測（TODO） | Phase2 平均 `>= 2.8 MB/s`、単一フォーム ±15% | `benchmarks/text/normalization.rs`、`NormalizationTest-15.1.0` を入力に使用。 |
| グラフェム分割 (segment/log stats) | `ns/char`、`cache hit %` | 未計測（TODO） | `segment_cold` ≤ Phase2 ×1.15、`segment_cached` ≥ Phase2 ×0.85 | `benchmarks/text/grapheme.rs`、`clear_grapheme_cache_for_tests` を含む。 |
| TextBuilder (push_* + finish) | `MB/s`、`effect.mem_bytes` | 未計測（TODO） | `push_str_finish` ≥ Phase2 ×0.85、`push_grapheme_finish` ≥ Phase2 ×0.85 | `benchmarks/text/builder.rs`、`TextBuilder::with_capacity(len)` を利用。 |

## 記録フォーマット
- `criterion` の `report/new/raw.csv` から `mean`, `std`, `throughput` を抽出し、小数第 2 位で丸めて記録する。
- `cache hit %` は `text::grapheme/segment_cached` のスループットに加えて `reports/spec-audit/ch1/core_text_grapheme_stats.json` の `cache_hits`/`cache_miss` を参照する。
- `effect.mem_bytes` は `TextBuilder` ベンチ後に `reml_runtime::text::take_text_effects_snapshot()` を呼び、`collector.effect.mem_bytes` をログへ追記する（将来の自動化対象）。
- 目標外の値を検出した場合、`docs/notes/text-unicode-performance-investigation.md` に追記し、本ファイルからリンクする。
