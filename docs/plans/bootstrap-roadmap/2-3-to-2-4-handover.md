# 2-3 → 2-4 ハンドオーバー

**作成日**: 2025-10-24  
**担当**: FFI 契約拡張チーム → 診断・監査パイプラインチーム

## 1. 概要
- Phase 2-3（FFI 契約拡張）は、監査スキーマ v1.1 の確定と 3 ターゲットでの `--emit-ir` / `--emit-audit` 追試、仕様・ガイドの更新を完了しました。
- `tooling/ci/collect-iterator-audit-metrics.py` および `tooling/ci/sync-iterator-audit.sh` に `ffi_bridge.audit_pass_rate` を組み込み、プラットフォーム別サマリー（macOS `macos-arm64` を含む）の検証基盤を整備済みです。
- Phase 2-4（診断・監査パイプライン強化）は、これらの監査フィールドを前提に Diagnostic/Audit の統合整備を進めます。本書では 2-4 着手時に確認すべき成果物と未解決課題をまとめます。

## 2. 引き継ぎ成果物
- **完了報告書**: `docs/plans/bootstrap-roadmap/2-3-completion-report.md`
- **計画書更新**: `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md`（完了サマリー・技術的負債リンク付与）
- **監査レポート**: `reports/ffi-bridge-summary.md`, `reports/ffi-macos-summary.md`
- **監査スキーマ**: `tooling/runtime/audit-schema.json`（v1.1）
- **CI スクリプト**: `tooling/ci/collect-iterator-audit-metrics.py`, `tooling/ci/sync-iterator-audit.sh`
- **技術的負債リスト**: `compiler/ocaml/docs/technical-debt.md`（ID 22, 23）

## 3. 未完了タスク（Phase 2-4 へ移行）
| ID | 項目 | 内容 | 追跡先 |
|----|------|------|--------|
| 22 | Windows Stage 自動検証不足 | GitHub Actions (windows-latest) 上で `tooling/ci/sync-iterator-audit.sh` を実行し、Stage override と `bridge.platform` を検証する。 | `compiler/ocaml/docs/technical-debt.md` |
| 23 | macOS FFI サンプル自動検証不足 | `ffi_dispatch_async.reml` / `ffi_malloc_arm64.reml` のビルド・実行と `ffi_bridge.audit_pass_rate` 反映を自動化。 | 同上 |
| - | `--verify-ir` 再有効化 | stub 無終端ブロック修正後、CLI 既定で `--verify-ir` を実施できるようにする。 | `docs/plans/bootstrap-roadmap/2-3-completion-report.md` §5 |
| - | CI ゲート整備 | Linux/Windows/macOS ワークフローに `ffi_bridge.audit_pass_rate` をゲート条件として組み込み。 | 同上 |

> **対応計画**: 具体的な手順と完了条件は `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` 「引き継ぎタスク対応計画」（ID 22/ID 23 セクション）にまとめた。進捗更新時は本節と合わせて参照すること。

## 4. 2-4 着手前チェックリスト
1. `tooling/runtime/audit-schema.json` v1.1 と `docs/spec/3-6-core-diagnostics-audit.md` §2.4.3 を確認し、Diagnostic/Audit の必須フィールド（`bridge.*`, `extensions`）を共通化する。
2. `reports/ffi-bridge-summary.md` の「備考」にある技術的負債 ID 22/23 を参照し、2-4 計画書の前提条件へ取り込む。
3. `tooling/ci/collect-iterator-audit-metrics.py --output tooling/ci/iterator-audit-metrics.json` を手動実行し、macOS `macos-arm64` の pass_rate が 1.0 になることを確認（成功ログが無い場合は ID 23 を参照）。
4. `reports/ffi-macos-summary.md` の TODO セクションを Phase 3 へ引き継ぐ旨を共有し、追加サンプル実行のスケジュールを調整する。

## 5. 参考リンク
- `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`
- `reports/ffi-bridge-summary.md`
- `reports/ffi-macos-summary.md`
- `compiler/ocaml/docs/technical-debt.md#23-macos-ffi-サンプル-ffi_dispatch_async-の自動検証不足`

## 6. 備考
- Phase 2-4 では診断・監査の統合を進めるため、`ffi_bridge.audit_pass_rate` と `iterator.stage.audit_pass_rate` の両方を CI ゲートへ組み込むタスクが最優先となります。
- macOS 固有サンプルは Phase 3 での Capability 昇格に直結するため、2-4 の早い段階で buildscripts とテストケースを準備してください。

---

*本ハンドオーバーは Phase 2-4 計画策定時の前提資料として利用してください。*
