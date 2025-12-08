# Reml ブートストラップ実装計画 - 統合マップ

本ディレクトリは、Reml言語のブートストラップ実装（OCaml 実装から Rust セルフホスト実装への移行）を段階的に進めるための詳細計画書を格納しています。Phase 2-8 の仕様完全性監査完了をもって Rust 版コンパイラ（`compiler/rust/`）を唯一のアクティブ実装とし、OCaml 版は履歴参照用のアーカイブとして扱います。

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

OCaml 実装による Reml コンパイラの最小構成を構築します（期間: 約16週）。ここで得た成果物は Phase 2-8 以降、Rust 実装へ知見を渡す基準点として参照アーカイブになります。

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

**目標**: OCaml 実装で全仕様を検証し、Windows x64 対応を追加した上で Phase 2-8 で Rust 実装への合流を宣言する。

**主な作業ディレクトリ**: `compiler/ocaml/` （Typer/Core IR/CodeGen 拡張）, `runtime/native`, `tooling/cli`, `tooling/ci`, `docs/spec/`, `docs/notes/`

| ファイル | マイルストーン | 内容 | 期限目安 |
|---------|--------------|------|---------|
| [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md) | - | Phase 2 全体概要 | - |
| [2-1-typeclass-strategy.md](2-1-typeclass-strategy.md) | M1: 型クラス | 辞書渡し vs モノモルフィゼーション評価 | 6週 |
| [2-2-effect-system-integration.md](2-2-effect-system-integration.md) | M2: 効果タグ | effect注釈、RuntimeCapability連携 | 10週 |
| [2-3-ffi-contract-extension.md](2-3-ffi-contract-extension.md) | M2: 効果タグ | FFI ABI/所有権、ブリッジコード生成 | 10週 |
| [2-4-diagnostics-audit-pipeline.md](2-4-diagnostics-audit-pipeline.md) | M3: 診断監査 | Diagnostic/AuditEnvelope実装（骨格） | 14週 |
| [2-5-spec-drift-remediation.md](2-5-spec-drift-remediation.md) | M4: 仕様レビュー | 仕様差分解消、サンプル検証 | 18週 |
| [2-6-windows-support.md](2-6-windows-support.md) | M4: 仕様レビュー | Windows x64 (MSVC ABI) 対応 | 18週 |
| [2-7-deferred-remediation.md](2-7-deferred-remediation.md) | M4: 技術的負債整理 | 診断残課題と CI 監査ゲートの整備 | 19週 |
| [2-8-spec-integrity-audit.md](2-8-spec-integrity-audit.md) | M4: 仕様最終監査 | 仕様完全性監査と最終調整 | 20週 |

**依存関係**: 2-1 → (2-2, 2-3 並行可) → 2-4 → (2-5, 2-6 並行可) → 2-7 → 2-8

**補足資料**:
- [2-2-completion-report.md](2-2-completion-report.md): Phase 2-2 効果システム統合の完了報告書
- [2-2-to-2-3-handover.md](2-2-to-2-3-handover.md): Phase 2-3 FFI 契約拡張への引き継ぎ情報
- [2-3-completion-report.md](2-3-completion-report.md): Phase 2-3 FFI 契約拡張の完了報告書
- [2-3-to-2-4-handover.md](2-3-to-2-4-handover.md): Phase 2-4 診断・監査パイプラインへの引き継ぎ情報
- [2-4-completion-report.md](2-4-completion-report.md): Phase 2-4 診断・監査パイプラインの完了報告書
- [2-4-to-2-5-handover.md](2-4-to-2-5-handover.md): Phase 2-5 仕様差分補正への引き継ぎ情報
- [2-5-to-2-7-handover.md](2-5-to-2-7-handover.md): Phase 2-5 仕様差分是正から Phase 2-7 への横断的ハンドオーバー
- [2-7-completion-report.md](2-7-completion-report.md): Phase 2-7 診断パイプライン残課題・技術的負債整理 完了報告書
- [2-7-to-2-8-handover.md](2-7-to-2-8-handover.md): Phase 2-8 仕様完全性監査への引き継ぎ情報

