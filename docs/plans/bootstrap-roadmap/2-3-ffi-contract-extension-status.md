# 2-3 FFI 契約拡張 調査メモ（2025-10-24）

> 対象タスク: [2-3 FFI 契約拡張計画](2-3-ffi-contract-extension.md)  
> 参照期日: Phase 2-3 移行直前

## 1. 調査サマリー
- 効果システム統合完了時点のハンドオーバーでは、Windows Stage override 自動検証が未整備のため Phase 2-3 序盤の最優先タスクとして設定されている（`docs/plans/bootstrap-roadmap/2-2-to-2-3-handover.md`）。
- 2-3 計画の進捗表では `--verify-ir` がスタブ無終端ブロックで失敗し、監査ログのゴールデン更新や Linux/Windows レポート整備が未完了のまま停滞している（`docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md`）。
- 現行の FFI ブリッジサマリーでは監査指標 `ffi_bridge.audit_pass_rate` の CI 反映や `AuditEnvelope` スキーマ v1.1 の公開が TODO として残っており、Linux/Windows の個別レポートも未作成である（`reports/ffi-bridge-summary.md`）。
- macOS 計測サマリーは `ci-local` 成功まで到達したが、`ffi_dispatch_async` などの検証ケースが未実施であり、Linux/Windows と比較するための共通テンプレート化が指示されている（`reports/ffi-macos-summary.md`）。
- 技術的負債リストでは Windows Capability Stage の自動検証不足が Phase 2-3 着手前の重点課題として整理されている（`compiler/ocaml/docs/technical-debt.md`）。

## 2. 残タスク一覧
- **LLVM IR / スタブ整合**
  - 2025-10-24 に `llvm_gen/codegen.ml` の空エントリブロックを修正し、`--verify-ir` を再度有効化して Linux/Windows/macOS の CLI 追試が通過。次段階として 3 ターゲット分の IR と監査ログをゴールデン化。
  - `reports/ffi-bridge-summary.md` に Linux/Windows/macOS の ABI・所有権ホワイトリスト差分を集約。`reports/ffi-linux-summary.md`・`reports/ffi-windows-summary.md` を追加し、macOS 版と併せて 3 ターゲット分のサマリーを整備。
- **監査・CI**
  - `tooling/ci/collect-iterator-audit-metrics.py` と `tooling/ci/sync-iterator-audit.sh` を拡張し、`ffi_bridge.audit_pass_rate` を CI ゲートへ追加。`tooling/runtime/audit-schema.json` v1.1 に `bridge.*` フィールドを正式反映。
  - Windows ランナーで `remlc --emit-audit` を実行し `reports/iterator-stage-summary.md` を自動収集するジョブを追加、差分監視を導入。
- **プラットフォーム検証**
  - macOS で未実施の FFI サンプル（`ffi_dispatch_async.reml` など）と Linux/Windows 同型テストを実行し、`tmp/cli-callconv-out/<platform>/` の成果物を各サマリーへ反映。
  - 監査ログの必須キーを更新し、`bridge.return.*` 欠落時に `ffi_bridge.audit_pass_rate` へ反映されるようランタイム計測 API を調整。
- **文書・報告**
  - `docs/spec/3-9-core-async-ffi-unsafe.md`、`docs/spec/3-6-core-diagnostics-audit.md`、`docs/guides/runtime-bridges.md` にスタブメタデータと監査指標の実装差分を反映。
  - Phase 2-3 完了報告用ドラフトを `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` に脚注リンクとして追記し、次フェーズへの引き継ぎ項目を整理。

## 3. 推奨進行計画（次の 3 スプリント）
1. **実装集中（週 1-2）**  
   - スタブ終端バグ修正（2025-10-24 完了）に続き、`--verify-ir` 付きで再取得した 3 ターゲットの IR/監査ゴールデンを更新。  
   - ABI/所有権ホワイトリストと型チェック表を `reports/ffi-bridge-summary.md` に統合。
2. **監査・CI 強化（週 2-3）**  
   - Windows 検証ジョブ追加と `ffi_bridge.audit_pass_rate` ゲート実装、`AuditEnvelope` スキーマ v1.1 公開。  
   - `iterator-stage-summary.md` 差分監視スクリプトを導入し、アラートを CI へ統合。
3. **ドキュメント整備（週 3-4）**  
   - Linux/Windows FFI サマリー作成、仕様・ガイド更新、Phase 2-3 完了報告ドラフト追記。  
   - 未消化テストを追加し、計測結果を `0-3-audit-and-metrics.md` と各サマリーへ反映。

## 4. 関連資料
- `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md`
- `docs/plans/bootstrap-roadmap/2-2-to-2-3-handover.md`
- `reports/ffi-bridge-summary.md`
- `reports/ffi-macos-summary.md`
- `compiler/ocaml/docs/technical-debt.md`
