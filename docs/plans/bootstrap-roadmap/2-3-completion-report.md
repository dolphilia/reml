# Phase 2-3 FFI 契約拡張 完了報告書

> 作成日: 2025-10-24  
> 担当: FFI 契約拡張チーム（Phase 2-3）

## 1. サマリー
- CLI (`--emit-ir` / `--emit-audit`) を Linux・Windows・macOS で再実行し、`tmp/cli-callconv-out/<platform>/` に成果物を集約。監査ログには `bridge.platform` と `bridge.return.{ownership,status,wrap,release_handler,rc_adjustment}` を出力させた。
- `tooling/runtime/audit-schema.json` を正式版へ更新し、`bridge.status` / `bridge.platform` / `bridge.return.*` を必須化。`compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden` および `diagnostics/ffi/unsupported-abi.json.golden` を同期。
- `tooling/ci/collect-iterator-audit-metrics.py` / `tooling/ci/sync-iterator-audit.sh` に FFI ブリッジ指標を統合し、プラットフォーム別サマリーと macOS (`macos-arm64`) の pass_rate 判定を追加。
- 仕様・ガイド（`docs/spec/3-9`, `docs/spec/3-6`, `docs/guides/runtime/runtime-bridges.md`）を更新し、監査キーの定義および CI 運用手順を明文化した。
- macOS 固有サンプル（`ffi_dispatch_async.reml` など）のビルド失敗、および Windows Stage override 自動検証の未整備は Phase 3 で対応する技術的負債 (ID 22, 23) として引き継ぐ。

## 2. 達成事項
1. **監査スキーマの拡張**  
   `tooling/runtime/audit-schema.json` v1.1 を策定し、`bridge.status` / `bridge.platform` / `bridge.return.*` を必須項目として定義。スキーマ変更に合わせて `collect-iterator-audit-metrics.py` の必須キー一覧を更新し、プラットフォーム別集計を追加。
2. **CLI 追試と成果物整理**  
   `tmp/cli-callconv-sample.reml`（＋ macOS 専用サンプル）を 3 プラットフォームで再実行し、IR・監査ログを再取得。`reports/ffi-bridge-summary.md` と `reports/ffi-macos-summary.md` にログパス・呼出規約・所有権結果を反映し、残タスクを明示。
3. **ゴールデンテストの更新**  
   `compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden` と `compiler/ocaml/tests/golden/diagnostics/ffi/unsupported-abi.json.golden` を更新し、`bridge.return.*` の検証を dune テストに組み込んだ。
4. **計画書・ドキュメントの整備**  
   `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` を最終版へ更新し、Phase 3 引き継ぎ事項を明記。仕様・ガイドにも監査指標の解説を反映した。

## 3. 既知の残課題
- **Windows Capability Stage 自動検証**（技術的負債 ID 22）  
  GitHub Actions (windows-latest) 上で `tooling/ci/sync-iterator-audit.sh` を実行するパイプラインが未整備。Stage override と `bridge.platform` の照合を自動化する必要がある。
- **macOS FFI サンプルの自動検証**（技術的負債 ID 23）  
  `ffi_dispatch_async.reml` / `ffi_malloc_arm64.reml` がビルド失敗のまま。libSystem 呼び出しを含む監査ログを収集し、`ffi_bridge.audit_pass_rate` に反映するタスクを Phase 3 へ引き継ぐ。
- **CI ワークフロー統合**  
  `ffi_bridge.audit_pass_rate` はスクリプト側で判定できる状態だが、GitHub Actions ワークフロー（Linux / Windows / macOS）へゲート条件として組み込む作業が残っている。
- **`--verify-ir` の再有効化**  
  stub 関数の無終端ブロック修正後に `--verify-ir` をデフォルトで有効にする検証手順を整える。

## 4. メトリクス
| 指標 | 現状 | 備考 |
|------|------|------|
| `ffi_bridge.audit_pass_rate` | 1.0（サンプル診断ベース） | `collect-iterator-audit-metrics.py` 手動実行で欠落キーなし。CI 連携は残課題。 |
| `iterator.stage.audit_pass_rate` | 1.0 | Phase 2-2 から継続。 |
| CLI 監査ログ `bridge.return.*` | 出力済み | Linux / Windows / macOS の成功ログで確認。 |

## 5. 必要なフォローアップ
1. GitHub Actions（Linux / Windows / macOS）で `tooling/ci/sync-iterator-audit.sh` を呼び出し、`ffi_bridge.audit_pass_rate` と macOS pass_rate のゲートを有効化する（技術的負債 ID 22 を参照）。
2. macOS 専用サンプル (`ffi_dispatch_async.reml` ほか) をビルド可能にし、監査ログをゴールデン化する（技術的負債 ID 23 を参照）。
3. `llvm_gen/codegen.ml` の stub 生成から無終端ブロックを排除し、`--verify-ir` を標準フローに戻す。
4. Windows / macOS CI で生成した `cli-callconv-*.audit.jsonl` をアーティファクト化し、`bridge.platform` と Capability Stage の突合をレビュー時に確認できる仕組みを整備する。

## 6. 添付・参照
- `reports/ffi-bridge-summary.md`（ターゲット別スタブ状況・監査ログチェック）
- `reports/ffi-macos-summary.md`（Apple Silicon 計測サマリー）
- `tooling/runtime/audit-schema.json` v1.1
- `compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden`
- `tooling/ci/collect-iterator-audit-metrics.py`
- `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md`

---

*本報告書は Phase 2-3 を完了として記録する最終版です。残課題は技術的負債リスト (ID 22, 23) と各レポートに登録済みのため、Phase 2-4 では診断パイプライン整備へ移行してください。*
