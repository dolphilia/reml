# Core.Text 既知の課題

## 目的
ユーザー影響がある Unicode 関連の課題を一覧化し、リリースノートやサポート回答で参照できるようにする。

## 課題一覧
| ID | 概要 | 影響バージョン | 対応状況 | 回避策 / メモ | 関連資料 |
| --- | --- | --- | --- | --- | --- |
| TUI-001 | tr-TR ロケールでのケース変換が `UnsupportedLocale` になる | 未実装 | Pending | `LocaleId` を `und` に設定し、事前に正規化する | text-case-width-gap.md | 
| TUI-002 | Streaming decode で 10MB 超の入力が遅延する | Rust frontend (WIP) | Investigating | `decode_stream` のチャンクサイズを 64KB 以下に調整 | text-unicode-performance-investigation.md | 
| TUI-003 | Grapheme キャッシュのメトリクス未出力 | Rust frontend (WIP) | Resolved | `log_grapheme_stats` が `cache_hits/cache_miss` を出力するようになり、`tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats` で検証可能。 | unicode-cache-cases.md, reports/text-grapheme-metrics.json |

## 更新ルール
- 新規課題を追加したら `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にリスクを登録する。
- 対応完了後は `Resolved` とし、コミット ID やリリースタグを備考に記載する。
