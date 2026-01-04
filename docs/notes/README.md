# docs/notes 目次

調査メモと将来計画をカテゴリ別に整理しています。

## 言語設計
- [reml-design-goals-and-appendix.md](language/reml-design-goals-and-appendix.md): 設計ゴール、横断テーマ、実装補遺や簡易 BNF の補足。
- [reml-influence-study.md](language/reml-influence-study.md): Reml の設計インスピレーションと影響源の整理。
- [reml-language-influences-analysis.md](language/reml-language-influences-analysis.md): 仕様分析に基づく影響源の詳細分析。
- [fp-language-pain-points.md](language/fp-language-pain-points.md): 関数型言語の辛さ調査と Reml への示唆。
- [glossary-rust-alignment-draft.md](language/glossary-rust-alignment-draft.md): Rust 移行に伴う用語整合の下書き。
- [localization-strategy.md](language/localization-strategy.md): L10n/i18n 方針とドキュメント言語戦略。
- [versioning-strategy-research.md](language/versioning-strategy-research.md): 言語/実装/stdlib/ABI/エコシステムのバージョニング調査。

## パーサー
- [core-parse-api-evolution.md](parser/core-parse-api-evolution.md): Core.Parse API の変更履歴と検証メモ。
- [core-parse-combinator-survey.md](parser/core-parse-combinator-survey.md): パーサコンビネータの比較調査と評価軸。
- [core-parse-cst-design.md](parser/core-parse-cst-design.md): CST 設計の方針と要件整理。
- [core-parse-improvement-survey.md](parser/core-parse-improvement-survey.md): Core.Parse 改善点の調査メモ。
- [core-parse-streaming-todo.md](parser/core-parse-streaming-todo.md): Streaming API 周辺の TODO と連携メモ。
- [core-parser-migration.md](parser/core-parser-migration.md): Rust パーサ移行の計画・作業ログ。
- [lexer-performance-study.md](parser/lexer-performance-study.md): Lexer 性能調査と改善候補。

## 型システム
- [type-inference-roadmap.md](types/type-inference-roadmap.md): 型推論のロードマップとチェックリスト。
- [typeclass-benchmark-status.md](types/typeclass-benchmark-status.md): Typeclass ベンチの実施状況ログ。
- [typeclass-performance-evaluation.md](types/typeclass-performance-evaluation.md): Typeclass 性能評価の結果まとめ。
- [pattern-matching-improvement.md](types/pattern-matching-improvement.md): パターンマッチ改善案と課題整理。

## 効果・ハンドラ
- [algebraic-effects-implementation-roadmap.md](effects/algebraic-effects-implementation-roadmap.md): 代数的効果の実装ロードマップ。
- [algebraic-effects-implementation-roadmap-revised.md](effects/algebraic-effects-implementation-roadmap-revised.md): 代数的効果の改訂版ロードマップ。
- [algebraic-effects-spec-update-plan.md](effects/algebraic-effects-spec-update-plan.md): 代数的効果仕様の更新計画。
- [algebraic-effects-review-checklist.md](effects/algebraic-effects-review-checklist.md): 代数的効果のレビュー用チェックリスト。
- [algebraic-effects-handlers-assessment.md](effects/algebraic-effects-handlers-assessment.md): 現行効果システム/ハンドラの評価。
- [algebraic-effects-handlers-spec-proposal.md](effects/algebraic-effects-handlers-spec-proposal.md): ハンドラ仕様の提案と成功基準。
- [effect-system-tracking.md](effects/effect-system-tracking.md): 効果システムの追跡ログとタスク整理。

## DSL・プラグイン
- [dsl-enhancement-proposal.md](dsl/dsl-enhancement-proposal.md): DSL 拡張の提案と方針。
- [dsl-paradigm-kit-audit-risk-notes.md](dsl/dsl-paradigm-kit-audit-risk-notes.md): パラダイムキットの監査・リスクメモ。
- [dsl-paradigm-support-research.md](dsl/dsl-paradigm-support-research.md): パラダイム支援機構の調査/提案。
- [dsl-plugin-roadmap.md](dsl/dsl-plugin-roadmap.md): DSL プラグインの提供ロードマップ。
- [opbuilder-dsl-decisions.md](dsl/opbuilder-dsl-decisions.md): OpBuilder DSL の決定事項と背景。

## 標準ライブラリ
- [core-library-outline.md](stdlib/core-library-outline.md): Core 標準ライブラリの章構成アウトライン。
- [core-library-scope.md](stdlib/core-library-scope.md): Core 標準ライブラリの範囲と設計ゴール整理。
- [stdlib-expansion-research.md](stdlib/stdlib-expansion-research.md): 標準ライブラリ拡張の調査メモ。
- [stdlib-improvement-proposal.md](stdlib/stdlib-improvement-proposal.md): 標準ライブラリ改善提案。
- [collections-audit-bridge-todo.md](stdlib/collections-audit-bridge-todo.md): Core.Collections audit_bridge の TODO 整理。
- [collect-iterator-audit-open-items.md](stdlib/collect-iterator-audit-open-items.md): Collect/Iterator 監査の未解決項目。
- [core-io-path-gap-log.md](stdlib/core-io-path-gap-log.md): Core.IO/Core.Path 仕様と実装の差分ログ。

