# 2.3 Phase P2 — Rust バックエンド統合作業計画

Phase P2 の目的は、OCaml 実装と等価な LLVM バックエンドを Rust 側で再現し、既存ランタイム・Capability Registry・監査パイプラインへ安全に接続することである。本計画は `docs/plans/rust-migration/2-0-llvm-backend-plan.md`・`2-1-runtime-integration.md`・`2-2-adapter-layer-guidelines.md` に記載された要求を束ね、`docs/plans/bootstrap-roadmap/2-x` 系列との整合ポイントやハンドオーバー条件を Rust 移植ディレクトリ内で追跡する。

## 2.3.1 背景
- Phase 1（Parser/TypeChecker）と Phase 2 前半（spec drift 是正、Windows 対応）で得たゴールデンアセットを Rust 実装へ引き継ぎ、OCaml 実装と dual-write で比較する枠組みが既に整備されている。
- Rust 移植計画の P2 章 (`docs/plans/rust-migration/overview.md`) では LLVM バックエンド、ランタイム連携、アダプタ層を同フェーズで達成することが前提となっており、Bootstrap Roadmap 側でも同じ完了条件を追跡する必要がある。
- Windows x64 (MSVC/GNU) の LLVM 配布物に関する調査 (`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md`) は Phase 2 の Go/No-Go 判定に直結しており、Rust 版の導線に反映しない限り Phase 3 の CI 拡張が着手できない。

## 2.3.2 ゴール
1. `compiler/rust/backend/llvm/`（仮）で生成した LLVM IR が OCaml 版 (`compiler/ocaml/src/llvm_gen/`) と同じ `TargetMachine`/`DataLayout`/最適化パスを持ち、`opt -verify` と `llc` 比較で差分ゼロを達成する。
2. Rust 実装の FFI 層が `runtime/native/` の API と Capability Registry/Stage 契約（[docs/spec/3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md)）を満たし、`AuditEnvelope.metadata.bridge.*` を発行できる。
3. FS/Network/Time/Random/Process を対象としたアダプタ層が `RunConfig` および `Diagnostic.extensions["cfg"]` にメタデータを出力し、Windows でのパス/時刻/ソケット差分を吸収する。
4. 前記 3 項の成果物を `docs/plans/rust-migration/2-x` 章で定義された完了条件と同期し、Phase 3 `3-0-ci-and-dual-write-strategy.md` へハンドオーバーできるレビューレポートを発行する。

## 2.3.3 スコープ
- **含む**: LLVM バックエンド実装とゴールデン比較、FFI/Runtime 統合、アダプタ API、監査/診断メタデータ、Windows 3 ターゲット（GNU/MSVC, macOS, Linux）の検証、dual-write 自動化（`scripts/poc_dualwrite_compare.sh` 拡張）。
- **含まない**: OCaml 実装側の大規模刷新、DSL プラグイン Capability の昇格審査（Chapter 4 以降）、CI マトリクス最適化（Phase 3 スコープ）、Self-host 実行判定（Phase 4）。
- **前提**: P1 計画 (`docs/plans/rust-migration/1-0-front-end-transition.md` ほか) の完了条件が満たされ、MIR まで Rust 側で生成できる。P0 (`0-1-baseline-and-diff-assets.md`) の比較治具が稼働しており、`p1-spec-compliance-gap.md`・`p1-rust-frontend-gap-report.md` で列挙された差分がクローズ済み。

## 2.3.4 成果物一覧

