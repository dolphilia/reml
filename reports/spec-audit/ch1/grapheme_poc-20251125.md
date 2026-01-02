# Grapheme 分割 PoC (unicode-segmentation + unicode-width)

- **実施日**: 2025-11-25
- **目的**: docs/notes/text/text-unicode-segmentation-comparison.md で推奨した候補 A（`unicode-segmentation` + `unicode-width`）を runtime に取り込み、`segment_graphemes` と表示幅計測の観測ポイントを確認する。
- **実装箇所**: `compiler/rust/runtime/src/text/grapheme.rs`（新規）、`compiler/rust/runtime/Cargo.toml`（依存登録）

## 入力ケース
```
"a🇯🇵👨‍👩‍👧‍👦 café"
```

| # | Grapheme | Bytes | `unicode_width` | Emoji判定 |
|---|----------|-------|-----------------|-----------|
| 1 | `a` | 1 | 1 | false |
| 2 | `🇯🇵` | 8 (regional indicator + ZWJ) | 2 | true |
| 3 | `👨‍👩‍👧‍👦` | 25 | 2 | true |
| 4 | ` ` | 1 | 1 | false |
| 5 | `c` | 1 | 1 | false |
| 6 | `a` | 1 | 1 | false |
| 7 | `f` | 1 | 1 | false |
| 8 | `é` | 2 | 1 | false |

## GraphemeStats (PoC)
- `grapheme_count`: 8
- `total_bytes`: 40
- `total_display_width`: 10
- `avg_width`: 1.25
- `emoji_ratio`: 0.25
- `cache_hits`: 8 （1パス分割結果を全てキャッシュ）
- `cache_miss`: 0 （再分割なし）

## 所見
1. `unicode-width` は家族絵文字を幅 2 と判定し、CLI 下線に十分な情報を提供できる。一方で `é`（合成文字）は幅 1 のため、NFC 正規化と組み合わせれば Diagnostics と整合する想定。
2. GraphemeSeq では byte offset を `Vec<usize>` として保持し、今後の `IndexCache`（docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md#1.3）に再利用できる形で保存済み。
3. `Str::iter_graphemes` 相当の Iterator ラッパ、`effect {unicode}` タグ、および `log_grapheme_stats` 出力への配線は今後のタスク。PoC では `GraphemeSeq::stats()` を介して観測できるようにした。

## フォローアップ
1. `unicode-width` の emoji 幅差分を `docs/notes/text/text-case-width-gap.md` へ追記し、Narrow/Wide/Locale モードの要件を整理する。
2. `segment_graphemes` の戻り値を `Result<GraphemeSeq, UnicodeError>` とし、`effect {unicode}` 設計（docs/spec/3-3-core-text-unicode.md §4）と整合させる。
3. `tooling/ci/collect-iterator-audit-metrics.py --section text` 向けの `cache_hits/cache_miss` メトリクスを追加し、UAX #29 conformance テストを `tests/data/unicode/` に取り込む。
