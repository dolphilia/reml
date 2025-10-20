# 2-3 FFI 契約拡張 調査メモ（2025-10-24）

> 対象タスク: [2-3 FFI 契約拡張計画](2-3-ffi-contract-extension.md)  
> 参照期日: Phase 2-3 移行直前

## 1. 進捗サマリー

### 1.1 完了済みの主な作業
- `llvm_gen/codegen.ml` のスタブ無終端ブロックを修正し、3 ターゲットすべてで `--verify-ir` が再度有効化された（2025-10-24 実装済み）。
- `tmp/cli-callconv-sample.reml` 系の CLI 追試を Linux / Windows / macOS 各ターゲットで再取得し、IR と監査ログを `tmp/cli-callconv-out/<platform>/` に集約。
- GitHub Actions の Windows ワークフローで PowerShell の行継続をバッククォートに変更し、メトリクス収集が安定化（`.github/workflows/bootstrap-windows.yml`）。
- `compiler/ocaml/scripts/gen_llvm_link_flags.py` と `compiler/ocaml/src/llvm_gen/dune` を調整し、macOS/Linux の LLVM リンク失敗を解消。
- `tooling/ci/collect-iterator-audit-metrics.py` を更新し、失敗診断が混在しても `ffi_bridge.audit_pass_rate` が適切に集計されるよう `bridge.status` 検証を緩和。

### 1.2 未解決の論点
- Windows Stage override の自動検証が未整備であり、Phase 2-3 序盤の最優先タスクとして残存（`docs/plans/bootstrap-roadmap/2-2-to-2-3-handover.md`）。
- `reports/ffi-bridge-summary.md` で定義した Linux / Windows / macOS 向けレポート更新が完了しておらず、監査ログのゴールデン更新も保留。
- macOS 向け計測サマリーでは `ffi_dispatch_async` など未実施ケースが残り、Linux / Windows と比較可能なテンプレート整備が必要。
- `tooling/runtime/audit-schema.json` の v1.1 公開と `bridge.*` フィールド追加が未完了で、CI 側の JSON 検証にも反映されていない。
- 技術的負債リストに登録済みの Windows Capability Stage 自動検証不足（`compiler/ocaml/docs/technical-debt.md`）が引き続きフォローアップ対象。

## 2. 残タスクと次ステップ

| カテゴリ | 現状 | 次のステップ |
| --- | --- | --- |
| LLVM IR / スタブ整合 | `--verify-ir` は 3 ターゲットで成功。IR・監査ログの最新成果物は `tmp/cli-callconv-out/<platform>/` に収集済みだが、ゴールデン差分とレポート反映が未完。 | `compiler/ocaml/tests/golden/ffi/*.ll` および `compiler/ocaml/tests/golden/audit/cli-ffi-bridge-*.jsonl.golden` を更新し、`reports/ffi-bridge-summary.md` と各プラットフォームサマリーを同期する。 |
| 監査・CI | `collect-iterator-audit-metrics.py` の緩和対応までは完了。CI ゲートへの `ffi_bridge.audit_pass_rate` 追加と `AuditEnvelope` スキーマ v1.1 の定義は未反映。 | `tooling/ci/sync-iterator-audit.sh` と Windows PowerShell スクリプトに `ffi_bridge.audit_pass_rate` を追加し、`tooling/runtime/audit-schema.json` v1.1 と整合を取る。 |
| プラットフォーム検証 | macOS の基本サンプルは成功し、Linux / Windows も CLI 再実行済み。追加シナリオ（`ffi_dispatch_async.reml` 等）や Windows ランナーでの自動収集は未実施。 | 各プラットフォームの追加サンプルを実行し、生成ログを `reports/ffi-*-summary.md` へ反映。Windows ランナーで `--emit-audit` を自動化して `reports/iterator-stage-summary.md` と連動させる。 |
| 文書・報告 | 計画書・サマリーはドラフト段階。仕様書／ガイド（3-9, 3-6, runtime-bridges）への反映と完了報告ドラフト整備が未着手。 | 残タスク消化後に関連ドキュメントへ反映し、Phase 2-3 完了報告ドラフトと Phase 3 引き継ぎ項目を `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` から参照可能にする。 |

## 3. 推奨進行計画（次の 3 スプリント）
1. **実装集中（週 1-2）**  
   - 取得済みの Linux / Windows / macOS 成果物をゴールデンへ反映し、`reports/ffi-bridge-summary.md`・`reports/ffi-*-summary.md` を更新。  
   - ABI / 所有権ホワイトリストを整理し、差分をドキュメント化。
2. **監査・CI 強化（週 2-3）**  
   - Windows 検証ジョブと `ffi_bridge.audit_pass_rate` ゲートを GitHub Actions に追加し、`AuditEnvelope` スキーマ v1.1 のレビューを開始。  
   - `iterator-stage-summary.md` の差分監視スクリプトを導入し、アラートを CI へ統合。
3. **ドキュメント整備（週 3-4）**  
   - Linux / Windows FFI サマリー作成、仕様・ガイド更新、Phase 2-3 完了報告ドラフト追記。  
   - 未消化テストを追加し、計測結果を `0-3-audit-and-metrics.md` と各サマリーへ反映。

## 4. 関連資料
- `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md`
- `docs/plans/bootstrap-roadmap/2-2-to-2-3-handover.md`
- `reports/ffi-bridge-summary.md`
- `reports/ffi-macos-summary.md`
- `compiler/ocaml/docs/technical-debt.md`
