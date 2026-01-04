# Unicode キャッシュ検証ケース

## 目的
`GraphemeSeq` など内部キャッシュを持つ API の整合性・性能を確認し、`log_grapheme_stats` に記録するメトリクス（`cache_hits`/`cache_miss`）を管理する。

## ケース一覧
| Case ID | 入力規模 / ロケール | キャッシュ前提 | 期待されるメトリクス | 再現手順 / 資産 | 状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| UC-01 | 5MB、単一ロケール (ja-JP) | `IndexCache` 初回生成 | `cache_miss >= 1`, `cache_hits = 0`, `version_mismatch_evictions = 0` | `cargo test --manifest-path compiler/runtime/Cargo.toml UC_01` | Green (2024-04-15) | 大規模入力で GC 圧を計測し、`reports/spec-audit/ch1/core_text_grapheme_stats.json` の `cache_miss_log` を確認。 |
| UC-02 | 500KB、混在ロケール (ja + ar + emoji) | 再利用 (`GraphemeSeq::clone`) | `cache_hits / (hits+miss) >= 0.7`, `avg_generation = 1` | `cargo test --manifest-path compiler/runtime/Cargo.toml UC_02` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats --text-source reports/spec-audit/ch1/core_text_grapheme_stats.json --require-success --check script_mix` | Green (2024-04-15) | Clone 後の書記素分割を確認し、`text.grapheme.cache_hit` KPI を更新。 |
| UC-03 | 200KB、Streaming decode | `TextBuilder` → `GraphemeSeq::from_builder` | `cache_miss == 0` （builder 共有）、`collector.effect.text_cache_hits > 0` | `cargo test --manifest-path compiler/runtime/Cargo.toml UC_03` | Green (2024-04-15) | `effect {audit}` ログ (`text.grapheme_stats`) と整合させる。 |

## メモ
- 結果は `reports/spec-audit/ch1/core_text_grapheme_stats.json` と `docs/notes/text/text-unicode-performance-investigation.md` に転記する。`cache_hits`, `cache_miss`, `version_mismatch_evictions`, `avg_generation` を最低限保存する。
- キャッシュ仕様が変わった場合はこのファイルのケース表を更新し、`docs/notes/text/text-unicode-ownership.md` および `docs/notes/stdlib/core-library-outline.md#runtimecachespeccoretext-キャッシュモデル` へ参照を追加する。
- `log_grapheme_stats` に `cache_hits`/`cache_miss` が未出力の場合は `docs/notes/text/text-unicode-known-issues.md` の `TUI-003` を更新し、`phase3-core-text` CI で回転を止める。
