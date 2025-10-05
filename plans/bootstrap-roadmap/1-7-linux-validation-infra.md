# 1.7 x86_64 Linux 検証インフラ計画

## 目的
- Phase 1 の最終マイルストーン M4 までに、x86_64 Linux (System V ABI) を対象とした自動検証環境を GitHub Actions 上に構築する。
- LLVM 15 以上の固定バージョンに基づく CI パイプラインを整備し、Parser/Typer/Core IR/LLVM/ランタイムのスモークテストを一体化する。

## スコープ
- **含む**: GitHub Actions ワークフロー設計、依存キャッシュ、コンパイル・テスト・リンカ実行、成果物の収集、監査ログへの記録。
- **含まない**: Windows/macOS ランナー、長時間ベンチマーク、本番配布。これらは Phase 2 以降で追加。
- **前提**: CLI と各フェーズのテストがコマンドラインから実行可能になっていること。

## 作業ブレークダウン
1. **CI 設計**: `.github/workflows/bootstrap-linux.yml`（仮）を作成し、Lint → Build → Test → Artifact のステージを定義。
2. **LLVM セットアップ**: `actions/cache` を利用し LLVM 15 バイナリを再利用、バージョンは `0-3-audit-and-metrics.md` と同期。
3. **テスト統合**: Parser/Typer/Core IR/LLVM/ランタイムの各コマンドをジョブ上で実行し、失敗時にログをアップロード。
4. **アーティファクト収集**: 生成した LLVM IR、Core IR ダンプ、バイナリ、診断ログをアーティファクトとして保存し、レビュー時に参照可能にする。
5. **監査ログ更新**: CI 実行結果の要約を `0-3-audit-and-metrics.md` に追記し、失敗時は `0-4-risk-handling.md` へ自動起票するフックを検討。
6. **開発環境ガイド**: ローカルで CI 手順を再現するスクリプト（`scripts/ci-local.sh` 仮）を作成し、README に追記。

## 成果物と検証
- GitHub Actions の定期実行（push/pr）で全テストが通過することを確認。
- アーティファクトが 30 日保持され、レビューで差分確認に利用できる。
- ローカル再現スクリプトにより、開発者が CI と同じ手順を実行可能であることを README へ明記。

## リスクとフォローアップ
- LLVM ダウンロードが CI のボトルネックとなる場合、事前ビルド済み Docker イメージを作成し GitHub Container Registry に登録する。
- CI 実行時間が長くなる可能性があるため、Phase 2 でジョブ分割やキャッシュ戦略の再検討を行う。
- バイナリアーティファクトのサイズが増大した場合、`0-3-audit-and-metrics.md` に上限値を記録し整理する。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)

