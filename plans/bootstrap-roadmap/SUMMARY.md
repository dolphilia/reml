# Remlブートストラップ計画 - エグゼクティブサマリ

本計画書は、Reml言語のOCaml実装からセルフホスト実装への移行を、4つのPhaseに分けて段階的に実現する詳細計画です。

## 計画の全体像

### 期間と規模
- **総期間**: 約86週（約20ヶ月）
- **4つのPhase**: Bootstrap → 安定化 → Self-Host → 移行完了
- **27の詳細計画書**: 各計画書に8ステップの作業ブレークダウン
- **対象プラットフォーム**: x86_64 Linux（主）、Windows x64、ARM64 macOS

### Phase概要

| Phase | 期間 | 目標 | 主要成果物 |
|-------|------|------|-----------|
| **Phase 1** | 1-16週（4ヶ月） | OCaml実装によるMVP | Parser/Typer/LLVM IR生成/最小ランタイム |
| **Phase 2** | 17-34週（4.5ヶ月） | 仕様安定化と本格機能実装 | 型クラス/効果システム/診断/Windows対応 |
| **Phase 3** | 35-68週（8.5ヶ月） | Reml実装への段階的移行 | セルフホストコンパイラ/クロスコンパイル |
| **Phase 4** | 69-86週（4.5ヶ月） | 正式リリースとエコシステム | マルチターゲットリリース/エコシステム移行 |

## Phase 1: Bootstrap Implementation（OCaml）

**期間**: 1-16週（4ヶ月）

### 主要マイルストーン
- **M1（4週）**: Parser MVP - AST生成とSpan付与
- **M2（8週）**: Typer MVP - HM型推論（単相+let多相）
- **M3（12週）**: CodeGen MVP - LLVM IR生成、ランタイム連携
- **M4（16週）**: 診断フレーム - エラーメッセージ、x86_64 Linux検証

### 重点領域
1. **[1-1-parser-implementation.md](1-1-parser-implementation.md)**: Menhir使用、8ステップで字句・構文解析を完成
2. **[1-2-typer-implementation.md](1-2-typer-implementation.md)**: HM推論エンジン、TypedAST生成
3. **[1-3-core-ir-min-optimization.md](1-3-core-ir-min-optimization.md)**: Core IR設計、基本最適化
4. **[1-4-llvm-targeting.md](1-4-llvm-targeting.md)**: LLVM IR生成、x86_64 System V ABI対応
5. **[1-5-runtime-integration.md](1-5-runtime-integration.md)**: 最小ランタイム、RC所有権モデル
6. **[1-6-developer-experience.md](1-6-developer-experience.md)**: CLI整備、診断出力
7. **[1-7-linux-validation-infra.md](1-7-linux-validation-infra.md)**: x86_64 Linux CI構築

### 技術的ハイライト
- Menhirパーサジェネレータによる高品質な構文解析
- 制約ベースHM型推論の実装
- 参照カウント（RC）ベースのメモリ管理
- x86_64 Linux優先のターゲット戦略

## Phase 2: 言語仕様の安定化

**期間**: 17-34週（4.5ヶ月）

### 主要マイルストーン
- **M1（24週）**: 型クラスサポート - 辞書渡し vs モノモルフィゼーション評価・決定
- **M2（29週）**: 効果タグ検証 - effect注釈とRuntimeCapability連携
- **M3（31週）**: 診断監査基盤 - Diagnostic/AuditEnvelope実装
- **M4（34週）**: 仕様レビュー完了 - Windows x64対応完了、仕様差分解消

