# 2-2 → 2-3 ハンドオーバー

**作成日**: 2025-10-18  
**担当**: 効果システム統合チーム → FFI 契約拡張チーム

## 1. 概要
- Phase 2-2（効果システム統合）は、Typer／Runtime／CI を貫通する効果ステージ検証を完了し、`iterator.stage.audit_pass_rate` を 1.0 で維持するゲートを GitHub Actions（Linux / macOS）へ常設しました。
- `reports/runtime-capabilities-validation.json` と `reports/iterator-stage-summary.md` が最新化され、`tooling/runtime/capabilities/default.json` の Windows override（`x86_64-pc-windows-msvc` → `beta`）も検証済みです。
- Phase 2-3（FFI 契約拡張）では、効果ステージ情報と Capability Registry の整合を前提として FFI 安全性と診断強化を進めます。本書では 2-3 着手前に確認すべき残作業と指針をまとめます。

## 2. 主要成果物
- **効果/ステージ統合実装**: `type_inference_effect.ml`, `type_error.ml`, `main.ml` を更新し、CLI/JSON/環境変数/Runtime のステージ判定を単一フローで共有。`AuditEnvelope.metadata.stage_trace` と `Diagnostic.extensions.effect.stage_trace` を同一配列で維持。
- **CI ゲート常設化**: `.github/workflows/bootstrap-linux.yml`, `.github/workflows/bootstrap-macos.yml` に `tooling/ci/sync-iterator-audit.sh` を追加し、`iterator.stage.audit_pass_rate` が 1.0 未満・`stage_trace` 欠落・LLVM 検証失敗をブロック条件として設定。
- **監査/メトリクス更新**: `reports/runtime-capabilities-validation.json` と `reports/iterator-stage-summary.md` を最新化し、手順を [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) §0.3.7 に記録。
- **設計ノート・計画書整備**: `compiler/ocaml/docs/effect-system-design-note.md`・[2-2-effect-system-integration.md](2-2-effect-system-integration.md) を更新し、残タスクと 2-3 引き継ぎ項目を明示。

## 3. 未完了タスク（フォローアップ）
| 項目 | 内容 | 推奨タイミング | 備考 |
|------|------|----------------|------|
| Windows Stage override 実行テスト自動化 | PowerShell / Windows ランナーで `remlc --runtime-capabilities ...` を実行し、`stage_trace` と `iterator-stage-summary.md` を自動収集 | Phase 2-3 序盤 | 技術的負債に登録済み |
| `iterator-stage-summary.md` 差分監視 | CI アーティファクトを比較し、pass_rate や欠落件数の変動を通知する仕組みを追加 | Phase 2-3 中盤 | Diagnostics チームと連携 |
| Stage override 拡張 | `aarch64-pc-windows-msvc` など追加ターゲットの Capability JSON を検証し、`reports/runtime-capabilities-validation.json` へ記録 | Phase 2-3 と 2-6 の並行タスク | FFI 契約拡張と整合 |
| 効果診断仕様追補 | `effects.syntax.invalid_attribute` の補助メッセージ、Stage ガイダンスを [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) / [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) に反映 | Phase 2-3 仕様レビュー | ドキュメント更新が必要 |

## 4. 2-3 着手前チェックリスト
1. `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` を再実行し、`runtime_candidates` に Windows ターゲットが含まれることを確認する。必要に応じて `--cli-stage` / `--env-stage` を併用。
2. 最新の `reports/iterator-stage-summary.md` を確認し、Typer/Runtime ステージが一致しているか、pass_rate=1.0 であるかをレビューコメントに添付。
3. FFI 追加 Capability を扱う計画（[2-3-ffi-contract-extension.md](2-3-ffi-contract-extension.md)）で参照する Stage 情報を、`tooling/runtime/capabilities/*.json` と照合して更新。
4. 追加診断・監査キーを導入する場合は、`tooling/ci/collect-iterator-audit-metrics.py` の必須項目リスト更新とゴールデン再生成を忘れずに行う。

## 5. 参照アーティファクト
- `reports/runtime-capabilities-validation.json`（2025-10-18 更新）
- `reports/iterator-stage-summary.md`（CI サマリー）
- CI 設定: `.github/workflows/bootstrap-linux.yml`, `.github/workflows/bootstrap-macos.yml`
- スクリプト: `tooling/ci/sync-iterator-audit.sh`, `tooling/ci/collect-iterator-audit-metrics.py`, `scripts/validate-runtime-capabilities.sh`
- ゴールデン: `compiler/ocaml/tests/golden/diagnostics/effects/`, `compiler/ocaml/tests/golden/audit/effects-stage.json.golden`

## 6. 連絡先・レビュア
- 効果システム実装担当: compiler/ocaml（Typer / Runtime）
- 監査・CI 担当: tooling/ci & diagnostics チーム
- 仕様整合: docs/spec Chapter 1 / 3 編集チーム
- FFI 契約拡張: Phase 2-3 FFI チーム

## 7. 備考
- Windows override 自動検証は未整備のため、Phase 2-3 で最優先タスクとしてスケジュールしてください。詳細は `compiler/ocaml/docs/technical-debt.md` の「Windows Capability Stage 自動検証不足」を参照。
- `iterator-stage-summary.md` の内容は CI アーティファクトに保存されるのみでレビュー差分が自動表示されないため、当面は PR コメントへ概要を貼り付ける運用を継続してください（差分監視ツールが整備され次第置き換え予定）。