## ランタイム
- [runtime-bridges-roadmap.md](runtime/runtime-bridges-roadmap.md): Runtime bridge 運用とロードマップ。
- [runtime-capability-stage-log.md](runtime/runtime-capability-stage-log.md): Capability stage の差分ログ。
- [runtime-metrics-capability.md](runtime/runtime-metrics-capability.md): メトリクスと Capability 連携の仕様メモ。
- [core-numeric-stability.md](runtime/core-numeric-stability.md): 数値安定化手法と再現手順。
- [core-numeric-time-gap-log.md](runtime/core-numeric-time-gap-log.md): Core.Numeric/Core.Time の差分ログ。

## テキスト・Unicode
- [text-case-width-gap.md](text/text-case-width-gap.md): ケース変換/文字幅の差分記録。
- [text-unicode-diagnostic-bridge.md](text/text-unicode-diagnostic-bridge.md): Unicode 診断ブリッジの仕様メモ。
- [text-unicode-gap-log.md](text/text-unicode-gap-log.md): Core.Text/Unicode 仕様と実装の差分ログ。
- [text-unicode-known-issues.md](text/text-unicode-known-issues.md): Unicode 関連の既知問題と回避策。
- [text-unicode-ownership.md](text/text-unicode-ownership.md): Core.Text の所有権・参照モデル整理。
- [text-unicode-performance-investigation.md](text/text-unicode-performance-investigation.md): Text/Unicode の性能調査と KPI 追跡。
- [text-unicode-segmentation-comparison.md](text/text-unicode-segmentation-comparison.md): 書記素セグメンテーション手法の比較検討。
- [unicode-upgrade-log.md](text/unicode-upgrade-log.md): Unicode テーブル/テストデータ更新の履歴。

## FFI
- [ffi-improvement-survey.md](ffi/ffi-improvement-survey.md): FFI 改善点の調査メモ。
- [ffi-wasm-component-model-log.md](ffi/ffi-wasm-component-model-log.md): WASM Component Model の検討ログ。
- [native-escape-hatches-research.md](ffi/native-escape-hatches-research.md): ネイティブ逃げ道/unsafe API の調査。

## バックエンド・クロスコンパイル
- [cross-compilation-spec-intro.md](backend/cross-compilation-spec-intro.md): クロスコンパイル導入の調査と設計方針。
- [cross-compilation-spec-update-plan.md](backend/cross-compilation-spec-update-plan.md): クロスコンパイル仕様の更新計画と判断基準。
- [llvm-spec-status-survey.md](backend/llvm-spec-status-survey.md): LLVM 仕様/実装状況の調査。
- [loop-implementation-plan.md](backend/loop-implementation-plan.md): ループ構文/実装の計画メモ。
- [a-jit.md](backend/a-jit.md): JIT/AOT バックエンドの検討事項整理。
- [performance-optimization-research-20251221.md](backend/performance-optimization-research-20251221.md): 性能最適化の調査レポート。
- [rem-experimental-updates-summary.md](backend/rem-experimental-updates-summary.md): rem 実験版の更新点と仕様反映まとめ。
- [linux-ci-llvm-link-error-report.md](backend/linux-ci-llvm-link-error-report.md): Linux CI の LLVM リンクエラー調査。
- [macos-ci-llvm-link-error-report.md](backend/macos-ci-llvm-link-error-report.md): macOS CI の LLVM リンクエラー調査。
- [linux-ci-local-setup-2025.md](backend/linux-ci-local-setup-2025.md): Linux CI 失敗のローカル再現手順。

## プロセス・運用
- [docs-update-log.md](process/docs-update-log.md): Core.Text 系ドキュメントの更新ログ。
- [guides-to-spec-integration-plan.md](process/guides-to-spec-integration-plan.md): ガイド→仕様統合の計画書。
- [spec-integrity-audit-checklist.md](process/spec-integrity-audit-checklist.md): 仕様整合監査のチェックリスト草案。
- [licensing-todo.md](process/licensing-todo.md): FFI ヘッダ生成とライセンス整理の TODO。
- [examples-regression-log.md](process/examples-regression-log.md): examples 実行結果の回帰ログ。

---
ノートを更新・追加する際は、関連する仕様・ガイドへのリンクを追記し、`docs/README.md` や `docs/plans/repository-restructure-plan.md` に影響がないか確認してください。
