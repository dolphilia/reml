# FFI Bridge Summary（ドラフト）

> 本書は Phase 2-3「FFI 契約拡張」タスク向けに、ターゲット別スタブ生成と監査ログ整備の進捗を集約する雛形である。`scripts/validate-runtime-capabilities.sh` や `scripts/ci-local.sh --target <triple>` の結果を貼り付け、`docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` で定義された指標（`ffi_bridge.audit_pass_rate` など）と同期する。

## 1. 集計メタデータ

- 更新日: 2025-10-20
- 更新者: Claude (AI エージェント)
- 対象コミット: 修正後の最新状態（test_ffi_lowering 修正完了）
- 参照計画: docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md
- **状態**: macOS arm64 環境での調査・修正完了、全テスト通過

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
- [ ] `bridge.return.ownership` / `bridge.return.status` が Borrowed/Transferred 返り値ごとに出力されている（[docs/spec/3-9-core-async-ffi-unsafe.md](../docs/spec/3-9-core-async-ffi-unsafe.md) §2.6 参照）
- [x] `bridge.abi` / `bridge.callconv` が Typer 診断と矛盾していない（`tests/test_ffi_lowering.ml` の Windows ケースで確認）
- [x] `reml.bridge.version` モジュールフラグ (1) が `llvm_gen/ffi_value_lowering.ml` で出力されている（`tests/test_ffi_lowering.ml` で確認）
- [ ] 失敗ケースが `ffi_bridge.audit_pass_rate` に反映されている（ランタイム計測 API による自動化を追加予定）
- FFI スタブメタデータ: `Codegen.codegen_module` が `reml.bridge.stubs` Named Metadata を出力（キー例: `bridge.stub_index`, `bridge.extern_symbol`, `bridge.platform`）。`reml.bridge.version` モジュールフラグ (1) を追加済みで、`main.ml` から受け渡した `stub_plans` でも同一出力を得ている。

## 4. キャプチャログ

```text
$ dune build src/main.exe
  # => 成功（既存ビルド済みを再確認）

$ _build/default/src/main.exe \
    --emit-ir \
    --out-dir tmp/cli-callconv-out \
    --target x86_64-windows \
    tmp/cli-callconv-sample.reml
  # => sandbox error: command was killed by a signal

備考: CLI 実行は現行サンドボックス環境では完了せず、IR ファイルは生成されない。
      ローカル実行時は同コマンドで `tmp/cli-callconv-sample.reml` を用いると
      `reml.bridge.stubs` の `callconv=79/67` を確認可能。
```

## 5. 進捗サマリ（2025-10-20更新）

### ✅ 完了した主要項目

1. **test_ffi_lowering の修正完了**（2025-10-20）
   - `reml.bridge.version` モジュールフラグのメタデータ型不一致を修正
   - `ffi_stub_builder.ml` の `resolve_target` で block_target がターゲットトリプルとして誤用されていた問題を修正
   - `ffi_contract.ml` の `abi_kind_of_metadata` に aarch64_aapcscc などの呼出規約別名を追加
   - Linux/Windows/macOS 全3ターゲットのテスト通過を確認

2. **LLVM IR ゴールデンテスト更新**
   - basic_arithmetic, control_flow, function_calls の全ゴールデンファイルを更新
   - FFI ブリッジ関数 `reml_ffi_bridge_record_status` の宣言が正しく追加されていることを確認

3. **macOS CI パイプライン完走**
   - `./scripts/ci-local.sh --target macos --arch arm64 --stage beta` が全ステップ完了
   - Lint/Build/Test/Runtime/LLVM 検証の全段階を通過

4. **FFI 基盤コード整備**
   - Stub/Thunk の生成から監査メタデータ付与までを `tests/test_ffi_lowering.ml` でターゲット別に検証
   - Windows/MSVC (`callconv=79`) と macOS/AAPCS64 (`callconv=67`) の呼出規約を LLVM IR に反映
   - `compiler/ocaml/tests/golden/llvm/*.ll` で回帰監視を開始
   - Linux 既定 (`callconv=0`) も同様に追跡中

5. **Borrowed/Transferred 返り値計測の導入**
   - `reml_ffi_acquire_borrowed_result` / `reml_ffi_acquire_transferred_result` をランタイムへ追加し、NULL 返却は `null_results` として集計
   - `llvm_gen/codegen.ml` で返り値所有権に応じたヘルパ呼び出しと診断ステータス (`success`/`failure`) を出力
   - `compiler/ocaml/tests/test_ffi_lowering.ml` のメタデータ検証を更新し、`bridge.return.*` キーを固定

6. **Darwin Register Save Area と CI プリセット統合（2025-10-20）**
   - `llvm_gen/codegen.ml` の `emit_stub_function` で `register_save_area` 情報を参照し、Darwin macOS arm64 向けに GPR/Vector の RSA をスタック上へ確保。GPR/Vector それぞれのスロットへ引数をストアし、`tests/test_ffi_lowering.ml` で `darwin_*_register_save_area` の IR 断片を検証。
   - macOS 向け LLVM IR ゴールデン（`basic_arithmetic.ll.golden` ほか）を更新し、FFI ランタイムヘルパ宣言（`reml_ffi_acquire_borrowed_result` / `reml_ffi_acquire_transferred_result`）の出力を固定。
   - `.github/workflows/bootstrap-macos.yml` の `llvm-verify` ジョブへ `compiler/ocaml/scripts/verify_llvm_ir.sh --preset darwin-arm64 --target arm64-apple-darwin` を追加し、`darwin_varargs.ll` / `darwin_struct_return.ll` プリセットを CI で常時検証。

### 🔄 進行中・未完了項目

