# FFI Bridge Summary（ドラフト）

> 本書は Phase 2-3「FFI 契約拡張」タスク向けに、ターゲット別スタブ生成と監査ログ整備の進捗を集約する雛形である。`scripts/validate-runtime-capabilities.sh` や `scripts/ci-local.sh --target <triple>` の結果を貼り付け、`docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` で定義された指標（`ffi_bridge.audit_pass_rate` など）と同期する。

## 1. 集計メタデータ

- 更新日: 2025-10-20
- 更新者: Codex（AIエージェント）
- 対象コミット: 未確定（作業ブランチ）
- 参照計画: docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md

## 2. ターゲット別スタブ状況

| ターゲット | 呼出規約 (plan) | 所有権 (plan) | 監査タグ確認 | メモ |
| --- | --- | --- | --- | --- |
| x86_64-unknown-linux-gnu | `ccc` | `borrowed` | yes | ターゲット別メタデータテスト（`compiler/ocaml/tests/test_ffi_lowering.ml`）で `reml.bridge.stubs` とランタイムフックを確認。呼出規約サマリを `tests/golden/llvm/linux-default.ll` で追跡中。 |
| x86_64-pc-windows-msvc | `win64` | `transferred` | yes | `tests/test_ffi_lowering.ml` で stub/thunk → 外部シンボル呼び出しおよび成功メトリクス記録を検証済み。`win64` CallConv を適用し、ゴールデン（`tests/golden/llvm/windows-transferred.ll`）で監視。 |
| arm64-apple-darwin | `aarch64_aapcscc` | `borrowed` | yes | ターゲット別メタデータテスト（`compiler/ocaml/tests/test_ffi_lowering.ml`）で `reml.bridge.stubs` とサンク経路を検証。`aarch64_aapcscc` CallConv を適用し、ゴールデン（`tests/golden/llvm/macos-borrowed.ll`）を用意。CI ログ収集と IR ゴールデン強化は今後の対応。 |

> **記入例**: Linux 版のみ実装済みの場合は監査タグを `yes`、他は `pending` とし、差分に必要なタスク (例: `runtime/native/src/ffi_bridge.c` の実装) をメモ欄へ記載。

## 3. 監査ログチェック

- 取得コマンド: `remlc --emit-audit ...`
- 出力ファイル: <!-- path/to/audit.jsonl -->
- 確認項目:
- [ ] `bridge.platform` が `reports/runtime-capabilities-validation.json` のステージと一致
- [x] `bridge.abi` / `bridge.callconv` が Typer 診断と矛盾していない（`tests/test_ffi_lowering.ml` の Windows ケースで確認）
- [x] `reml.bridge.version` モジュールフラグ (1) が `llvm_gen/ffi_value_lowering.ml` で出力されている（`tests/test_ffi_lowering.ml` で確認）
- [ ] 失敗ケースが `ffi_bridge.audit_pass_rate` に反映されている（ランタイム計測 API による自動化を追加予定）
- FFI スタブメタデータ: `Codegen.codegen_module` が `reml.bridge.stubs` Named Metadata を出力（キー例: `bridge.stub_index`, `bridge.extern_symbol`, `bridge.platform`）。`reml.bridge.version` モジュールフラグ (1) を追加済みで、`main.ml` から受け渡した `stub_plans` でも同一出力を得ている。

## 4. キャプチャログ

```text
<!-- ここに CLI / CI 実行ログを貼り付ける。Linux/Windows/macOS 用の雛形は
     reports/ffi-linux-summary.md, reports/ffi-windows-summary.md, reports/ffi-macos-summary.md
     を参照して記入する。 -->
```

## 5. フォローアップ TODO

- [x] Windows スタブで `Ownership::Transferred` メタデータ生成テストを追加 (`tests/test_ffi_lowering.ml`)
- [x] `runtime/native/include/reml_ffi_bridge.h` に audit hook とメトリクス API を整備 (`runtime/native/src/ffi_bridge.c`)
- [x] `llvm_gen/codegen.ml` でプレースホルダの stub/thunk を生成し、`reml_ffi_bridge_record_status` 呼び出しを含む最低限の lowering と IR 検証 (`tests/test_ffi_lowering.ml`)
- [x] `codegen/ffi_stub_builder.ml` → `llvm_gen/ffi_value_lowering.ml` → runtime API を本実装で連結し、stub/thunk が引数マーシャリング・所有権操作を伴って `reml.bridge.stubs` をターゲット別に検証（`compiler/ocaml/tests/test_ffi_lowering.ml` で Linux/Windows/macOS を確認）
- [x] LLVM CallConv (`win64` / `aarch64_aapcscc`) を適用し、プラットフォーム固有の呼出規約を IR とメタデータに反映（`compiler/ocaml/tests/golden/llvm/*.ll` でサマリを固定）
- [ ] Borrowed/Transferred の返り値処理（`dec_ref`、`wrap_foreign_ptr` 等）を実装し、メモリ所有権の監査要件を満たす
- [ ] CLI (`remlc --emit-ir`) で生成した Linux/Windows IR に `reml.bridge.stubs` と `bridge.*` メタデータが含まれることを手動サンプルで確認し、表の `監査タグ確認` を更新
- [ ] `tooling/ci/sync-iterator-audit.sh` / `collect-iterator-audit-metrics.py` を拡張して `ffi_bridge.audit_pass_rate` を CI ゲートへ追加（Linux/Windows 共通ロジック）
- [ ] `reports/ffi-linux-summary.md`・`reports/ffi-windows-summary.md`・`reports/ffi-macos-summary.md` を実測ログで更新し、監査ゴールデン (`compiler/ocaml/tests/golden/audit/ffi-bridge-*.jsonl.golden`) を確定
- [ ] 仕様書 `docs/spec/3-9`, `docs/spec/3-6` とガイド `docs/guides/runtime-bridges.md` を stub メタデータ/計測 API 情報で更新し、`docs/notes/licensing-todo.md` の TODO 消化を記録

## 6. 参考リンク

- docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md
- docs/spec/3-9-core-async-ffi-unsafe.md
- docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md
- compiler/ocaml/src/codegen/ffi_stub_builder.ml
- compiler/ocaml/src/main.ml
- runtime/native/include/reml_ffi_bridge.h
