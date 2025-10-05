# 3.8 ドキュメント・仕様フィードバック計画

## 目的
- Phase 3 で Reml 実装特有の仕様変更・知見を言語仕様書へ反映し、セルフホスト化後のドキュメント整合を保つ。
- クロスコンパイル機能やランタイム更新で判明したギャップを `notes/llvm-spec-status-survey.md`・`notes/dsl-plugin-roadmap.md` 等へ連携し、Phase 4 の移行計画へ橋渡しする。

## スコープ
- **含む**: 実装差分の記録、仕様書更新提案、脚注/TODO の追加、関連ノートの更新、レビュー連携。
- **含まない**: 新規仕様章の全面改稿。必要な場合は別タスクを起票。
- **前提**: Phase 3 の各実装タスクからフィードバックが集まる仕組みがあり、差分が明確化していること。

## 作業ブレークダウン
1. **差分収集**: Parser/TypeChecker/ランタイム/クロスコンパイルからの差分リストを収集し、優先度を分類。
2. **仕様反映案作成**: 各差分について修正案を草稿化し、関連ファイル (Chapter 1〜3, `3-8`, `3-10` 等) へマッピング。
3. **レビュー運用**: 仕様更新のレビュー手順を整理し、レビュアアサイン・スケジュールを `0-3-audit-and-metrics.md` に記載。
4. **ノート更新**: ギャップや将来課題を `notes/llvm-spec-status-survey.md`、`notes/dsl-plugin-roadmap.md` などへ追記。
5. **索引更新**: `README.md` と `0-0-overview.md` のリンクを最新化。
6. **フォローアップ TODO**: Phase 4 へ持ち越す事項を `0-4-risk-handling.md` または新規 TODO ノートに記録。

## 成果物と検証
- 仕様更新の差分がレビューを通過し、履歴が残る。
- ノート類が最新状態になり、フェーズ間の引き継ぎが明瞭。
- 索引のリンクが正しく更新され、リンク切れが無い。

## リスクとフォローアップ
- レビュー負荷が高い場合は更新を段階的に分割し、最重要項目から着手。
- 大きな仕様変更が必要な場合は Phase 4 のロードマップに組み込み、読者へ影響範囲を通知。
- ノートの散逸を防ぐため、更新履歴を `0-3-audit-and-metrics.md` で管理。

## 参考資料
- [3-0-phase3-self-host.md](3-0-phase3-self-host.md)
- [0-0-overview.md](../../0-0-overview.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)
- [notes/dsl-plugin-roadmap.md](../../notes/dsl-plugin-roadmap.md)

