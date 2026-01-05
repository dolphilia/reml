# docs/guides 目次

Reml の実装・運用に関するガイドをカテゴリ別に整理しています。

## 開発ワークフロー & ツールチェーン
- [tooling/cli-authoring.md](tooling/cli-authoring.md) — `Core.Cli` で宣言的に CLI を構築する最小ガイド。
- [tooling/testing.md](tooling/testing.md) — `Core.Test` によるスナップショット/ゴールデンテスト運用。
- [tooling/benchmarks.md](tooling/benchmarks.md) — ベンチマークの実行手順と運用上の注意点。
- [tooling/diagnostic-format.md](tooling/diagnostic-format.md) — 診断出力のテキスト/JSON 形式仕様。
- [tooling/config-cli.md](tooling/config-cli.md) — `Core.Config` を CLI から検証・差分・レンダリングする手順。
- [tooling/ci-strategy.md](tooling/ci-strategy.md) — マルチターゲット CI/テスト戦略と診断ポリシーの整合。

## LSP 連携
- [lsp/lsp-integration.md](lsp/lsp-integration.md) — Reml 生成情報を LSP 経由で IDE に連携する実装指針。
- [lsp/lsp-authoring.md](lsp/lsp-authoring.md) — `Core.Lsp` を使った LSP サーバー実装の最小ガイド。

## コンパイラ / 解析
- [compiler/core-parse-streaming.md](compiler/core-parse-streaming.md) — ストリーミング解析の運用パターンと補足ノート。
- [compiler/llvm-integration-notes.md](compiler/llvm-integration-notes.md) — Reml→LLVM 連携の段階的実装メモ。

## DSL / プラグイン
- [dsl/DSL-plugin.md](dsl/DSL-plugin.md) — `Core.Parse.Plugin` の設計手順と Capability 運用。
- [dsl/plugin-authoring.md](dsl/plugin-authoring.md) — プラグイン開発/配布の基本手順と注意点。
- [dsl/dsl-first-guide.md](dsl/dsl-first-guide.md) — DSL ファースト戦略の導入ステップとチェックリスト。
- [dsl/dsl-gallery.md](dsl/dsl-gallery.md) — DSL サンプル収集・公開の運用ガイド。
- [dsl/dsl-performance-playbook.md](dsl/dsl-performance-playbook.md) — DSL の性能計測/最適化プレイブック。
- [dsl/conductor-pattern.md](dsl/conductor-pattern.md) — 複数 DSL を協調実行する Conductor パターン。
- [dsl/constraint-dsl-best-practices.md](dsl/constraint-dsl-best-practices.md) — 制約/ポリシー DSL の設計ベストプラクティス。
- [dsl/formatter-authoring.md](dsl/formatter-authoring.md) — `Core.Text.Pretty` によるフォーマッタ実装ガイド。
- [dsl/doc-authoring.md](dsl/doc-authoring.md) — `Core.Doc` を使ったドキュメント生成の最小ガイド。

## エコシステム & コミュニティ
- [ecosystem/ai-integration.md](ecosystem/ai-integration.md) — AI 支援機能の安全な統合と運用指針。
- [ecosystem/manifest-authoring.md](ecosystem/manifest-authoring.md) — `reml.toml` の記述/検証/運用ベストプラクティス。
- [ecosystem/package-management.md](ecosystem/package-management.md) — パッケージ管理ツール設計のドラフトメモ。
- [ecosystem/community-handbook.md](ecosystem/community-handbook.md) — コミュニティ運営の体制・ルール指針。
- [ecosystem/early-design-concepts.md](ecosystem/early-design-concepts.md) — パーサーコンビネーター指向の言語設計初期ノート。
- [ecosystem/collection-pipeline-guide.md](ecosystem/collection-pipeline-guide.md) — 収集パイプライン構築の実装ガイド。
- [ecosystem/data-model-reference.md](ecosystem/data-model-reference.md) — `Core.Data`/`Nest.Data` の運用と監査手順。

## ランタイム / システム連携
- [runtime/runtime-bridges.md](runtime/runtime-bridges.md) — ランタイムブリッジの契約/運用と監査の指針。
- [runtime/system-programming-primer.md](runtime/system-programming-primer.md) — システム系プラグインの導入と全体像。
- [runtime/cross-compilation.md](runtime/cross-compilation.md) — クロスコンパイルの CLI/ツールチェーン運用手順。
- [runtime/portability.md](runtime/portability.md) — マルチプラットフォーム運用の手順と注意点。

## FFI / 低レベル
- [ffi/reml-ffi-handbook.md](ffi/reml-ffi-handbook.md) — FFI の全体像と安全運用をまとめたハンドブック。
- [ffi/reml-bindgen-guide.md](ffi/reml-bindgen-guide.md) — C/C++ ヘッダからのバインディング生成手順。
- [ffi/ffi-dsl-guide.md](ffi/ffi-dsl-guide.md) — `Core.Ffi.Dsl` による安全な FFI 利用法。
- [ffi/ffi-build-integration-guide.md](ffi/ffi-build-integration-guide.md) — FFI 生成とビルド統合の運用ガイド。
- [ffi/ffi-wit-poc.md](ffi/ffi-wit-poc.md) — WIT/Component Model 連携 PoC の最小手順。
- [ffi/core-unsafe-ptr-api-draft.md](ffi/core-unsafe-ptr-api-draft.md) — `Core.Unsafe.Ptr` の安全な利用とチェックリスト。

---
各ガイドは `docs/spec/` の対応する節と整合するよう維持してください。更新時は `docs/README.md` とルート README のリンクも合わせて調整します。