| 成果物 | 目的 | 参照先 |
| --- | --- | --- |
| Rust LLVM バックエンド crate 設計書 | `compiler/rust/backend/llvm/` のモジュール配置、`TargetMachine` 初期化、PassManager 抽象化 | `docs/plans/rust-migration/2-0-llvm-backend-plan.md`, `docs/guides/compiler/llvm-integration-notes.md` |
| バックエンド差分ハーネス更新 | `--emit-llvm` dual-write、`opt -verify`/`llc` 自動化、`reports/diagnostic-format-regression.md` への連携 | `0-1-baseline-and-diff-assets.md`, `scripts/poc_dualwrite_compare.sh` |
| Runtime/Capability 連携仕様 | `ForeignPtr`/`Span`/`RuntimeString`、`CapabilityRegistry` Stage チェック、`AuditEnvelope.metadata.bridge.*` テンプレート | `docs/plans/rust-migration/2-1-runtime-integration.md`, `docs/spec/3-8` |
| Adapter API ガイド | FS/Network/Time/Random/Process/Env の抽象化、およびターゲット能力マップ | `docs/plans/rust-migration/2-2-adapter-layer-guidelines.md`, `runtime/native/include/reml_os.h` |
| Phase P2 完了報告テンプレート | Go/No-Go 判定基準、Phase 3 への引き継ぎ項目、`docs/plans/rust-migration/overview.md` と同期するチェックリスト | `docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md`, `docs/plans/rust-migration/overview.md` |

## 2.3.5 作業ステップ

### ステップ A: LLVM バックエンド等価性の確立
1. OCaml 側 `compiler/ocaml/src/llvm_gen/` をベースに `TargetMachine`/`DataLayout`/`PassManager` の挙動を仕様化し、Rust 実装の API 設計書を作成する（`docs/plans/rust-migration/2-0-llvm-backend-plan.md` §2.0.4 を引用）。
2. `scripts/poc_dualwrite_compare.sh` を Rust バックエンドに対応させ、`--backend {ocaml,rust}` で `opt -verify`/`llc` を実行できる CLI を `0-1-baseline-and-diff-assets.md` に登録する。
3. Windows x64 (GNU/MSVC)・macOS・Linux で `llc` / `opt` のバージョン互換性を検証し、`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` に記載された fallback 手順（MSYS2 LLVM 16 → 公式 ZIP 19.1.1）を Rust 版 README へ反映する。
4. `reports/diagnostic-format-regression.md` の監査キー（`target.config.*`, `llvm.verify.*`）に Rust バックエンド出力を追加し、差分が残った場合は `2-5-spec-drift-remediation.md` へフィードバックする。

### ステップ B: Runtime/Capability 連携
1. `runtime/native/include/reml_runtime.h` の API を棚卸しし、Rust `extern "C"` 宣言と `Result<T, FfiError>` ラッパを `compiler/rust/runtime/ffi/` へ実装する。
2. 所有権ヘルパ（`ForeignPtr`, `RuntimeString`, `Span`）を追加し、参照カウント操作（`inc_ref`/`dec_ref`）を `AuditEnvelope.metadata.bridge.refcount` として記録する。
3. `CapabilityRegistry`・`StageRequirement::{Exact, AtLeast}` を Rust 側で実装し、`verify_capability_stage` の結果を `Diagnostic.extensions["bridge"]` に反映する（参照: [docs/spec/3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) §10）。
4. `effects` デバッグフラグで `effects.contract.stage_mismatch` を捕捉し、ゼロ件であることを Phase P2 exit 条件とする。差分が出た場合は `docs/plans/rust-migration/p1-spec-compliance-gap.md` で使用したフォーマットで Issue 化。

### ステップ C: アダプタ層整備
1. `docs/plans/rust-migration/2-2-adapter-layer-guidelines.md` のサブシステム表をベースに、FS/Network/Time/Random/Process API の MVP を定義し、`compiler/rust/adapter/` に配置する。
2. Windows パス（UNC, drive letter）・時計（`QueryPerformanceCounter` vs `clock_gettime`）・ソケット差分を `runtime/native/include/reml_os.h` の API で吸収し、必要に応じて Rust 側薄ラッパを追加する。
3. `RunConfig` と `Diagnostic.extensions["cfg"]` にターゲットメタデータを付与し、`target.config.mismatch` / `target.profile.missing` の診断がゼロ件であることを CI で確認する。
4. Adapter API を利用する CLI/CI 手順（`tooling/ci/`, `docs/plans/bootstrap-roadmap/2-6-windows-support.md`）を更新し、Phase 3 `3-0-phase3-self-host.md` に記載されたセルフホスト前提と矛盾しないことをレビューで確認する。

