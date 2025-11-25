# Core.Text 既知の課題

## 目的
ユーザー影響がある Unicode 関連の課題を一覧化し、リリースノートやサポート回答で参照できるようにする。

## 課題一覧
| ID | 概要 | 影響バージョン | 対応状況 | 回避策 / メモ | 関連資料 |
| --- | --- | --- | --- | --- | --- |
| TUI-001 | tr-TR ロケールでのケース変換が `UnsupportedLocale` になる | 未実装 | Pending | `LocaleId` を `und` に設定し、事前に正規化する | text-case-width-gap.md | 
| TUI-002 | Streaming decode で 10MB 超の入力が遅延する | Rust frontend (WIP) | Investigating | `decode_stream` のチャンクサイズを 64KB 以下に調整 | text-unicode-performance-investigation.md | 
| TUI-003 | Grapheme キャッシュのメトリクス未出力 | Rust frontend (WIP) | Planned | `log_grapheme_stats` を有効化して再実行 | unicode-cache-cases.md | 

## 更新ルール
- 新規課題を追加したら `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にリスクを登録する。
- 対応完了後は `Resolved` とし、コミット ID やリリースタグを備考に記載する。