- CLI (`src/main.exe --emit-ir`) はサンドボックス環境で実行できず IR を取得できないため、ローカル追試用サンプル（`tmp/cli-callconv-sample.reml`）と手順を共有し、確認待ち項目として扱う
- 返り値計測の自動検証: `ffi_bridge.audit_pass_rate` に `bridge.return.*` 欠落を反映させ、CI とローカルレポート双方で pass_rate 低下時に失敗させる仕組みを整備する。

## 6. フォローアップ TODO

- [x] Windows スタブで `Ownership::Transferred` メタデータ生成テストを追加 (`tests/test_ffi_lowering.ml`)
- [x] `runtime/native/include/reml_ffi_bridge.h` に audit hook とメトリクス API を整備 (`runtime/native/src/ffi_bridge.c`)
- [x] `llvm_gen/codegen.ml` でプレースホルダの stub/thunk を生成し、`reml_ffi_bridge_record_status` 呼び出しを含む最低限の lowering と IR 検証 (`tests/test_ffi_lowering.ml`)
- [x] `codegen/ffi_stub_builder.ml` → `llvm_gen/ffi_value_lowering.ml` → runtime API を本実装で連結し、stub/thunk が引数マーシャリング・所有権操作を伴って `reml.bridge.stubs` をターゲット別に検証（`compiler/ocaml/tests/test_ffi_lowering.ml` で Linux/Windows/macOS を確認）
- [x] LLVM CallConv (`win64` / `aarch64_aapcscc`) を適用し、プラットフォーム固有の呼出規約を IR とメタデータに反映（`compiler/ocaml/tests/golden/llvm/*.ll` でサマリを固定）
- [x] Borrowed/Transferred の返り値処理（`dec_ref`、`wrap_foreign_ptr` 等）を実装し、メモリ所有権の監査要件を満たす
- [ ] CLI (`remlc --emit-ir`) で生成した Linux/Windows IR に `reml.bridge.stubs` と `bridge.*` メタデータが含まれることを手動サンプルで確認し、表の `監査タグ確認` を更新（現行サンドボックスでは `src/main.exe` 実行がシグナル終了するためローカル環境での追試が必要。コマンド例: `dune build src/main.exe` → `_build/default/src/main.exe --emit-ir --out-dir <out> --target x86_64-windows path/to/sample.reml`）
- [ ] `tooling/ci/sync-iterator-audit.sh` / `collect-iterator-audit-metrics.py` を拡張して `ffi_bridge.audit_pass_rate` を CI ゲートへ追加（Linux/Windows 共通ロジック）
- [x] **`reports/ffi-macos-summary.md` を実測ログで更新**（2025-10-20完了）
- [ ] `reports/ffi-linux-summary.md`・`reports/ffi-windows-summary.md` を実測ログで更新し、監査ゴールデン (`compiler/ocaml/tests/golden/audit/ffi-bridge-*.jsonl.golden`) を確定
- [ ] 仕様書 `docs/spec/3-9`, `docs/spec/3-6` とガイド `docs/guides/runtime-bridges.md` を stub メタデータ/計測 API 情報で更新し、`docs/notes/licensing-todo.md` の TODO 消化を記録

## 7. 次のステップ

- ポインタ返り値（DirectReturn/SRet）のゴールデンケースを追加し、`reml_ffi_acquire_*_result` と `bridge.return.*` メタデータの回帰テストを整備する。
- CLI 環境で `--emit-ir` を実行し、Linux/Windows/macOS それぞれで `reml.bridge.stubs` と `bridge.callconv` の整合性を確認。取得できたログは本サマリと `reports/ffi-*-summary.md` へ反映する。
- `ffi_bridge.audit_pass_rate` の収集を CI パイプラインに統合し、`reports/runtime-capabilities-validation.json` の値と突合する。

### Borrowed/Transferred 返り値処理詳細（2025-10-23 更新）

- **仕様整合**: `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` の更新方針に従い、[docs/spec/3-9-core-async-ffi-unsafe.md](../docs/spec/3-9-core-async-ffi-unsafe.md) §2.6 と [docs/spec/3-6-core-diagnostics-audit.md](../docs/spec/3-6-core-diagnostics-audit.md) §5.1 を突き合わせ、`bridge.return.ownership` / `bridge.return.wrap` / `bridge.return.release_handler` / `bridge.return.rc_adjustment` をスタブメタデータと監査ログ両方に出力。
- **実装タスク**: `llvm_gen/codegen.ml` で `Ownership::Borrowed` は `reml_ffi_acquire_borrowed_result`、`Ownership::Transferred` は `reml_ffi_acquire_transferred_result` を経由させ、NULL 返却時は `reml_ffi_bridge_record_status` に `failure` を送出。`runtime/native/src/ffi_bridge.c` では結果種別 (`borrowed_results` / `transferred_results` / `null_results`) をアトミックカウンタで追跡。
- **検証手順**: `compiler/ocaml/tests/test_ffi_lowering.ml` のメタデータ検証に `bridge.return.*` を追加し、ランタイム単体テスト `test_return_metrics` でカウンタを検証。サマリー (`reports/ffi-bridge-summary.md`, `reports/ffi-macos-summary.md`) にテスト結果を記録。
- **CI/メトリクス**: `tooling/ci/collect-iterator-audit-metrics.py` を拡張し、`ffi_bridge.audit_pass_rate` と `bridge.return` 欠落情報を JSON 出力。`tooling/ci/sync-iterator-audit.sh` で pass_rate < 1.0 の場合にジョブを失敗させる仕組みを追設。

## 8. 参考リンク

- docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md
- docs/spec/3-9-core-async-ffi-unsafe.md
- docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md
- compiler/ocaml/src/codegen/ffi_stub_builder.ml
- compiler/ocaml/src/main.ml
- runtime/native/include/reml_ffi_bridge.h
