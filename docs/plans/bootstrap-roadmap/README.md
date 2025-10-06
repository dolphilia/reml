# Reml ブートストラップ実装計画 - 統合マップ

本ディレクトリは、Reml言語のブートストラップ実装（OCaml実装からセルフホスト実装への移行）を段階的に進めるための詳細計画書を格納しています。

## 計画書の構成

### 0系列: 原則・運用ガバナンス

計画全体の基本方針、測定指標、リスク管理フレームワークを定義します。

| ファイル | 内容 | 主要な参照先 |
|---------|------|------------|
| [0-1-roadmap-principles.md](0-1-roadmap-principles.md) | 計画の基本原則と判断基準 | [0-1-project-purpose.md](../../spec/0-1-project-purpose.md) |
| [0-2-roadmap-structure.md](0-2-roadmap-structure.md) | 計画書体系と更新フロー | [0-0-overview.md](../../spec/0-0-overview.md) |
| [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) | 測定指標とレビュー記録 | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) |
| [0-4-risk-handling.md](0-4-risk-handling.md) | リスク管理とエスカレーション | [llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md) |

### Phase 1: Bootstrap Implementation (OCaml)

OCaml実装によるRemlコンパイラの最小構成を構築します（期間: 約16週）。

**目標**: x86_64 LinuxターゲットでLLVM IRを生成し、最小ランタイムと連携する。

**主な作業ディレクトリ**: `compiler/ocaml/src`, `compiler/ocaml/tests`, `runtime/native`, `tooling/cli`, `tooling/ci`

| ファイル | マイルストーン | 内容 | 期限目安 |
|---------|--------------|------|---------|
| [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md) | - | Phase 1 全体概要 | - |
| [1-1-parser-implementation.md](1-1-parser-implementation.md) | M1: Parser MVP | OCaml製パーサ、AST生成、Span付与 | 4週 |
| [1-2-typer-implementation.md](1-2-typer-implementation.md) | M2: Typer MVP | HM型推論（単相+let多相） | 8週 |
| [1-3-core-ir-min-optimization.md](1-3-core-ir-min-optimization.md) | M3: CodeGen MVP (1/3) | Core IR設計、糖衣削除、基本最適化 | 12週 |
| [1-4-llvm-targeting.md](1-4-llvm-targeting.md) | M3: CodeGen MVP (2/3) | LLVM IR生成、x86_64 Linux ABI対応 | 12週 |
| [1-5-runtime-integration.md](1-5-runtime-integration.md) | M3: CodeGen MVP (3/3) | 最小ランタイム、RC所有権モデル | 12週 |
| [1-6-developer-experience.md](1-6-developer-experience.md) | M4: 診断フレーム | CLI整備、エラーメッセージ、観測機能 | 16週 |
| [1-7-linux-validation-infra.md](1-7-linux-validation-infra.md) | M4: 診断フレーム | x86_64 Linux検証インフラ、CI構築 | 16週 |

**依存関係**: 1-1 → 1-2 → (1-3, 1-4, 1-5 並行可) → (1-6, 1-7 並行可)

### Phase 2: 言語仕様の安定化

型クラス、効果システム、診断を本格実装し、仕様を確定します（期間: 約18週）。

**目標**: OCaml実装で全仕様を検証し、Windows x64対応を追加する。

**主な作業ディレクトリ**: `compiler/ocaml/` （Typer/Core IR/CodeGen 拡張）, `runtime/native`, `tooling/cli`, `tooling/ci`, `docs/spec/`, `docs/notes/`

| ファイル | マイルストーン | 内容 | 期限目安 |
|---------|--------------|------|---------|
| [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md) | - | Phase 2 全体概要 | - |
| [2-1-typeclass-strategy.md](2-1-typeclass-strategy.md) | M1: 型クラス | 辞書渡し vs モノモルフィゼーション評価 | 6週 |
| [2-2-effect-system-integration.md](2-2-effect-system-integration.md) | M2: 効果タグ | effect注釈、RuntimeCapability連携 | 10週 |
| [2-3-ffi-contract-extension.md](2-3-ffi-contract-extension.md) | M2: 効果タグ | FFI ABI/所有権、ブリッジコード生成 | 10週 |
| [2-4-diagnostics-audit-pipeline.md](2-4-diagnostics-audit-pipeline.md) | M3: 診断監査 | Diagnostic/AuditEnvelope実装 | 14週 |
| [2-5-spec-drift-remediation.md](2-5-spec-drift-remediation.md) | M4: 仕様レビュー | 仕様差分解消、サンプル検証 | 18週 |
| [2-6-windows-support.md](2-6-windows-support.md) | M4: 仕様レビュー | Windows x64 (MSVC ABI) 対応 | 18週 |

**依存関係**: 2-1 → (2-2, 2-3 並行可) → 2-4 → (2-5, 2-6 並行可)

### Phase 3: Core Library 完成