### ステップ D: 観測・ハンドオーバー準備
1. `reports/diagnostic-format-regression.md` と `tooling/ci/collect-iterator-audit-metrics.py` に Rust バックエンド指標を追加し、CI で `llvm.verify`, `bridge.stage`, `adapter.*` のメトリクスを保存する。
2. Phase P2 用 Go/No-Go チェックリストを作成し、`docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md` のフォーマットに従って `2-3` 章の成果物リスト・未解決課題を整理する。
3. `docs/plans/rust-migration/overview.md` のフェーズ表に対応する最新ステータス（完了/進行中/ブロック）を `docs/plans/bootstrap-roadmap/SUMMARY.md` に反映し、仕様変更が発生した場合は `docs/spec/0-2-glossary.md`/`docs/spec/3-10-core-env.md` など関連章へ脚注を追加する。

## 2.3.6 マイルストーン

| 週 | マイルストーン | 内容 | 主要検証 |
| --- | --- | --- | --- |
| W1-2 | LLVM 設計同期 | TargetMachine/DataLayout/Pipeline 仕様書と Rust モジュール構成の策定 | `opt -verify` smoke、`scripts/poc_dualwrite_compare.sh --backend rust` |
| W3-4 | バックエンド差分ゼロ化 | dual-write で全ゴールデン通過、Windows fallback 手順確定 | `llc`/`opt` 比較、`reports/diagnostic-format-regression.md` 更新 |
| W5-6 | Runtime/Capability 連携 | FFI/Stage チェック/監査ログ整備 | `ffi-smoke` E2E、`effects.contract.stage_mismatch` = 0 |
| W7-8 | Adapter 層と RunConfig 拡張 | FS/Network/Time/Random/Process API とターゲットメタデータ | クロスプラットフォーム単体テスト、`target.config.*` 診断ゼロ |
| W9 | 観測ハンドオーバー | Go/No-Go チェック、Phase 3 引継資料 | チェックリスト承認、`SUMMARY.md` 更新 |

## 2.3.7 依存関係とインターフェース
- **前提完了**: `docs/plans/rust-migration/1-3-dual-write-runbook.md` の差分ワークフロー、`docs/plans/bootstrap-roadmap/p1-test-migration-plan.md` のテスト資産移行。
- **仕様参照**: 型/効果/Capability は `docs/spec/1-2-types-Inference.md`・`1-3-effects-safety.md`・`3-6-core-diagnostics-audit.md`・`3-8-core-runtime-capability.md`。
- **CI/計測**: Phase 3 以降の計測ライン（`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md`、`3-0-phase3-self-host.md`）へメトリクスを連携。

## 2.3.8 リスクとフォローアップ
| リスク | 内容 | 緩和策 |
| --- | --- | --- |
| LLVM バージョン非互換 | Windows (MSYS2 16 vs 公式 19.1.1) の差分で `llc` 出力が揃わない | `windows-llvm-build-investigation.md` の fallback 手順を Rust README に組み込み、CI で両方を smoke 実行 |
| Capability Stage のミスマッチ | ランタイム/Adapter の Stage 情報が Rust 実装と乖離し `effects.contract.stage_mismatch` が多発 | `docs/spec/3-8` の Stage テーブルをソース化し、`CapabilityRegistry` 実装を単一 crate に集約 |
| Adapter API の肥大化 | FS/Network/Time/Random/Process を同フェーズで仕上げられない | MVP API を `Result<T, AdapterError>` + Stage フラグに限定し、追加機能は `docs/notes/dsl-plugin-roadmap.md` に TODO 登録 |

## 2.3.9 Exit Criteria
- 2.3.5〜2.3.8 に記載した条件が満たされ、`reports/diagnostic-format-regression.md` が Rust バックエンドの監査キー一致を記録。
- `docs/plans/bootstrap-roadmap/README.md` と `SUMMARY.md` に Phase P2 Rust 移植計画の項目が追加され、Phase 3 フェーズ文書から参照できる。
- Go/No-Go ミーティングで Phase 3 着手承認が下り、未解決項目が `2-7-deferred-remediation.md` に記録されている。
