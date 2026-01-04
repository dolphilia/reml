# tooling ディレクトリ構成（準備中）

開発者体験、CI/検証、リリースパイプライン、LSP 連携など、周辺ツール資産を集約する領域です。各サブディレクトリは対応するブートストラップ計画書と直接リンクします。

## サブディレクトリ
- `cli/`: Phase 1 `1-6-developer-experience.md` に紐付く CLI 実装・ドキュメント
- `ci/`: Phase 1 `1-7-linux-validation-infra.md` などで定義される CI スクリプトとローカル再現ツール
- `release/`: Phase 4 `4-2-multitarget-release-pipeline.md` に基づく署名・配布スクリプト
- `lsp/`: Phase 2 以降の LSP/IDE 補助資産（`3-6-core-diagnostics-audit-plan.md` など）

## TODO
- [ ] `tooling/cli/`・`tooling/ci/`・`tooling/release/`・`tooling/lsp/` の README を Phase 1〜4 計画に合わせて拡充
- [ ] `.github/workflows/` との責務境界と参照方法を整理
- [ ] 共通ユーティリティ（ロギング、テンプレート生成など）が必要になった際の配置ポリシーを決定