標準ライブラリ Chapter 3 の正式仕様を Reml 実装へ揃え、Prelude から Runtime Capability までの API を完成させます（期間: 約34週）。

**目標**: Core Prelude/Collections/Text/Numeric/IO/Diagnostics/Config/Runtime の全 API を仕様通りに実装し、効果タグ・監査・Capability 契約を整合させる。

**主な作業ディレクトリ**: `compiler/ocaml/src`, `compiler/ocaml/tests`, `examples/`, `docs/spec/3-x`, `docs/guides/`, `docs/notes/`

| ファイル | マイルストーン | 内容 | 期限目安 |
|---------|--------------|------|---------|
| [3-0-phase3-self-host.md](3-0-phase3-self-host.md) | - | Phase 3 全体概要 | - |
| [3-1-core-prelude-iteration-plan.md](3-1-core-prelude-iteration-plan.md) | M1: Prelude基盤 | Option/Result/Iter と Collector 整備 | 8週 |
| [3-2-core-collections-plan.md](3-2-core-collections-plan.md) | M2: Collections | 永続/可変コレクションと差分API | 16週 |
| [3-3-core-text-unicode-plan.md](3-3-core-text-unicode-plan.md) | M3: Text & Unicode | 文字列三層モデルとUnicode処理 | 20週 |
| [3-4-core-numeric-time-plan.md](3-4-core-numeric-time-plan.md) | M4: Numeric & Time | 統計ユーティリティと時間API | 24週 |
| [3-5-core-io-path-plan.md](3-5-core-io-path-plan.md) | M4: IO & Path | IO抽象とパス/セキュリティAPI | 26週 |
| [3-6-core-diagnostics-audit-plan.md](3-6-core-diagnostics-audit-plan.md) | M5: Diagnostics & Audit | Diagnostic/Audit基盤統合 | 30週 |
| [3-7-core-config-data-plan.md](3-7-core-config-data-plan.md) | M6: Config & Data | Manifest/Schema/互換性ポリシー | 33週 |
| [3-8-core-runtime-capability-plan.md](3-8-core-runtime-capability-plan.md) | M6: Runtime Capability | Capability Registry と Stage検証 | 34週 |

**依存関係**: 3-1 → 3-2 → 3-3 → (3-4, 3-5 並行可) → 3-6 → 3-7 → 3-8

### Phase 4: 移行完了と運用体制

Reml実装を正式版として採用し、エコシステムを移行します（期間: 約18週）。

**目標**: マルチターゲットリリースパイプラインを確立し、OCaml実装をアーカイブする。

**主な作業ディレクトリ**: `tooling/ci`, `tooling/release`, `.github/workflows/`, `docs/spec/`, `docs/notes/`, `docs/guides/`

| ファイル | マイルストーン | 内容 | 期限目安 |
|---------|--------------|------|---------|
| [4-0-phase4-migration.md](4-0-phase4-migration.md) | - | Phase 4 全体概要 | - |
| [4-1-multitarget-compatibility-verification.md](4-1-multitarget-compatibility-verification.md) | M1: 出力一致 | 3ターゲット差分検証、承認プロセス | 6週 |
| [4-2-multitarget-release-pipeline.md](4-2-multitarget-release-pipeline.md) | M2: リリース | CI/CD自動化、署名・notarization | 10週 |
| [4-3-documentation-updates.md](4-3-documentation-updates.md) | M3: エコシステム | ドキュメント更新、マルチターゲット明記 | 14週 |
| [4-4-ecosystem-migration.md](4-4-ecosystem-migration.md) | M3: エコシステム | パッケージ、プラグイン、CI移行 | 14週 |
| [4-5-backward-compat-checklist.md](4-5-backward-compat-checklist.md) | M4: OCamlアーカイブ | 後方互換チェック、移行ガイド | 18週 |
| [4-6-support-policy.md](4-6-support-policy.md) | M4: OCamlアーカイブ | LTSポリシー、サポート終了計画 | 18週 |

**依存関係**: 4-1 → 4-2 → (4-3, 4-4 並行可) → (4-5, 4-6 並行可)

## 全体工程と重要マイルストーン

### 工程概要

```
Phase 1 (16週) → Phase 2 (18週) → Phase 3 (34週) → Phase 4 (18週)
合計: 約86週 (約20ヶ月)
```

### クリティカルパス

1. **Parser実装** (1-1) → **Typer実装** (1-2) → **CodeGen実装** (1-3/1-4/1-5)
2. **型クラス戦略確定** (2-1) → **効果システム統合** (2-2) → **診断基盤** (2-4)
3. **Core Prelude整備** (3-1) → **Collections実装** (3-2) → **Text & Unicode整備** (3-3) → **Numeric/IO基盤** (3-4/3-5) → **Diagnostics/Audit統合** (3-6) → **Config/Runtime Capability完成** (3-7/3-8)
4. **互換性検証** (4-1) → **リリースパイプライン** (4-2) → **エコシステム移行** (4-4)

