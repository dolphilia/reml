# Remlブートストラップ計画 - エグゼクティブサマリ

本計画書は、Reml言語のOCaml実装からセルフホスト実装（Rust 版）へ至るまでに必要な標準ライブラリ整備と移行作業を、4つのPhaseに分けて段階的に実現する詳細計画です。Phase 2-8 で仕様監査を完了した時点から、Rust 版コンパイラ（`compiler/rust/`）を唯一のアクティブ実装とし、OCaml 版は履歴参照のみを目的としたアーカイブ扱いになります。

## 計画の全体像

### 期間と規模
- **総期間**: 約86週（約20ヶ月）
- **4つのPhase**: Bootstrap → 安定化 → Core Library 完成 → 移行完了
- **27の詳細計画書**: 各計画書に8ステップの作業ブレークダウン
- **対象プラットフォーム**: x86_64 Linux（主）、Windows x64、ARM64 macOS

### Phase概要

| Phase | 期間 | 目標 | 主要成果物 |
|-------|------|------|-----------|
| **Phase 1** | 1-16週（4ヶ月） | OCaml実装によるMVP | Parser/Typer/LLVM IR生成/最小ランタイム |
| **Phase 2** | 17-34週（4.5ヶ月） | 仕様安定化と本格機能実装<br>※2-8で Rust 実装への合流を確定 | 型クラス/効果システム/診断/Windows対応 |
| **Phase 3** | 35-68週（8.5ヶ月） | Coreライブラリ仕様の実装（Rust 実装基準） | Prelude〜Runtime Capability API 完成 |
| **Phase 4** | 69-86週（4.5ヶ月） | 正式リリースとエコシステム（Rust セルフホストのみ） | マルチターゲットリリース/エコシステム移行 |

## Phase 1: Bootstrap Implementation（OCaml）

**期間**: 1-16週（4ヶ月）

Phase 1 では OCaml 実装による Reml コンパイラの最小構成を構築し、Rust 実装へ知見を移す基準点を確立します。Phase 2-8 完了後は、ここで得たコードと成果は参照アーカイブとして扱われます。

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
8. **[1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md)**: macOS プレビルド対応とCI整備

### 技術的ハイライト
- Menhirパーサジェネレータによる高品質な構文解析
- 制約ベースHM型推論の実装
- 参照カウント（RC）ベースのメモリ管理
- x86_64 Linux優先のターゲット戦略

## Phase 2: 言語仕様の安定化

**期間**: 17-34週（4.5ヶ月）

型クラス、効果システム、診断を本格実装し、仕様を確定します。Phase 2-8 の仕様完全性監査で Rust 実装への最終合流を宣言し、以降の作業は `compiler/rust/` を唯一のアクティブ実装として進めます。

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
7. **[2-7-deferred-remediation.md](2-7-deferred-remediation.md)**: 診断残課題と技術的負債整理
8. **[2-8-spec-integrity-audit.md](2-8-spec-integrity-audit.md)**: 仕様完全性監査と最終調整
9. **[2-2-completion-report.md](2-2-completion-report.md)**: Phase 2-2 効果システム統合 完了報告書
10. **[2-2-to-2-3-handover.md](2-2-to-2-3-handover.md)**: Phase 2-3 FFI 契約拡張への引き継ぎ資料
11. **[2-7-completion-report.md](2-7-completion-report.md)**: Phase 2-7 診断パイプライン残課題・技術的負債整理 完了報告書
12. **[2-7-to-2-8-handover.md](2-7-to-2-8-handover.md)**: Phase 2-8 仕様完全性監査への引き継ぎ資料

### 技術的ハイライト
- 型クラス実装方式の定量的評価（性能・コードサイズ・保守性）
- 効果システムとRuntimeCapabilityの統合
- マルチターゲット対応の基盤確立（Linux + Windows）

## Phase 3: Core Library 完成

**期間**: 35-68週（8.5ヶ月）

Phase 3 では Rust 版 Reml コンパイラを唯一の実装として標準ライブラリ仕様を仕上げ、Phase 2-8 の監査成果を `compiler/rust/` と直接同期させます。OCaml 版は設計参照のみであり、CI や成果物には関与しません。

### 主要マイルストーン
- **M1（42週）**: Prelude & Iteration - Option/Result/Iter 実装
- **M2（48週）**: Collections - 永続/可変コレクション整備
- **M3（52週）**: Text & Unicode - 文字列三層モデルと正規化
- **M4（56週）**: Numeric/IO - 統計・時間 API と IO/Path
- **M5（62週）**: Diagnostics/Config - 診断・監査・マニフェスト整合
- **M6（68週）**: Runtime Capability - Capability Registry と最終レビュー

