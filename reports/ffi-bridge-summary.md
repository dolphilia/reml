# FFI Bridge Summary（ドラフト）

> 本書は Phase 2-3「FFI 契約拡張」タスク向けに、ターゲット別スタブ生成と監査ログ整備の進捗を集約する雛形である。`scripts/validate-runtime-capabilities.sh` や `scripts/ci-local.sh --target <triple>` の結果を貼り付け、`docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` で定義された指標（`ffi_bridge.audit_pass_rate` など）と同期する。

## 1. 集計メタデータ

- 更新日: 2025-10-19
- 更新者: <!-- your-name -->
- 対象コミット: <!-- git rev-parse HEAD -->
- 参照計画: docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md

## 2. ターゲット別スタブ状況

| ターゲット | 呼出規約 (plan) | 所有権 (plan) | 監査タグ確認 | メモ |
| --- | --- | --- | --- | --- |
| x86_64-unknown-linux-gnu | `ccc` | `borrowed` | pending | LLVM lowering へ統合予定（Linux テンプレートを `reports/ffi-linux-summary.md` に追加済み） |
| x86_64-pc-windows-msvc | `win64` | `transferred` | yes | `reml.bridge.version=1` / `reml.bridge.stubs` メタデータ生成（`tests/test_ffi_lowering.ml` で検証） |
| arm64-apple-darwin | `aarch64_aapcscc` | `borrowed` | pending | macOS 計測テンプレートを `reports/ffi-macos-summary.md` に更新、CI ログ収集待ち |

> **記入例**: Linux 版のみ実装済みの場合は監査タグを `yes`、他は `pending` とし、差分に必要なタスク (例: `runtime/native/src/ffi_bridge.c` の実装) をメモ欄へ記載。

## 3. 監査ログチェック

- 取得コマンド: `remlc --emit-audit ...`
- 出力ファイル: <!-- path/to/audit.jsonl -->
- 確認項目:
- [ ] `bridge.platform` が `reports/runtime-capabilities-validation.json` のステージと一致
- [x] `bridge.abi` / `bridge.callconv` が Typer 診断と矛盾していない（`tests/test_ffi_lowering.ml` の Windows ケースで確認）
- [ ] 失敗ケースが `ffi_bridge.audit_pass_rate` に反映されている（ランタイム計測 API による自動化を追加予定）
- FFI スタブメタデータ: `Codegen.codegen_module` が `reml.bridge.stubs` Named Metadata を出力（キー例: `bridge.stub_index`, `bridge.extern_symbol`, `bridge.platform`）。`reml.bridge.version` モジュールフラグ (1) を追加済み。

## 4. キャプチャログ

```text
<!-- ここに CLI / CI 実行ログを貼り付ける。Linux/Windows/macOS 用の雛形は
     reports/ffi-linux-summary.md, reports/ffi-windows-summary.md, reports/ffi-macos-summary.md
     を参照して記入する。 -->
```

## 5. フォローアップ TODO

- [x] Windows スタブで `Ownership::Transferred` メタデータ生成テストを追加 (`tests/test_ffi_lowering.ml`)
- [x] `runtime/native/include/reml_ffi_bridge.h` に audit hook とメトリクス API を整備 (`runtime/native/src/ffi_bridge.c`)
- [ ] `tooling/ci/sync-iterator-audit.sh` へ FFI チェックを統合し、`ffi_bridge.audit_pass_rate` を CI ゲートに追加
- [ ] Linux / macOS ゴールデン更新後に `ffi_bridge.audit_pass_rate` の 1.0 維持を自動確認するスクリプトを実装

## 6. 参考リンク

- docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md
- docs/spec/3-9-core-async-ffi-unsafe.md
- docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md
- compiler/ocaml/src/codegen/ffi_stub_builder.ml
- runtime/native/include/reml_ffi_bridge.h
