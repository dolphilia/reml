# docs/notes 目次

調査メモと将来計画をカテゴリ別に整理しています。

## 言語設計
- [reml-design-goals-and-appendix.md](language/reml-design-goals-and-appendix.md)
- [reml-influence-study.md](language/reml-influence-study.md)
- [reml-language-influences-analysis.md](language/reml-language-influences-analysis.md)
- [fp-language-pain-points.md](language/fp-language-pain-points.md)
- [glossary-rust-alignment-draft.md](language/glossary-rust-alignment-draft.md)
- [localization-strategy.md](language/localization-strategy.md)
- [versioning-strategy-research.md](language/versioning-strategy-research.md)

## パーサー
- [core-parse-api-evolution.md](parser/core-parse-api-evolution.md)
- [core-parse-combinator-survey.md](parser/core-parse-combinator-survey.md)
- [core-parse-cst-design.md](parser/core-parse-cst-design.md)
- [core-parse-improvement-survey.md](parser/core-parse-improvement-survey.md)
- [core-parse-streaming-todo.md](parser/core-parse-streaming-todo.md)
- [core-parser-migration.md](parser/core-parser-migration.md)
- [lexer-performance-study.md](parser/lexer-performance-study.md)

## 型システム
- [type-inference-roadmap.md](types/type-inference-roadmap.md)
- [typeclass-benchmark-status.md](types/typeclass-benchmark-status.md)
- [typeclass-performance-evaluation.md](types/typeclass-performance-evaluation.md)
- [pattern-matching-improvement.md](types/pattern-matching-improvement.md)

## 効果・ハンドラ
- [algebraic-effects-implementation-roadmap.md](effects/algebraic-effects-implementation-roadmap.md)
- [algebraic-effects-implementation-roadmap-revised.md](effects/algebraic-effects-implementation-roadmap-revised.md)
- [algebraic-effects-spec-update-plan.md](effects/algebraic-effects-spec-update-plan.md)
- [algebraic-effects-review-checklist.md](effects/algebraic-effects-review-checklist.md)
- [algebraic-effects-handlers-assessment.md](effects/algebraic-effects-handlers-assessment.md)
- [algebraic-effects-handlers-spec-proposal.md](effects/algebraic-effects-handlers-spec-proposal.md)
- [effect-system-tracking.md](effects/effect-system-tracking.md)

## DSL・プラグイン
- [dsl-enhancement-proposal.md](dsl/dsl-enhancement-proposal.md)
- [dsl-paradigm-kit-audit-risk-notes.md](dsl/dsl-paradigm-kit-audit-risk-notes.md)
- [dsl-paradigm-support-research.md](dsl/dsl-paradigm-support-research.md)
- [dsl-plugin-roadmap.md](dsl/dsl-plugin-roadmap.md)
- [opbuilder-dsl-decisions.md](dsl/opbuilder-dsl-decisions.md)

## 標準ライブラリ
- [core-library-outline.md](stdlib/core-library-outline.md)
- [core-library-scope.md](stdlib/core-library-scope.md)
- [stdlib-expansion-research.md](stdlib/stdlib-expansion-research.md)
- [stdlib-improvement-proposal.md](stdlib/stdlib-improvement-proposal.md)
- [collections-audit-bridge-todo.md](stdlib/collections-audit-bridge-todo.md)
- [collect-iterator-audit-open-items.md](stdlib/collect-iterator-audit-open-items.md)
- [core-io-path-gap-log.md](stdlib/core-io-path-gap-log.md)

## ランタイム
- [runtime-bridges-roadmap.md](runtime/runtime-bridges-roadmap.md)
- [runtime-capability-stage-log.md](runtime/runtime-capability-stage-log.md)
- [runtime-metrics-capability.md](runtime/runtime-metrics-capability.md)
- [core-numeric-stability.md](runtime/core-numeric-stability.md)
- [core-numeric-time-gap-log.md](runtime/core-numeric-time-gap-log.md)

## テキスト・Unicode
- [text-case-width-gap.md](text/text-case-width-gap.md)
- [text-unicode-diagnostic-bridge.md](text/text-unicode-diagnostic-bridge.md)
- [text-unicode-gap-log.md](text/text-unicode-gap-log.md)
- [text-unicode-known-issues.md](text/text-unicode-known-issues.md)
- [text-unicode-ownership.md](text/text-unicode-ownership.md)
- [text-unicode-performance-investigation.md](text/text-unicode-performance-investigation.md)
- [text-unicode-segmentation-comparison.md](text/text-unicode-segmentation-comparison.md)
- [unicode-upgrade-log.md](text/unicode-upgrade-log.md)

## FFI
- [ffi-improvement-survey.md](ffi/ffi-improvement-survey.md)
- [ffi-wasm-component-model-log.md](ffi/ffi-wasm-component-model-log.md)
- [native-escape-hatches-research.md](ffi/native-escape-hatches-research.md)

## バックエンド・クロスコンパイル
- [cross-compilation-spec-intro.md](backend/cross-compilation-spec-intro.md)
- [cross-compilation-spec-update-plan.md](backend/cross-compilation-spec-update-plan.md)
- [llvm-spec-status-survey.md](backend/llvm-spec-status-survey.md)
- [loop-implementation-plan.md](backend/loop-implementation-plan.md)
- [a-jit.md](backend/a-jit.md)
- [performance-optimization-research-20251221.md](backend/performance-optimization-research-20251221.md)
- [rem-experimental-updates-summary.md](backend/rem-experimental-updates-summary.md)
- [linux-ci-llvm-link-error-report.md](backend/linux-ci-llvm-link-error-report.md)
- [macos-ci-llvm-link-error-report.md](backend/macos-ci-llvm-link-error-report.md)
- [linux-ci-local-setup-2025.md](backend/linux-ci-local-setup-2025.md)

## プロセス・運用
- [docs-update-log.md](process/docs-update-log.md)
- [guides-to-spec-integration-plan.md](process/guides-to-spec-integration-plan.md)
- [spec-integrity-audit-checklist.md](process/spec-integrity-audit-checklist.md)
- [licensing-todo.md](process/licensing-todo.md)
- [examples-regression-log.md](process/examples-regression-log.md)

---
ノートを更新・追加する際は、関連する仕様・ガイドへのリンクを追記し、`docs/README.md` や `docs/plans/repository-restructure-plan.md` に影響がないか確認してください。