### 重要な意思決定ポイント

| 時期 | 決定事項 | 関連計画書 | 影響範囲 |
|------|---------|----------|---------|
| Phase 1 M2 (8週) | HM型推論の成熟度評価 | 1-2, 0-3 | Phase 2の型クラス実装方針 |
| Phase 2 M1 (6週) | 型クラス実装方式（辞書 vs モノモルフィゼーション） | 2-1, 0-4 | Phase 3の効果タグ/Capability整合 |
| Phase 2 M4 (18週) | Windows対応完了判定 | 2-6, 0-3 | Phase 3の IO/Config 実装前提 |
| Phase 3 M3 (20週) | Unicode/文字列処理方針の確定 | 3-3, 0-3 | Parser/Diagnostics のUnicode整合 |
| Phase 3 M5 (30週) | Diagnostics/Audit 運用ポリシー承認 | 3-6, 0-4 | CLI/LSP/監査パイプライン |
| Phase 3 M6 (34週) | Capability Registry 公開可否 | 3-8, 3-7 | ランタイム機能・Manifest |
| Phase 4 M1 (6週) | OCaml実装との互換性承認 | 4-1, 0-3 | 正式版採用の可否 |

## ターゲットプラットフォーム戦略

### Phase 1-2: 基盤確立
- **主ターゲット**: x86_64 Linux (System V ABI)
- **開発環境**: macOS/Linux（クロスコンパイル可）
- **Phase 2追加**: Windows x64 (MSVC ABI)

### Phase 3: コアライブラリ整備
- **焦点**: プラットフォームに依存しない標準ライブラリ API の完成
- **IO/Path 対応**: x86_64 Linux/Windows x64/ARM64 macOS を対象に検証

### Phase 4: 本番リリース
- **第一ターゲット**: x86_64 Linux
- **公式サポート**: 上記3ターゲット全て
- **将来検討**: WASM/WASI（別計画）

## 測定指標と品質基準

詳細は [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) を参照。

### 性能指標
- `parse_throughput`: 10MBソース解析時間（目標: 線形スケール）
- `memory_peak_ratio`: ピークメモリ/入力サイズ（目標: 2倍以下）

### 安全性指標
- `stage_mismatch_count`: CapabilityStageミスマッチ（目標: 0件）
- `ffi_ownership_violation`: FFI所有権警告（目標: 0件）

### DX指標
- `diagnostic_regressions`: 診断差分件数（目標: 週次ゼロ）
- `error_resolution_latency`: 重大バグ修正日数（目標: 7日以内）

## リスク管理

詳細は [0-4-risk-handling.md](0-4-risk-handling.md) を参照。

### 主要リスクカテゴリ
1. **技術的負債**: 型クラス性能、所有権モデル複雑化
2. **スケジュール**: レビュー遅延、依存関係ブロック
3. **互換性**: OCaml実装との差分、診断フォーマット不一致
4. **セキュリティ/安全性**: Stage/Capabilityミスマッチ、FFIリーク
5. **エコシステム**: プラグイン互換性、移行遅延

### エスカレーション基準
- 性能指標 10%超過 → Phase進行停止
- StageミスマッチPR検出 → 新機能凍結
- 診断差分7日未解決 → 仕様見直し

## 参照資料

### 主要仕様書
- [0-0-overview.md](../../spec/0-0-overview.md) - 言語概要
- [1-1-syntax.md](../../spec/1-1-syntax.md) - 構文仕様
- [1-2-types-Inference.md](../../spec/1-2-types-Inference.md) - 型システム
- [2-0-parser-api-overview.md](../../spec/2-0-parser-api-overview.md) - Parser API
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) - 診断・監査
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) - RuntimeCapability

### 技術ガイド
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md) - LLVM連携
- [notes/cross-compilation-spec-update-plan.md](../../notes/cross-compilation-spec-update-plan.md) - クロスコンパイル
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md) - LLVM仕様調査

## 補助文書の作成方針

以下の場合に新規文書を作成します：

### パターン1: 実装詳細ガイド（xxx-detail.md）
- コード構造、インタフェース定義、内部アルゴリズムの詳細
- 例: `1-1-1-parser-module-structure.md`（現時点では未作成）

### パターン2: 作業手順書（xxx-workflow.md）
- ステップバイステップの作業指示、チェックリスト、検証コマンド
- 例: `1-3-1-core-ir-transform-workflow.md`（現時点では未作成）

### パターン3: 技術調査メモ（xxx-investigation.md）
- 技術選択の根拠、評価結果、決定事項の記録
- 例: `2-1-1-typeclass-implementation-comparison.md`（現時点では未作成）

**作成基準**: 既存計画書の「作業ブレークダウン」が10項目を超える、または技術的複雑度が高い場合に分割を検討。

---

**最終更新**: 2025-10-05
**責任者**: TBD（各Phaseレビュアが決定後に更新）
**次回レビュー**: Phase 1 開始時
