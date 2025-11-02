# Reml 公式仕様目次

章番号付き仕様書を章別に整理しています。各章の概要は以下のとおりです。

## 0.x 導入資料・設計指針
- [0-0-overview.md](0-0-overview.md) — Reml の概要と設計ゴール
- [0-1-project-purpose.md](0-1-project-purpose.md) — プロジェクトの目的と判断指針
- [0-2-glossary.md](0-2-glossary.md) — 用語集
- [0-3-code-style-guide.md](0-3-code-style-guide.md) — コードスタイルとサンプル規約

## 1.x 言語コア仕様
- [1-0-language-core-overview.md](1-0-language-core-overview.md)
- [1-1-syntax.md](1-1-syntax.md) — 構文仕様（識別子セクションに ASCII 限定の暫定脚注、効果構文は `-Zalgebraic-effects` 前提の PoC 脚注で Stage を明示）
- [1-2-types-Inference.md](1-2-types-Inference.md) — 効果行統合の移行ガードと暫定運用を脚注 `[^type-row-metadata-phase25]` で明示
- [1-3-effects-safety.md](1-3-effects-safety.md) — 残余効果 (`Σ_before`/`Σ_after`) の PoC 計測 (Phase 2-5 `EFFECT-002` Step4) と `type_row_mode` ガード脚注 `[^type-row-metadata-phase25]` を同期管理
- [1-4-test-unicode-model.md](1-4-test-unicode-model.md)
- [1-5-formal-grammar-bnf.md](1-5-formal-grammar-bnf.md) — 形式文法（`Ident` に Phase 2-5 時点の ASCII 暫定脚注、効果構文 PoC の注記あり）

## 2.x 標準パーサー API
- [2-0-parser-api-overview.md](2-0-parser-api-overview.md)
- [2-1-parser-type.md](2-1-parser-type.md)
- [2-2-core-combinator.md](2-2-core-combinator.md)
- [2-3-lexer.md](2-3-lexer.md) — 字句仕様（Phase 2-5 は ASCII 限定で運用し、脚注で Unicode プロファイル移行計画を参照可能）
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
- [3-6-core-diagnostics-audit.md](3-6-core-diagnostics-audit.md) — `effects.type_row.integration_blocked` 診断と `effect.type_row.*` 監査キーを脚注 `[^type-row-metadata-phase25]` で管理
- [3-7-core-config-data.md](3-7-core-config-data.md)
- [3-8-core-runtime-capability.md](3-8-core-runtime-capability.md) — Stage 管理と `-Z` フラグ運用（効果構文 Capability は Phase 2-5 時点で Experimental と明記）
- [3-9-core-async-ffi-unsafe.md](3-9-core-async-ffi-unsafe.md)
- [3-10-core-env.md](3-10-core-env.md)

## 4.x 公式プラグイン仕様（Draft）
- [4-0-official-plugins-overview.md](4-0-official-plugins-overview.md)
- [4-1-system-plugin.md](4-1-system-plugin.md)
- [4-2-process-plugin.md](4-2-process-plugin.md)
- [4-3-memory-plugin.md](4-3-memory-plugin.md)
- [4-4-signal-plugin.md](4-4-signal-plugin.md)
- [4-5-hardware-plugin.md](4-5-hardware-plugin.md)
- [4-6-realtime-plugin.md](4-6-realtime-plugin.md)
- [4-7-core-parse-plugin.md](4-7-core-parse-plugin.md)

## 5.x エコシステム仕様（Draft）
- [5-0-ecosystem-overview.md](5-0-ecosystem-overview.md)
- [5-1-package-manager-cli.md](5-1-package-manager-cli.md)
- [5-2-registry-distribution.md](5-2-registry-distribution.md)
- [5-3-developer-toolchain.md](5-3-developer-toolchain.md)
- [5-4-community-content.md](5-4-community-content.md)
- [5-5-roadmap-metrics.md](5-5-roadmap-metrics.md)
- [5-6-risk-governance.md](5-6-risk-governance.md)

---
更新の際は章番号・ファイル名を維持し、相互リンクを `docs/spec/` 配下で完結させてください。

[^type-row-metadata-phase25]: Phase 2-5 `TYPE-002 Step3`（2026-04-22 完了）で追加した脚注。`RunConfig.extensions["effects"].type_row_mode = "metadata-only"` を既定ガードとし、`ty` に効果行を統合する前に仕様読者へ暫定運用を周知する。解除条件は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#type-002-effect-row-integration` を参照。
