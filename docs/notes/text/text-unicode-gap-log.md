# Text & Unicode ギャップログ

Core.Text/Unicode 仕様と実装の差分、調査結果、フォローアップを記録する。3-3 計画や Phase 3 KPI 更新時に参照する。

## 記入フォーマット
| 日付 | 区分 | 概要 | 影響範囲 | 対応状況 | チケット/リンク |
| --- | --- | --- | --- | --- | --- |

### 例
| 2025-11-20 | API 差分 | `TextBuilder::push_grapheme` が未実装 | `compiler/runtime/src/text/builder.rs` | Pending | docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md#2.3 |

## 最新エントリ
| 日付 | 区分 | 概要 | 影響範囲 | 対応状況 | チケット/リンク |
| --- | --- | --- | --- | --- | --- |
| 2024-04-15 | Grapheme キャッシュ | `IndexCacheGeneration` を導入し `unicode_segmentation::UNICODE_VERSION`/`CACHE_VERSION` の不一致を検出、`version_mismatch_evictions`・`unicode_version` を `text.grapheme_stats` へ出力。`log_grapheme_stats` は `effects::record_audit_event_with_metadata` で `CollectorAuditTrail` に直接埋め込み、`text_internal_cache` UC-01〜03 を常時実行化して `reports/spec-audit/ch1/core_text_grapheme_stats.json` / `text_grapheme_stats.audit.jsonl` を更新。 | `compiler/runtime/src/text/grapheme.rs`, `compiler/frontend/src/diagnostic/unicode.rs`, `reports/spec-audit/ch1/*.json*` | Done | docs/plans/bootstrap-roadmap/3-3-core-text-unicode-gap-remediation.md#c-grapheme-キャッシュと監査パイプライン統合 |
| 2027-04-02 | ストリーミング | `decode_stream`/`encode_stream` をチャンク逐次処理＋ `effect {unicode}` 記録に対応させ、`UnicodeError` へ `IoError` ソースと `phase=io.decode.eof` を伝搬。CLI `text_stream_decode` に `--chunk-size`/`--replace`/`effect` 出力を追加。 | `compiler/runtime/src/io/text_stream.rs`, `compiler/runtime/examples/io/text_stream_decode.rs`, `docs/plans/bootstrap-roadmap/checklists/text-api-error-scenarios.md` | Done | docs/plans/bootstrap-roadmap/3-3-core-text-unicode-gap-remediation.md#b-ストリーミング-decodeencode-再設計 |
| 2025-11-25 | Rust実装欠落 | `compiler/runtime/src/text/` を新設し、`Bytes`/`Str`/`String` ラッパと `UnicodeError` を追加。API は骨格のみで effect/Audit 計測は未実装。 | `compiler/runtime/src/text/*` | Stub | docs/plans/bootstrap-roadmap/assets/text-unicode-api-diff.csv |
| 2025-11-25 | 仕様差分可視化 | Grapheme レイヤ (`GraphemeSeq`, `segment_graphemes`, `width`) が Rust runtime に存在せず、UAX #29 テストを配置できない。`Core.Text` 想定の effect 計測 (`log_grapheme_stats`) も阻害されている。→ `unicode-segmentation + unicode-width` PoC (`compiler/runtime/src/text/grapheme.rs`) と観測ログ（reports/spec-audit/ch1/grapheme_poc-20251125.md）を追加し、`Str::iter_graphemes`/`log_grapheme_stats` の導線も実装済み。 | `docs/spec/3-3-core-text-unicode.md#41-grapheme--word--sentence-境界`, `compiler/runtime/src` | PoC | docs/plans/bootstrap-roadmap/assets/text-unicode-api-diff.csv, docs/notes/text/text-unicode-segmentation-comparison.md, reports/spec-audit/ch1/grapheme_poc-20251125.md |
| 2025-11-25 | Rust実装欠落 | `TextBuilder` 系 API (`builder/append/push_grapheme/reserve/finish`) と `Core.Iter.collect_text` が未着手のため、Phase3 KPI である三層モデル構築ルートが塞がっている。 | `compiler/runtime/src/prelude/collectors/string.rs`, `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md#6` | 未着手 | docs/plans/bootstrap-roadmap/assets/text-unicode-api-diff.csv |
## TODO
- [ ] 既存の差分調査ノート（Phase 2）から Unicode 関連の項目を移設する。
- [ ] エントリごとに `docs/plans/bootstrap-roadmap/assets/text-unicode-api-diff.csv` の該当行をリンクする。