### Phase 3: Core Library 完成

標準ライブラリ Chapter 3 の正式仕様を Rust 実装へ揃え、Prelude から Runtime Capability までの API を完成させます（期間: 約34週）。

**目標**: Core Prelude/Collections/Text/Numeric/IO/Diagnostics/Config/Runtime の全 API を Rust 実装で仕様通りに実装し、効果タグ・監査・Capability 契約を整合させる。

**主な作業ディレクトリ**: `compiler/rust/`（ソース/テスト）、`examples/`, `docs/spec/3-x`, `docs/guides/`, `docs/notes/`

| ファイル | マイルストーン | 内容 | 期限目安 |
|---------|--------------|------|---------|
| [3-0-phase3-self-host.md](3-0-phase3-self-host.md) | - | Phase 3 全体概要 | - |
| [3-1-core-prelude-iteration-plan.md](3-1-core-prelude-iteration-plan.md) | M1: Prelude基盤 | Option/Result/Iter と Collector 整備 | 8週 |
| [3-1-iter-collector-remediation.md](3-1-iter-collector-remediation.md) | M1: Prelude補完 | Step3 未完了項目（監査/KPI/テスト）の是正計画 | 追加タスク |
| [3-1-core-prelude-remediation.md](3-1-core-prelude-remediation.md) | M1: Prelude改善 | 実装で判明した未処理課題（Iter エラー伝播・APIインベントリ更新・テスト欠落）への対応計画 | 追加タスク |
| [3-2-core-collections-plan.md](3-2-core-collections-plan.md) | M2: Collections | 永続/可変コレクションと差分API | 16週 |
| [3-3-core-text-unicode-plan.md](3-3-core-text-unicode-plan.md) | M3: Text & Unicode | 文字列三層モデルとUnicode処理 | 20週 |
| [assets/text-unicode-api-diff.csv](assets/text-unicode-api-diff.csv) | M3: Text & Unicode | Bytes/Str/String/GraphemeSeq/TextBuilder の API 差分トラッキング | 20週 |
| [3-4-core-numeric-time-plan.md](3-4-core-numeric-time-plan.md) | M4: Numeric & Time | 統計ユーティリティと時間API | 24週 |
| [3-5-core-io-path-plan.md](3-5-core-io-path-plan.md) | M4: IO & Path | IO抽象とパス/セキュリティAPI | 26週 |
| [3-6-core-diagnostics-audit-plan.md](3-6-core-diagnostics-audit-plan.md) | M5: Diagnostics & Audit | Diagnostic/Audit基盤統合 | 30週 |
| [3-7-core-config-data-plan.md](3-7-core-config-data-plan.md) | M6: Config & Data | Manifest/Schema/互換性ポリシー | 33週 |
| [3-8-core-runtime-capability-plan.md](3-8-core-runtime-capability-plan.md) | M6: Runtime Capability | Capability Registry と Stage検証 | 34週 |

**依存関係**: 3-1 → 3-2 → 3-3 → (3-4, 3-5 並行可) → 3-6 → 3-7 → 3-8

**進行中のハイライト**
- Core.Text (3-3) のサンプル/ドキュメント更新タスク #5 を実施し、`examples/core-text/text_unicode.reml` と `expected/text_unicode.*.golden` に Grapheme/Streaming/Builder 連携を合流済み。`reports/spec-audit/ch1/core_text_examples-YYYYMMDD.md` へ実行ログを保存した。
- Core.Path (3-5 §4.3) の文字列ユーティリティを Rust Runtime に実装し、`tests/path_string_utils.rs` / `tests/data/core_path/unicode_cases.json` / `reports/spec-audit/ch3/path_unicode-20251130.md` で POSIX・Windows・UNC の正規化/結合/相対経路を検証。Core.Text 計画書と相互参照して `PathStyle` と `record_text_mem_copy` の接続を確認済み。
- Core.IO & Path (3-5 §6) のドキュメント・サンプル更新タスクを着手し、`examples/practical/core_io/file_copy/canonical.reml` / `examples/practical/core_path/security_check/relative_denied.reml`（旧 `examples/core_io` / `examples/core_path`）と `tooling/examples/run_examples.sh --suite core_io|core_path` を追加。`docs/spec/3-5-core-io-path.md`・`docs/guides/runtime-bridges.md`・`docs/guides/plugin-authoring.md` にサンプル参照や `IoContext` 運用例を追記し、`core_io.example_suite_pass_rate` KPI を `0-3-audit-and-metrics.md` に登録した。

