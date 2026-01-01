# docs/guides 目次

Reml の実装・運用に関するガイドをカテゴリ別に整理しています。

## 開発ワークフロー & ツールチェーン
- [tooling/cli-workflow.md](tooling/cli-workflow.md)
- [tooling/cli-help-template.md](tooling/cli-help-template.md)
- [tooling/cli-authoring.md](tooling/cli-authoring.md)
- [man/remlc-ocaml.1.md](man/remlc-ocaml.1.md)
- [tooling/testing.md](tooling/testing.md)
- [tooling/trace-output.md](tooling/trace-output.md)
- [tooling/diagnostic-format.md](tooling/diagnostic-format.md)
- [tooling/config-cli.md](tooling/config-cli.md)
- [tooling/ci-strategy.md](tooling/ci-strategy.md)

## LSP 連携
- [lsp/lsp-integration.md](lsp/lsp-integration.md)
- [lsp/lsp-authoring.md](lsp/lsp-authoring.md)

## コンパイラ / 解析
- [compiler/core-parse-streaming.md](compiler/core-parse-streaming.md)
- [compiler/llvm-integration-notes.md](compiler/llvm-integration-notes.md)

## DSL / プラグイン
- [dsl/DSL-plugin.md](dsl/DSL-plugin.md)
- [dsl/plugin-authoring.md](dsl/plugin-authoring.md)
- [dsl/dsl-first-guide.md](dsl/dsl-first-guide.md)
- [dsl/dsl-gallery.md](dsl/dsl-gallery.md)
- [dsl/dsl-performance-playbook.md](dsl/dsl-performance-playbook.md)
- [dsl/conductor-pattern.md](dsl/conductor-pattern.md)
- [dsl/constraint-dsl-best-practices.md](dsl/constraint-dsl-best-practices.md)
- [dsl/formatter-authoring.md](dsl/formatter-authoring.md)
- [dsl/doc-authoring.md](dsl/doc-authoring.md)

## エコシステム & コミュニティ
- [ecosystem/ai-integration.md](ecosystem/ai-integration.md)
- [ecosystem/manifest-authoring.md](ecosystem/manifest-authoring.md)
- [ecosystem/package-management.md](ecosystem/package-management.md)
- [ecosystem/community-handbook.md](ecosystem/community-handbook.md)
- [ecosystem/early-design-concepts.md](ecosystem/early-design-concepts.md)
- [ecosystem/collection-pipeline-guide.md](ecosystem/collection-pipeline-guide.md)
- [ecosystem/data-model-reference.md](ecosystem/data-model-reference.md)

## ランタイム / システム連携
- [runtime/runtime-bridges.md](runtime/runtime-bridges.md)
- [runtime/system-programming-primer.md](runtime/system-programming-primer.md)
- [runtime/cross-compilation.md](runtime/cross-compilation.md)
- [runtime/portability.md](runtime/portability.md)

## FFI / 低レベル
- [ffi/reml-ffi-handbook.md](ffi/reml-ffi-handbook.md)
- [ffi/reml-bindgen-guide.md](ffi/reml-bindgen-guide.md)
- [ffi/ffi-dsl-guide.md](ffi/ffi-dsl-guide.md)
- [ffi/ffi-build-integration-guide.md](ffi/ffi-build-integration-guide.md)
- [ffi/ffi-wit-poc.md](ffi/ffi-wit-poc.md)
- [ffi/core-unsafe-ptr-api-draft.md](ffi/core-unsafe-ptr-api-draft.md)

---
各ガイドは `docs/spec/` の対応する節と整合するよう維持してください。更新時は `docs/README.md` とルート README のリンクも合わせて調整します。
