# Core.Text 既知の課題

## 目的
ユーザー影響がある Unicode 関連の課題を一覧化し、リリースノートやサポート回答で参照できるようにする。

## 課題一覧
| ID | 概要 | 影響バージョン | 対応状況 | 回避策 / メモ | 関連資料 |
| --- | --- | --- | --- | --- | --- |
| TUI-001 | tr-TR ロケールでのケース変換が `UnsupportedLocale` になる | Rust runtime ≤20270328 | Resolved | `compiler/runtime/src/text/locale.rs` で `LocaleId::parse("tr-TR")` をサポートし、`docs/notes/text/text-case-width-gap.md` `Closed`。CLI/LSP は `LocaleScope::Case` を指定して再実行する。 | text-case-width-gap.md, reports/spec-audit/ch1/unicode_case_width.rs |
| TUI-002 | Streaming decode で 10MB 超の入力が遅延する | Rust frontend (WIP) | Investigating | `decode_stream` のチャンクサイズを 64KB 以下に調整し、`examples/core-text/expected/text_unicode.stream_decode.golden` で BOM/invalid ポリシーを固定する。 | text-unicode-performance-investigation.md, core_text_examples-20270330.md | 
| TUI-003 | Grapheme キャッシュのメトリクス未出力 | Rust frontend (WIP) | Resolved | `log_grapheme_stats` が `cache_hits/cache_miss` を出力するようになり、`tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats` で検証可能。 | unicode-cache-cases.md, reports/text-grapheme-metrics.json |
| TUI-004 | Unicode データ差分により CLI/LSP と AI 入力の正規化結果がずれる | Unicode データ更新時 | Monitoring | `examples/core-text/expected/text_unicode.*.golden` を Unicode 更新ごとに再生成し、`reports/spec-audit/ch1/core_text_examples-YYYYMMDD.md` でログを残す。逸脱時は `R-041 Unicode Data Drift` へリンク。 | docs/plans/bootstrap-roadmap/0-4-risk-handling.md, examples/core-text/README.md |

## 更新ルール
- 新規課題を追加したら `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にリスクを登録する。
- 対応完了後は `Resolved` とし、コミット ID やリリースタグを備考に記載する。