### 重点領域
1. **[3-1-core-prelude-iteration-plan.md](3-1-core-prelude-iteration-plan.md)**: Option/Result/Iter と Collector 整備
2. **[3-2-core-collections-plan.md](3-2-core-collections-plan.md)**: 永続/可変コレクションと差分 API
3. **[3-3-core-text-unicode-plan.md](3-3-core-text-unicode-plan.md)**: Unicode 正規化・TextBuilder
4. **[3-4-core-numeric-time-plan.md](3-4-core-numeric-time-plan.md)**: 統計・時間 API、監査メトリクス
5. **[3-5-core-io-path-plan.md](3-5-core-io-path-plan.md)**: Reader/Writer/Path とセキュリティヘルパ
6. **[3-6-core-diagnostics-audit-plan.md](3-6-core-diagnostics-audit-plan.md)**: Diagnostic/Audit 基盤統合
7. **[3-7-core-config-data-plan.md](3-7-core-config-data-plan.md)**: Manifest/Schema/互換性ポリシー
8. **[3-8-core-runtime-capability-plan.md](3-8-core-runtime-capability-plan.md)**: Capability Registry と Stage 検証

### 技術的ハイライト
- 仕様に沿った効果タグ・Stage 情報の統合
- Unicode/IO/Config などプラットフォーム横断 API の整備
- Diagnostics/Audit/Capability を貫通させた監査トレーサビリティ構築

## Phase 4: 移行完了と運用体制

**期間**: 69-86週（4.5ヶ月）

Rust セルフホスト版のみを配布対象とし、マルチターゲットリリース/エコシステム移行/旧実装アーカイブを完遂します。

### 主要マイルストーン
- **M1（72週）**: 出力一致サインオフ - 3ターゲット差分承認
- **M2（76週）**: リリースパイプライン - 署名・notarization対応
- **M3（79週）**: エコシステム移行 - パッケージ/プラグイン/CI更新
- **M4（86週）**: 旧実装アーカイブ - OCaml 版を参照ブランチへ移行しサポート終了

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
| 24週 | 型クラス実装方式決定（辞書 vs モノモルフィゼーション） | Phase 3の効果タグ/Capability整合 |
| 34週 | Windows対応完了判定 | Phase 3の IO/Config 実装前提 |
| 54週 | Unicode/文字列処理方針の確定 | Parser/Diagnostics のUnicode整合 |
| 62週 | Diagnostics/Audit 運用ポリシー承認 | CLI/LSP/監査パイプライン |
| 68週 | Capability Registry 公開可否 | ランタイム機能・Manifest |
| 72週 | Rust セルフホスト版の最終品質承認 | 正式版採用の可否（旧実装アーカイブ可否） |

## 並行タスクとクリティカルパス

### Phase 1の並行タスク
- 9-12週: Core IR、DX、Linux CIは並行可
- 13-16週: LLVM、Runtimeは並行可

### Phase 2の並行タスク
- 17-24週: 型クラス、Windowsは並行可
- 24-34週: 効果、FFI、診断、仕様差分は部分並行可

### Phase 3のクリティカルパス
Prelude/Iter 実装 → Collections 整備 → Text/Unicode 実装 → Numeric・IO 基盤 → Diagnostics/Audit 統合 → Config/Runtime Capability 完成

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
- Core Prelude/Collections/Text/Numeric/IO/Diagnostics/Config/Runtime API が仕様通りに実装され CI を通過
- 効果タグ・監査・Capability ステージが整合し、メトリクス/監査ログ基準を満たす
- 仕様ドキュメントおよびマニフェスト/サンプルが更新され、フォローアップタスクが整理済み

### Phase 4完了条件
- Rust セルフホスト版のマルチターゲット成果物が署名付きで安定提供され、Phase 3 ベースライン比で性能・診断の合格基準を満たしている
- 3ターゲット全てで署名付きリリース成果物生成と配布が自動化されている
- エコシステム移行完了、OCaml 実装は参照アーカイブとして周知・保全済み

## 関連文書

### 計画書
- **[README.md](../../spec/README.md)** - 統合計画マップ（本計画の全体像とナビゲーション）
- **[IMPLEMENTATION-GUIDE.md](IMPLEMENTATION-GUIDE.md)** - 実装の進め方ガイド

### 基本方針
- **[0-1-roadmap-principles.md](0-1-roadmap-principles.md)** - 基本原則
- **[0-2-roadmap-structure.md](0-2-roadmap-structure.md)** - 文書体系
- **[0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)** - 測定指標
- **[0-4-risk-handling.md](0-4-risk-handling.md)** - リスク管理

### 主要仕様書
- **[1-1-syntax.md](../../spec/1-1-syntax.md)** - 構文仕様
- **[1-2-types-Inference.md](../../spec/1-2-types-Inference.md)** - 型システム
- **[guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)** - LLVM連携

---

**最終更新**: 2025-10-05
**責任者**: Phase開始時に各レビュアが割当