### 重点領域
1. **[2-1-typeclass-strategy.md](2-1-typeclass-strategy.md)**: 型クラス実装方式の評価と決定
2. **[2-2-effect-system-integration.md](2-2-effect-system-integration.md)**: 効果システムとCapability整合
3. **[2-3-ffi-contract-extension.md](2-3-ffi-contract-extension.md)**: FFI ABI/所有権、Linux/Windows対応
4. **[2-4-diagnostics-audit-pipeline.md](2-4-diagnostics-audit-pipeline.md)**: 診断・監査パイプライン
5. **[2-5-spec-drift-remediation.md](2-5-spec-drift-remediation.md)**: 仕様差分の解消
6. **[2-6-windows-support.md](2-6-windows-support.md)**: Windows x64 MSVC対応

### 技術的ハイライト
- 型クラス実装方式の定量的評価（性能・コードサイズ・保守性）
- 効果システムとRuntimeCapabilityの統合
- マルチターゲット対応の基盤確立（Linux + Windows）

## Phase 3: Self-Host Transition

**期間**: 35-68週（8.5ヶ月）

### 主要マイルストーン
- **M1（42週）**: Parser移植 - Core.Parse APIのReml実装
- **M2（48週）**: TypeChecker移植 - HM型推論のReml実装
- **M3（54週）**: クロスコンパイル - 3ターゲット対応完了
- **M4（58週）**: CodeGen移植 - Core/MIR/LLVM IR生成
- **M5（62週）**: ランタイム統合 - Capability/Stage検証、RC vs GC評価
- **M6（68週）**: セルフホストビルド - 3段階CI構築、IR比較自動化

### 重点領域
1. **[3-1-reml-parser-port.md](3-1-reml-parser-port.md)**: Core.Parse実装、ストリーミングAPI
2. **[3-2-reml-typechecker-port.md](3-2-reml-typechecker-port.md)**: 型推論・型クラスのReml移植
3. **[3-3-cross-compilation.md](3-3-cross-compilation.md)**: x86_64 Linux/Windows + ARM64 macOS対応
4. **[3-4-intermediate-ir-and-codegen.md](3-4-intermediate-ir-and-codegen.md)**: IR/CodeGenの再実装
5. **[3-5-runtime-capability-integration.md](3-5-runtime-capability-integration.md)**: Capability統合
6. **[3-6-memory-management-evaluation.md](3-6-memory-management-evaluation.md)**: RC vs GC評価
7. **[3-7-self-host-build-pipeline.md](3-7-self-host-build-pipeline.md)**: セルフホストCI構築
8. **[3-8-doc-spec-feedback.md](3-8-doc-spec-feedback.md)**: 仕様フィードバック反映

### 技術的ハイライト
- Core.Parse APIによるストリーミングパーサ実装
- 3ターゲットクロスコンパイル機能
- RC vs GCの定量的評価と方針決定
- OCaml→Reml→セルフホストの3段階CI

## Phase 4: 移行完了と運用体制

**期間**: 69-86週（4.5ヶ月）

### 主要マイルストーン
- **M1（72週）**: 出力一致サインオフ - 3ターゲット差分承認
- **M2（76週）**: リリースパイプライン - 署名・notarization対応
- **M3（79週）**: エコシステム移行 - パッケージ/プラグイン/CI更新
- **M4（86週）**: OCaml実装アーカイブ - LTS化とサポート終了

### 重点領域
1. **[4-1-multitarget-compatibility-verification.md](4-1-multitarget-compatibility-verification.md)**: 3ターゲット互換性検証
2. **[4-2-multitarget-release-pipeline.md](4-2-multitarget-release-pipeline.md)**: リリース自動化
3. **[4-3-documentation-updates.md](4-3-documentation-updates.md)**: ドキュメント最終化
4. **[4-4-ecosystem-migration.md](4-4-ecosystem-migration.md)**: エコシステム支援
5. **[4-5-backward-compat-checklist.md](4-5-backward-compat-checklist.md)**: 後方互換検証
6. **[4-6-support-policy.md](4-6-support-policy.md)**: サポートポリシー策定

### 技術的ハイライト
- マルチターゲットリリースパイプライン（Linux/Windows/macOS）
- コードサイニング・Apple Notarization対応
- エコシステム移行の段階的アプローチ

