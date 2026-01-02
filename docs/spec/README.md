# Reml 公式仕様目次

章番号付き仕様書を章別に整理しています。各章の概要は以下のとおりです。

## 0.x 導入資料・設計指針
- [0-0-overview.md](0-0-overview.md) — Reml の概要と設計ゴール
- [0-1-project-purpose.md](0-1-project-purpose.md) — プロジェクトの目的と判断指針
- [0-2-glossary.md](0-2-glossary.md) — 用語集
- [0-3-code-style-guide.md](0-3-code-style-guide.md) — コードスタイルとサンプル規約

## 1.x 言語コア仕様
- [1-0-language-core-overview.md](1-0-language-core-overview.md)
- [1-1-syntax.md](1-1-syntax.md) — 構文仕様（識別子は Unicode プロファイルを既定とし、`identifier_profile` 切替と効果構文 `-Zalgebraic-effects` の Stage 脚注を掲載）
- [1-2-types-Inference.md](1-2-types-Inference.md) — 効果行を `TArrow` に統合した型システム仕様と `type_row_mode` の互換運用
- [1-3-effects-safety.md](1-3-effects-safety.md) — 残余効果 (`Σ_before`/`Σ_after`) の PoC 計測と `type_row_mode` の運用ポリシー
- [1-4-test-unicode-model.md](1-4-test-unicode-model.md)
- [1-5-formal-grammar-bnf.md](1-5-formal-grammar-bnf.md) — 形式文法（`Ident` は Unicode プロファイルを前提、効果構文 PoC の注記あり）

## 2.x 標準パーサー API
- [2-0-parser-api-overview.md](2-0-parser-api-overview.md)
- [2-1-parser-type.md](2-1-parser-type.md)
- [2-2-core-combinator.md](2-2-core-combinator.md)
- [2-3-lexer.md](2-3-lexer.md) — 字句仕様（`identifier_profile` による `unicode`／`ascii-compat` の切替と監査指標を含む）
- [2-4-op-builder.md](2-4-op-builder.md)
- [2-5-error.md](2-5-error.md)
- [2-6-execution-strategy.md](2-6-execution-strategy.md)
- [2-7-core-parse-streaming.md](2-7-core-parse-streaming.md)

## 3.x 標準ライブラリ
- [3-0-core-library-overview.md](3-0-core-library-overview.md)
- [3-1-core-prelude-iteration.md](3-1-core-prelude-iteration.md)
- [3-2-core-collections.md](3-2-core-collections.md)
- [3-3-core-text-unicode.md](3-3-core-text-unicode.md)
- [3-4-core-numeric-time.md](3-4-core-numeric-time.md)
- [3-5-core-io-path.md](3-5-core-io-path.md)
- [3-6-core-diagnostics-audit.md](3-6-core-diagnostics-audit.md) — `effects.type_row.integration_blocked` 診断と `effect.type_row.*` 監査キーの運用
- [3-7-core-config-data.md](3-7-core-config-data.md)
- [3-8-core-runtime-capability.md](3-8-core-runtime-capability.md) — Stage 管理と `-Z` フラグ運用（効果構文 Capability は Phase 2-5 時点で Experimental と明記）
- [3-9-core-async-ffi-unsafe.md](3-9-core-async-ffi-unsafe.md)
- [3-10-core-env.md](3-10-core-env.md)
- [3-11-core-test.md](3-11-core-test.md)
- [3-12-core-cli.md](3-12-core-cli.md)
- [3-13-core-text-pretty.md](3-13-core-text-pretty.md)
- [3-14-core-lsp.md](3-14-core-lsp.md)
- [3-15-core-doc.md](3-15-core-doc.md)
- [3-16-core-dsl-paradigm-kits.md](3-16-core-dsl-paradigm-kits.md)
- [3-17-core-net.md](3-17-core-net.md)
- [3-18-core-system.md](3-18-core-system.md)

## 4.x エコシステム仕様（Draft）
- [4-0-ecosystem-overview.md](4-0-ecosystem-overview.md)
- [4-1-package-manager-cli.md](4-1-package-manager-cli.md)
- [4-2-registry-distribution.md](4-2-registry-distribution.md)
- [4-3-developer-toolchain.md](4-3-developer-toolchain.md)
- [4-4-community-content.md](4-4-community-content.md)
- [4-5-roadmap-metrics.md](4-5-roadmap-metrics.md)
- [4-6-risk-governance.md](4-6-risk-governance.md)

## 5.x 公式プラグイン仕様（Draft / 再検討中）
- [5-0-official-plugins-overview.md](5-0-official-plugins-overview.md)
- [5-1-system-plugin.md](5-1-system-plugin.md)
- [5-2-process-plugin.md](5-2-process-plugin.md)
- [5-3-memory-plugin.md](5-3-memory-plugin.md)
- [5-4-signal-plugin.md](5-4-signal-plugin.md)
- [5-5-hardware-plugin.md](5-5-hardware-plugin.md)
- [5-6-realtime-plugin.md](5-6-realtime-plugin.md)
- [5-7-core-parse-plugin.md](5-7-core-parse-plugin.md)

---
更新の際は章番号・ファイル名を維持し、相互リンクを `docs/spec/` 配下で完結させてください。
