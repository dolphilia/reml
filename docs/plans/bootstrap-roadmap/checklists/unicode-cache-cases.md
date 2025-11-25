# Unicode キャッシュ検証ケース

## 目的
`GraphemeSeq` など内部キャッシュを持つ API の整合性・性能を確認し、`log_grapheme_stats` に記録するメトリクス（`cache_hits`/`cache_miss`）を管理する。

## ケース一覧
| Case ID | 入力規模 / ロケール | キャッシュ前提 | 期待されるメトリクス | 再現手順 / 資産 | 状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| UC-01 | 5MB、単一ロケール (ja-JP) | `IndexCache` 初回生成 | `cache_miss >= 1`, `cache_hits = 0` | `cargo test text_internal_cache -- --ignored UC_01` | Pending | 大規模入力で GC 圧を計測 |
| UC-02 | 500KB、混在ロケール (ja + ar + emoji) | 再利用 (`GraphemeSeq::clone`) | `cache_hits / (hits+miss) >= 0.7` | `tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats` | Pending | Clone 後の書記素分割を確認 |
| UC-03 | 200KB、Streaming decode | `TextBuilder` → `GraphemeSeq::from_builder` | `cache_miss == 0` （builder 共有） | `scripts/ci/run_core_text_regressions.sh --case streaming` | Pending | `effect {audit}` ログと整合させる |

## メモ
- 結果は `reports/spec-audit/ch1/core_text_grapheme_stats.json` と `docs/notes/text-unicode-performance-investigation.md` に転記する。
- キャッシュ仕様が変わった場合はこのファイルのケース表を更新し、`docs/notes/text-unicode-ownership.md` へ参照を追加する。
