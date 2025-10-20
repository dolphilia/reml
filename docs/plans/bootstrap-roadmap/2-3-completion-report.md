# Phase 2-3 FFI 契約拡張 完了報告書（ドラフト）

> 作成日: 2025-10-21  
> 担当: FFI 契約拡張チーム（Phase 2-3）

## 1. サマリー
- CLI (`--emit-ir` / `--emit-audit`) を Linux・Windows・macOS で再実行し、`tmp/cli-callconv-out/<platform>/` に成果物を集約。監査ログには `bridge.platform` と `bridge.return.{ownership,status,wrap,release_handler,rc_adjustment}` を追加した。
- `tooling/runtime/audit-schema.json` を v1.1 へ更新し、`bridge.status`・`bridge.platform`・`bridge.return.*` を必須化。`compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden` と `diagnostics/ffi/unsupported-abi.json.golden` を更新して dune ゴールデンテストに反映済み。
- `tooling/ci/collect-iterator-audit-metrics.py` に `ffi_bridge.audit_pass_rate` を正式追加し、ブリッジ診断で `bridge.return.*` が欠落した場合に失敗として記録できる状態を整備した。
- 既知の未解決事項（LLVM IR 検証での無終端エントリブロック、`bridge.platform` と Capability Stage の突合）は Phase 3 TODO として後続チームへ引き継ぐ。

## 2. 達成事項
1. **監査スキーマの拡張**  
   `tooling/runtime/audit-schema.json` v1.1 を策定し、`bridge.status` / `bridge.platform` / `bridge.return.*` を必須項目として定義。スキーマ変更に合わせて `collect-iterator-audit-metrics.py` の必須キー一覧を更新。
2. **CLI 追試と成果物整理**  
   `tmp/cli-callconv-sample.reml`（＋ macOS 専用サンプル）を 3 プラットフォームで再実行し、IR・監査ログを再取得。`reports/ffi-bridge-summary.md` と `reports/ffi-macos-summary.md` にログパス・呼出規約・所有権結果を反映。
3. **ゴールデンテストの更新**  
   `compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden`、`compiler/ocaml/tests/golden/diagnostics/ffi/unsupported-abi.json.golden` を最新化。`dune exec tests/test_ffi_contract.exe` で新しい監査フィールドを固定化。
4. **計画書・レポートの整備**  
   `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` に最新進捗と Phase 3 TODO を追記し、`reports/ffi-bridge-summary.md` / `reports/ffi-macos-summary.md` の更新状況を反映。

## 3. 既知の残課題
- **LLVM IR 検証 (`--verify-ir`)**: stub 関数の `entry` ブロックに終端命令が無く、`llvm-as` が失敗する。`llvm_gen/codegen.ml` で空ブロックを排除したうえで検証を再度有効化する必要がある。
- **Capability Stage と `bridge.platform` の突合**: 監査ログに `bridge.platform` が追加されたが、`reports/runtime-capabilities-validation.json` の Stage 情報と自動で比較する仕組みは未実装。Phase 3 で CI ゲートに統合する。
- **CI パイプラインへの組み込み**: `ffi_bridge.audit_pass_rate` を GitHub Actions（Linux/Windows/macOS）にゲート条件として追加する作業が未完。`tooling/ci/sync-iterator-audit.sh` への実装が必要。
- **ドキュメント更新**: `docs/spec/3-9` / `docs/spec/3-6` / `docs/guides/runtime-bridges.md` で監査フィールド (`bridge.return.*`, `bridge.platform`) を説明する追記が未反映。

## 4. メトリクス
| 指標 | 現状 | 備考 |
|------|------|------|
| `ffi_bridge.audit_pass_rate` | 暫定 0/0（スクリプト実装のみ） | CI 連携待ち。手動実行では欠落キー検出が機能することを確認。 |
| `iterator.stage.audit_pass_rate` | 1.0 | Phase 2-2 から継続。 |
| CLI 監査ログ `bridge.return.*` | 出力済み | Linux/Windows/macOS 3 ターゲットで確認。 |

## 5. 必要なフォローアップ
1. `llvm_gen/codegen.ml` の stub 生成から無終端エントリブロックを排除し、`--verify-ir` を再度有効化。
2. `tooling/ci/sync-iterator-audit.sh` と GitHub Actions ワークフローに `ffi_bridge.audit_pass_rate` をゲートとして追加。Darwin プリセット成功条件を連動。
3. `docs/spec/3-9-core-async-ffi-unsafe.md` / `docs/spec/3-6-core-diagnostics-audit.md` / `docs/guides/runtime-bridges.md` を更新し、`bridge.return.*` / `bridge.platform` の意味付けと使用例を追記。
4. `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に FFI ブリッジ指標の測定手順と CI での失敗時対応（例: pass_rate < 1.0 の扱い）を明記。
5. Windows/macOS CI で `--emit-audit` の JSONL をアーティファクト化し、`bridge.platform` と Stage override が一致することをレビューで確認するルールを整備。

## 6. 添付・参照
- `reports/ffi-bridge-summary.md`（ターゲット別スタブ状況・監査ログチェック）
- `reports/ffi-macos-summary.md`（Apple Silicon 計測サマリー）
- `tooling/runtime/audit-schema.json` v1.1
- `compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden`
- `tooling/ci/collect-iterator-audit-metrics.py`
- `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md`

---

*本報告書はレビュー用ドラフトです。Phase 2-3 の終了判定時にメトリクス値と CI ログの最終確認を行い、確定版を提出してください。*