## 品質保証と測定

### 測定指標（[0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)）

| カテゴリ | 指標 | 目標 |
|---------|------|------|
| 性能 | parse_throughput | 10MBソース解析が線形スケール |
| 性能 | memory_peak_ratio | ピークメモリ/入力サイズ ≤ 2倍 |
| 安全性 | stage_mismatch_count | 0件（CI毎） |
| 安全性 | ffi_ownership_violation | 0件 |
| DX | diagnostic_regressions | 週次ゼロ |
| DX | error_resolution_latency | ≤ 7日 |

### リスク管理（[0-4-risk-handling.md](0-4-risk-handling.md)）

**エスカレーション基準**:
- 性能指標10%超過 → Phase進行停止
- StageミスマッチPR検出 → 新機能凍結
- 診断差分7日未解決 → 仕様見直し

## 重要な意思決定ポイント

| 時期 | 決定事項 | 影響範囲 |
|------|---------|---------|
| 8週 | HM型推論の成熟度評価 | Phase 2の型クラス実装方針 |
| 24週 | 型クラス実装方式決定（辞書 vs モノモルフィゼーション） | Phase 3のセルフホスト設計 |
| 34週 | Windows対応完了判定 | Phase 3のマルチターゲット前提 |
| 54週 | クロスコンパイル機能確定 | Phase 4のリリース対象 |
| 62週 | メモリ管理戦略決定（RC vs GC） | 長期運用の性能特性 |
| 72週 | OCaml実装との互換性承認 | 正式版採用の可否 |

## 並行タスクとクリティカルパス

### Phase 1の並行タスク
- 9-12週: Core IR、DX、Linux CIは並行可
- 13-16週: LLVM、Runtimeは並行可

### Phase 2の並行タスク
- 17-24週: 型クラス、Windowsは並行可
- 24-34週: 効果、FFI、診断、仕様差分は部分並行可

### Phase 3のクリティカルパス
Parser移植 → TypeChecker移植 → CodeGen移植 → セルフホストビルド

### Phase 4の並行タスク
- 73-79週: リリース、ドキュメントは並行可
- 80-85週: エコシステム、互換性は並行可

## 成功基準

### Phase 1完了条件
- LLVM IR生成が`opt -verify`通過
- x86_64 Linux でHello Worldが実行可能
- 性能ベースライン確立

### Phase 2完了条件
- 型クラス実装方式決定と実装完了
- Windows x64対応完了
- 仕様差分ゼロ

### Phase 3完了条件
- Reml実装がOCaml実装と同等のLLVM IR生成（差異 ±10%）
- 3ターゲット全てでセルフホストビルド成功
- RC vs GC方針決定

### Phase 4完了条件
- OCaml実装との出力一致率 ≥ 95%
- 3ターゲット全てで署名付きリリース成果物生成
- エコシステム移行完了、OCaml実装アーカイブ

## 関連文書

### 計画書
- **[README.md](README.md)** - 統合計画マップ（本計画の全体像とナビゲーション）
- **[IMPLEMENTATION-GUIDE.md](IMPLEMENTATION-GUIDE.md)** - 実装の進め方ガイド

### 基本方針
- **[0-1-roadmap-principles.md](0-1-roadmap-principles.md)** - 基本原則
- **[0-2-roadmap-structure.md](0-2-roadmap-structure.md)** - 文書体系
- **[0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)** - 測定指標
- **[0-4-risk-handling.md](0-4-risk-handling.md)** - リスク管理

### 主要仕様書
- **[1-1-syntax.md](../../1-1-syntax.md)** - 構文仕様
- **[1-2-types-Inference.md](../../1-2-types-Inference.md)** - 型システム
- **[guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)** - LLVM連携

---

**最終更新**: 2025-10-05
**責任者**: Phase開始時に各レビュアが割当