#### Capability Registry Snapshot

`tooling/runtime/capability_list.py` を実行すると Registry の一覧 (`reml_capability list --format json`) を取得し、下記テーブルと `docs/spec/3-8-core-runtime-capability.md` を同時に更新できる。Stage/効果スコープ/Provider 情報の差分は監査ログ（`docs/notes/runtime-capability-stage-log.md#capability-list-update`）へ記録する。

<!-- capability-table:start -->
| Capability | Stage | Effect Scope | Provider | Manifest Path |
| --- | --- | --- | --- | --- |
| `io.fs.read` | `Stable` | `fs.read`<br>`io` | Core | - |
| `io.fs.write` | `Stable` | `fs.write`<br>`io`<br>`mem` | Core | - |
| `fs.permissions.read` | `Stable` | `io`<br>`security` | Core | - |
| `fs.permissions.modify` | `Stable` | `io`<br>`security` | Core | - |
| `fs.symlink.query` | `Stable` | `fs.symlink`<br>`io` | Core | - |
| `fs.symlink.modify` | `Stable` | `fs.symlink`<br>`io`<br>`security` | Core | - |
| `fs.watcher.native` | `Stable` | `io`<br>`watcher` | Core | - |
| `fs.watcher.recursive` | `Stable` | `io`<br>`watcher` | Core | - |
| `watcher.resource_limits` | `Stable` | `io`<br>`watcher` | Core | - |
| `memory.buffered_io` | `Stable` | `mem` | Core | - |
| `security.fs.policy` | `Stable` | `security` | Core | - |
| `core.time.timezone.lookup` | `Beta` | `time` | Core | - |
| `core.time.timezone.local` | `Beta` | `time` | Core | - |
| `core.collections.audit` | `Stable` | `audit`<br>`mem` | Core | - |
| `metrics.emit` | `Stable` | `audit` | Core | - |
<!-- capability-table:end -->

### Phase 4: 実用テスト統合

Rust 実装を実務レベルの `.reml` シナリオで検証し、セルフホストや正式リリースへ進む前に「コンパイル→実行→結果検証」のパイプラインを整える（期間: 約10週）。

**目標**: Phase 3 の標準ライブラリと Capability を用いた実践的シナリオを整備し、`0-3-audit-and-metrics.md` の KPI をエンドツーエンド測定に接続する。

**主な作業ディレクトリ**: `examples/`, `tooling/examples/`, `compiler/rust/frontend`, `compiler/rust/tests`, `tooling/ci`, `reports/spec-audit/ch4`

| ファイル | マイルストーン | 内容 | 期限目安 |
|---------|--------------|------|---------|
| [4-0-phase4-migration.md](4-0-phase4-migration.md) | - | Phase 4 実用テスト計画（シナリオ/パイプライン/測定） | 10週 |
| [4-1-scenario-matrix-plan.md](4-1-scenario-matrix-plan.md) | M1: シナリオマトリクス | `.reml` 入力分類、`phase4-scenario-matrix` 整備、例示コード移植 | 72週 |
| [4-1-spec-core-regression-plan.md](4-1-spec-core-regression-plan.md) | M1: 回帰是正 | spec_core/practical スイートで判明した Parser/Typeck 不一致の是正計画 | 随時 |
| [4-2-practical-execution-pipeline-plan.md](4-2-practical-execution-pipeline-plan.md) | M2: 実行パイプライン | `run_examples.sh`/`cargo test -p reml_e2e` を用いた compile→run→verify の自動化 | 75週 |
| [4-3-observability-and-metrics-plan.md](4-3-observability-and-metrics-plan.md) | M3: 観測メトリクス | `collect-iterator-audit-metrics.py --section practical` とダッシュボード整備 | 77週 |
| [4-4-field-regression-and-readiness-plan.md](4-4-field-regression-and-readiness-plan.md) | M4: フィールドデータ | レグレッション管理、`phase4-readiness.md` 作成、Phase 5 へのハンドオーバー | 78週 |

