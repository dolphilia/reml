# examples ディレクトリ

Reml 仕様で紹介されるサンプル実装を集約しています。元々の `samples/` 配下のコンテンツを移設し、仕様・計画書・ガイドから参照しやすいよう整理しました。

## サブディレクトリ
- `algebraic-effects/`: 代数的効果の言語断片と検証用サンプル
- `core-collections/`: `Core.Collections` による永続/可変コレクションの利用例
- `core-text/`: `Core.Text`/`Core.Unicode` の三層モデルと Grapheme/Streaming decode の統合サンプル
- `core_config/`: `Core.Config` マニフェストと `@dsl_export` 署名を同期するサンプル（実務ケースは `practical/core_config/` へ移設中）
- `core_io/`: `Core.IO` と `Core.Path` の Reader/Writer・監査・セキュリティサンプル（実務ケースは `practical/core_io/` に統合）
- `core_path/`: パス正規化と `SecurityPolicy`/`is_safe_symlink` の利用例（実務ケースは `practical/core_path/` に統合）
- `core_diagnostics/`: `Core.Diagnostics` の監査ログ（PipelineStarted/PipelineCompleted）を再現する最小ケース
- `dsl_paradigm/`: `Core.Dsl.*` パラダイムキットの参照 DSL（Mini-Ruby / Mini-Erlang / Mini-VM）
- `language-impl-samples/`: Reml の小規模言語実装サンプルと改善調査資料
- `native/`: `Core.Native` の intrinsic/埋め込み API サンプル
- `spec_core/`: Chapter 1 BNF に沿った最小 `.reml` 入力（Phase 4 spec-core スイート）
- `practical/`: Chapter 3 の実務シナリオと監査ログを `scenario_id` 単位で整理した Phase 4 practical スイート

各サブディレクトリにはこれまで通り個別の `README.md` や補足ドキュメントが含まれます。仕様やガイドからサンプルを参照する際は `../examples/...` 形式のパスを使用してください。

## TODO
- [ ] サンプルごとのビルド・実行手順を整備し、必要に応じて自動検証スクリプトを追加
- [ ] 新規サンプル追加時は `docs/README.md` の目次と相互リンクを更新
