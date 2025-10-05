# 4.3 ドキュメント更新計画

## 目的
- Phase 4 マイルストーン M3 を支援するため、`README.md`・`0-0-overview.md`・`guides/llvm-integration-notes.md` など主要ドキュメントをセルフホスト版前提に更新する。
- 正式ターゲット (Linux > Windows > macOS) と配布経路を明示し、コミュニティへ周知する。

## スコープ
- **含む**: ドキュメント更新、索引リンク調整、リリースノートとの整合、周知計画。
- **含まない**: 外部サイト更新、翻訳。必要に応じて別計画。
- **前提**: リリースパイプラインが整備され、セルフホスト成果物が利用可能。

## 作業ブレークダウン
1. **更新対象洗い出し**: `README.md`、`0-0-overview.md`、`guides/llvm-integration-notes.md`、関連ガイド (`guides/plugin-authoring.md` 等) の修正箇所をリスト化。
2. **セルフホスト前提化**: ドキュメント上の「OCaml 実装」表記を移行後の扱い（LTS）に更新し、リンクをセルフホスト版へ切り替える。
3. **ターゲット記載**: 正式サポートターゲットを明記し、優先順位 (Linux > Windows > macOS) を記述。
4. **ダウンロード案内**: 新しい配布ページ・署名検証手順を記載。
5. **コミュニティ告知**: 変更内容をまとめ、`notes/` やコミュニティ投稿案を草案化。
6. **レビューと公開**: レビュー記録を残し、最終公開日・責任者を `0-3-audit-and-metrics.md` に記録。

## 成果物と検証
- 更新されたドキュメントがレビューを通過し、リンク切れが無い。
- セルフホスト版の利用手順が明確で、OS 別の注意事項が整理されている。
- サマリをコミュニティへ通知する準備が整う。

## リスクとフォローアップ
- ドキュメント量が多いため、優先順位を設定し分割してレビュー。
- コミュニティ周知が遅れると移行が滞る可能性があるため、告知計画を早期に準備。
- 翻訳が必要な場合に備え、用語統一を `0-2-project-purpose.md` と同期。

## 参考資料
- [4-0-phase4-migration.md](4-0-phase4-migration.md)
- [0-0-overview.md](../../0-0-overview.md)
- [README.md](../../README.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)