**依存関係**: Phase 3 の全成果完了 → Phase 4 シナリオ整備 → Phase 5 へハンドオーバー

#### Phase 4 監査レポートと自動同期

- `tooling/examples/run_phase4_suite.py` / `tooling/examples/run_examples.sh --suite spec_core|practical` を用いて `reports/spec-audit/ch4/spec-core-dashboard.md`・`reports/spec-audit/ch4/practical-suite-index.md` を生成し、週次レビュー資料として参照する。
- `tooling/examples/update_phase4_resolution.py` を実行すると最新の ScenarioResult が `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `resolution` / `resolution_notes` / `spec_vs_impl_decision` に反映される。`reports/spec-audit/ch4/logs/` と併せて KPI を追跡し、Phase 5 へのハンドオーバーに備える。

### Phase 5: セルフホスト移行

Phase 4 の成果を基に Rust コンパイラが自分自身をビルドできる状態を作り、Stage 0/1/2 の再現性と監査ログを確立する（期間: 約10週）。

**目標**: Self-host パイプライン（Stage 0 → Stage 1 → Stage 2）と CI ジョブを確立し、`collect-iterator-audit-metrics.py` の self-host セクションを緑化する。

**主な作業ディレクトリ**: `compiler/rust/`, `tooling/bootstrap/`, `.github/workflows/`, `reports/self-host/`

| ファイル | マイルストーン | 内容 | 期限目安 |
|---------|--------------|------|---------|
| [5-0-phase5-self-host.md](5-0-phase5-self-host.md) | - | Phase 5 Self-host 計画（Stage パイプライン/測定/リスク） | 10週 |

**依存関係**: Phase 4 シナリオ完了 → Stage 0/1/2 ビルド → Self-host 再現性レビュー → Phase 6 へ連携

### Phase 6: 移行完了と運用体制

Rust セルフホスト実装を正式版として採用し、マルチターゲットリリースとエコシステム移行を実施する（期間: 約18週）。旧 Phase 4 の計画書は Phase 6 配下として扱い、順次 `6-x` にリネームする。

**目標**: マルチターゲットリリースパイプラインを確立し、OCaml 実装を参照アーカイブとして告知する。

**主な作業ディレクトリ**: `tooling/ci`, `tooling/release`, `.github/workflows/`, `docs/spec/`, `docs/notes/`, `docs/guides/`

| ファイル | マイルストーン | 内容 | 期限目安 |
|---------|--------------|------|---------|
| [6-0-phase6-migration.md](6-0-phase6-migration.md) | - | Phase 6 全体概要 | - |
| [6-1-multitarget-compatibility-verification.md](6-1-multitarget-compatibility-verification.md) | M1: 出力一致 | 旧 Phase 4 M1（マルチターゲット差分検証） | 6週 |
| [6-2-multitarget-release-pipeline.md](6-2-multitarget-release-pipeline.md) | M2: リリース | 旧 Phase 4 M2（リリース自動化・署名） | 10週 |
| [6-3-documentation-updates.md](6-3-documentation-updates.md) | M3: エコシステム | 旧 Phase 4 M3（ドキュメント更新） | 14週 |
| [6-4-ecosystem-migration.md](6-4-ecosystem-migration.md) | M3: エコシステム | 旧 Phase 4 M3（パッケージ/プラグイン移行） | 14週 |
| [6-5-backward-compat-checklist.md](6-5-backward-compat-checklist.md) | M4: 旧実装アーカイブ | 旧 Phase 4 M4（互換チェック） | 18週 |
| [6-6-support-policy.md](6-6-support-policy.md) | M4: 旧実装アーカイブ | 旧 Phase 4 M4（サポートポリシー） | 18週 |

**依存関係**: Phase 5 Self-host ハンドオーバー → (旧)4-1 → 4-2 → (4-3, 4-4) → (4-5, 4-6)

## 全体工程と重要マイルストーン

### 工程概要

```
Phase 1 (16週) → Phase 2 (18週) → Phase 3 (34週) → Phase 4 (10週) → Phase 5 (10週) → Phase 6 (18週)
合計: 約106週 (約24ヶ月)
```

### クリティカルパス

1. **Parser実装** (1-1) → **Typer実装** (1-2) → **CodeGen実装** (1-3/1-4/1-5)
2. **型クラス戦略確定** (2-1) → **効果システム統合** (2-2) → **診断基盤** (2-4)
3. **Core Prelude整備** (3-1) → **Collections実装** (3-2) → **Text & Unicode整備** (3-3) → **Numeric/IO基盤** (3-4/3-5) → **Diagnostics/Audit統合** (3-6) → **Config/Runtime Capability完成** (3-7/3-8)
4. **実用テスト統合** (4-0) → **Self-host 準備** (5-0)
5. **互換性検証** (旧 4-1) → **リリースパイプライン** (旧 4-2) → **エコシステム移行** (旧 4-4)

### 重要な意思決定ポイント

| 時期 | 決定事項 | 関連計画書 | 影響範囲 |
|------|---------|----------|---------|
| Phase 1 M2 (8週) | HM型推論の成熟度評価 | 1-2, 0-3 | Phase 2の型クラス実装方針 |
| Phase 2 M1 (6週) | 型クラス実装方式（辞書 vs モノモルフィゼーション） | 2-1, 0-4 | Phase 3の効果タグ/Capability整合 |
| Phase 2 M4 (18週) | Windows対応完了判定 | 2-6, 0-3 | Phase 3の IO/Config 実装前提 |
| Phase 3 M3 (20週) | Unicode/文字列処理方針の確定 | 3-3, 0-3 | Parser/Diagnostics のUnicode整合 |
| Phase 3 M5 (30週) | Diagnostics/Audit 運用ポリシー承認 | 3-6, 0-4 | CLI/LSP/監査パイプライン |
| Phase 3 M6 (34週) | Capability Registry 公開可否 | 3-8, 3-7 | ランタイム機能・Manifest |
| Phase 4 M2 (6週) | 実用テストパイプラインの安定化 | 4-0, 0-3 | Self-host 着手の可否 |
| Phase 5 M2 (7週) | Self-host Stage 1/2 再現性レビュー | 5-0, 0-4 | Phase 6 互換性検証の前提 |
| Phase 6 M1 (6週) | Rust セルフホスト版の最終互換性承認 | 6-0, 4-1, 0-3 | 正式版採用/旧実装アーカイブ可否 |

## ターゲットプラットフォーム戦略

### Phase 1-2: 基盤確立
- **主ターゲット**: x86_64 Linux (System V ABI)
- **開発環境**: macOS/Linux（クロスコンパイル可）
- **Phase 2追加**: Windows x64 (MSVC ABI)

### Phase 3: コアライブラリ整備
- **焦点**: プラットフォームに依存しない標準ライブラリ API の完成
- **IO/Path 対応**: x86_64 Linux/Windows x64/ARM64 macOS を対象に検証

### Phase 4: 実用テスト統合
- **実行対象**: x86_64 Linux を基準とした `.reml` シナリオ（Windows/macOS は一部スモーク）
- **目標**: compile→run→診断→監査を実践ケースで回し、Self-host に必要な観測データを揃える

### Phase 5: セルフホスト移行
- **実行対象**: x86_64 Linux / Windows x64 / ARM64 macOS の Stage 0/1/2 ビルド
- **目標**: Self-host パイプラインを週次で安定稼働させ、Stage 間差分を監査ログで管理する

### Phase 6: 本番リリース
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
3. **互換性**: Rust 実装のゴールデンとの差分、旧 OCaml 参照との仕様差分周知
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
