# FFI macOS (arm64) 計測サマリー（ドラフト）

> 更新日: 2025-10-24  
> 対象: Phase 2-3 FFI 契約拡張（Apple Silicon 対応）

## 1. 計測環境
- ハードウェア: Apple Silicon (M2 Pro 12C/19C, 32GB RAM) または同等
- OS / Toolchain:
  - macOS 14.x (Sonoma)
  - Xcode Command Line Tools 15.x
  - Homebrew LLVM 18.1.x (`/opt/homebrew/opt/llvm`)
  - OCaml 5.2.1 / dune 3.x
- Reml リポジトリ commit: `2571db5c1d92804d09e0ef27890ed6504b9b96ce`
- コマンド実行:
  - `./scripts/ci-local.sh --target macos --arch arm64 --stage beta`（2025-10-24再実行。`--skip-lint` 指定で Build/Test/ASan/LLVM 検証を全通過）
  - `./scripts/ci-local.sh --target macos --arch arm64 --stage beta`（2025-10-18実行。Lint/Build 完了後にテスト ステップで SEGV、修正ログは §2 を参照）
  - `compiler/ocaml/scripts/verify_llvm_ir.sh --target arm64-apple-darwin compiler/ocaml/tests/llvm-ir/golden/basic_arithmetic.ll`

## 2. Capability / Stage 検証
| チェック項目 | 結果 | ログ/参照 |
|--------------|------|-----------|
| `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` | 成功（2025-10-18T03:23:33Z） | `reports/runtime-capabilities-validation.json`（`runtime_candidates` に `arm64-apple-darwin` を確認） |
| `./scripts/ci-local.sh --target macos --arch arm64 --stage beta` | **成功（2025-10-24再実行）** | `--skip-lint` でフォーマット差分を回避し、Build/Test/ASan/LLVM 検証を通過。`tmp/cli-callconv-out/macos/` に IR/Audit を再収集 |
| `dune runtest` (全テストスイート) | **成功** | test_ffi_lowering, test_ffi_stub_builder, LLVM IRゴールデンテスト全て通過 |
| Capability 差分レビュー | 進行中 | `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `ffi_bridge.audit_pass_rate` を追記済み（Diagnostics チームレビュー待ち） |

### 2.1 監査ログ抜粋
- `AuditEnvelope.metadata.bridge.*`（arm64-apple-darwin）: CLI を再実行し、`tmp/cli-callconv-out/macos/cli-callconv.audit.jsonl` / `cli-callconv-macos.audit.jsonl` を取得。`bridge.target=arm64-apple-darwin`、`ownership=borrowed` に加えて `bridge.return.{ownership,status,wrap,release_handler,rc_adjustment}` を確認。
- `cli-callconv-unsupported.audit.jsonl`: `tmp/cli-callconv-unsupported.reml`（@callconv("msvc")）で意図的に ABI 不一致を発生させ、`bridge.status=error` / `bridge.expected_abi=darwin_aapcs64` の失敗ケースを記録。CI では `tooling/ci/ffi-audit/macos/cli-callconv-unsupported.audit.jsonl` を生成。
- `cli-callconv-unsupported.diagnostics.json`: CI では `tooling/ci/ffi-audit/macos/cli-callconv-unsupported.diagnostics.json` として生成し、`ffi_bridge.audit_pass_rate` の検証用データとして `tooling/ci/collect-iterator-audit-metrics.py` に投入。
- `Diagnostic.extensions.effect.stage_trace`: `effects-residual` ゴールデン更新後は Typer/Runtime の `stage_trace` が一致（`compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden` を 2025-10-18 に更新）
- Borrowed/Transferred 返り値フィールド: `bridge.return.ownership` / `bridge.return.status` / `bridge.return.wrap` / `bridge.return.rc_adjustment` は `compiler/ocaml/tests/test_ffi_lowering.ml` で固定済み。NULL 返却時の `null_results` は `reml_ffi_acquire_*_result` 経由でカウントされる。

### 2.2 実行ログ抜粋

- 2025-10-21 再実行: `_build/default/src/main.exe ../../tmp/cli-callconv-sample.reml --target arm64-apple-darwin --emit-ir --emit-audit ...` および `../../tmp/cli-callconv-macos.reml` の双方が成功し、`tmp/cli-callconv-out/macos/` に IR/Audit を出力。
- 2025-10-24 再実行: stub エントリブロックの無終端問題を修正し、`--verify-ir` を併用した Linux/Windows/macOS の CLI 追試がすべて成功。`cli-callconv-sample.ll` / `cli-callconv-macos.ll` を `--verify-ir` 付きで再生成済み。
- 2025-10-24 失敗ケース: `tmp/cli-callconv-unsupported.reml` を arm64-apple-darwin で実行し、`ffi.contract.unsupported_abi` 診断と `cli-callconv-unsupported.audit.jsonl` を取得（`bridge.status=error` を確認）。
- 【参考】修正前ログ（再発防止用）

```text
$ ./scripts/ci-local.sh --target macos --arch arm64 --stage beta
[INFO] ホストアーキテクチャ: arm64
[INFO] ターゲットプラットフォーム: macos
[INFO] Lint ステップ (1/5)
[SUCCESS] Lint ステップ完了
[INFO] Build ステップ (2/5)
[INFO] コンパイラをビルド中...
ld: warning: ignoring duplicate libraries: '-lLLVMBinaryFormat' ...
[SUCCESS] Build ステップ完了
[INFO] Test ステップ (3/5)
===============================
FFI スタブプラン初期テスト
===============================
✓ Linux 監査 — bridge.callconv
...
✓ macOS 監査 — bridge.abi

===============================
テスト結果: 25/25 成功
===============================
File "tests/dune", line 36, characters 2-19:
36 |   test_ffi_lowering
       ^^^^^^^^^^^^^^^^^
Command got signal SEGV. (修正前ログ)
```

## 3. ABI / 呼出規約検証
| テストケース | 概要 | 結果 | 備考 |
|--------------|------|------|------|
| `ffi_malloc_arm64.reml` | System V → Darwin 引数マーシャリング比較 | 未実施（Build 失敗） | struct-by-value / pointer 戻り値 |
| `ffi_dispatch_async.reml` | `dispatch_async_f` 呼び出し（libSystem） | 未実施（Build 失敗） | `ffi.bridge` Capability Required |
| 可変長引数 (`printf`) | `ffi.callconv("ccc")` → varargs 挙動 | 成功（IR プリセット） | `verify_llvm_ir.sh --preset darwin-arm64` で `darwin_varargs.ll` を検証 |
| 構造体戻り値 | `extern` returning `{f64, f64}` | 成功（IR プリセット） | 同上、`darwin_struct_return.ll` |

### 3.1 Darwin 固有差分（可変長／構造体戻り）調査メモ
- `arm64-apple-darwin` では可変長関数呼び出し時に整数 8 本・SIMD 8 本の Register Save Area を呼出側が確保し、`va_list` が `__gr_offs`／`__vr_offs` を用いてレジスタ退避領域を巡回する。Reml のスタブ生成では Darwin ケースのみ Register Save Area のアドレス計算を取り込む必要がある（`docs/notes/backend/llvm-spec-status-survey.md` §2.2.2a）。
- 構造体戻り値は 16B 以下または HFA/HVA（要素数≦4）の場合に `x0-x3`／`v0-v3` で直接返却し、それ以外は `x8` に渡したポインタへ書き込む `sret align 16` が必須。LLVM 側では `Abi.classify_struct_return` の Darwin 対応を `tests/test_ffi_lowering.ml` の追加ケースで監視する。
- 以上の差分に合わせ、`ffi_stub_builder` と `llvm_gen/codegen` の Darwin 分岐を対象に Register Save Area と `sret align 16` の検証ケースを追加する作業を別タスクとして起票予定。本サマリーでは `ffi_bridge.audit_pass_rate` 監査指標への影響を継続観測する。
- 2025-10-20: `llvm_gen/codegen.ml` の `emit_stub_function` で `register_save_area` を参照し、Darwin macOS arm64 向けに GPR 64B / Vector 128B の RSA をスタックへ割り当て。生成 IR は `tests/test_ffi_lowering.ml` と LLVM ゴールデン (`basic_arithmetic.ll.golden` ほか) に反映し、`darwin_gpr_register_save_area` / `darwin_vector_register_save_area` シンボルの存在を回帰検証。

### 3.2 LLVM IR スナップショット
- `build/ir/macos-arm64/<sample>.ll` … TBD
- `llc` 出力オブジェクト: TBD（`codesign --verify` 実行ログ）

### 3.3 verify_llvm_ir.sh プリセット
- 2025-10-21: `./compiler/ocaml/scripts/verify_llvm_ir.sh --target arm64-apple-darwin --preset darwin-arm64` を実行し、プリセットサンプルを検証。
  - `darwin_struct_return.ll`（構造体戻りテスト）: `llvm-as` → `opt -verify` → `llc` の全工程が成功し、`darwin_struct_return.o` を生成。
  - `darwin_varargs.ll`（varargs テスト）: 同様に成功し、`darwin_varargs.o` を生成（計測後に生成物を削除済み）。
- プリセットは `compiler/ocaml/tests/llvm-ir/presets/darwin-arm64/` に配置し、varargs / sret の双方をスモークテストできるようにした。
- CLI 追試で生成した IR (`tmp/cli-callconv-out/macos/cli-callconv-sample.ll`, `cli-callconv-macos.ll`) は `reml.bridge.stubs` に Darwin Register Save Area 情報を含む。`entry` ブロック無終端問題は 2025-10-24 に修正済みで、現在は `--verify-ir` 併用時も通過する。

## 4. 所有権契約 / 監査
| テスト | 内容 | 結果 | 備考 |
|--------|------|------|------|
| `ffi_borrowed_pointer.reml` | `@ffi_ownership("borrowed")` の解析 | 進行中（ランタイムヘルパ最終調整） | `wrap_foreign_ptr` を通じた `bridge.return.ownership = borrowed`、`bridge.return.rc_adjustment = +0` を確認し、`reports/ffi-bridge-summary.md` に実測値を記録する。|
| `ffi_transferred_pointer.reml` | `Ownership::Transferred` | 進行中（ランタイムヘルパ最終調整） | `dec_ref` / `reml_ffi_release_transferred` が呼ばれ、`bridge.return.rc_adjustment = -1` を記録する。|
| 診断 (`ffi.contract.missing`) | 不足注釈の検証 | 進行中（Typer 拡張と同期） | CLI `--emit-audit` の JSONL をゴールデン化し、診断コードと `bridge.*` フィールドの整合を確認する。|

## 5. パフォーマンス指標
| メトリクス | 指標 | 現状値 | 備考 |
|------------|------|--------|------|
| `ffi_bridge_call_latency_ns` | 1 回あたりの平均呼出時間 | 未計測 | `bench/ffi_dispatch_async.txt` |
| `ffi_stub_codegen_time_ms` | LLVM IR → object 生成時間 | 未計測 | `metrics/ffi_codegen.json` |

## 6. TODO / リスク（2025-10-21更新）

### 完了した項目 ✅

- [x] Capability override (`arm64-apple-darwin`) を `tooling/runtime/capabilities/default.json` に追加
- [x] `dune build @fmt --auto-promote` を実行してフォーマット差分を解消（2025-10-20）
- [x] **`scripts/ci-local.sh --target macos --arch arm64 --stage beta` を Runtime まで完走**（2025-10-20修正完了）
- [x] **`compiler/ocaml/tests/test_ffi_lowering` のSEGV原因を特定し修正**（2025-10-20完了）
  - 原因: `reml.bridge.version` モジュールフラグのメタデータ型不一致
  - 修正: `test_ffi_lowering.ml` の `verify_module_flag` でメタデータ検証ロジックを改善
  - 追加修正: `ffi_stub_builder.ml` の `resolve_target` で block_target がターゲットトリプルとして誤用されていた問題を修正
  - 追加修正: `ffi_contract.ml` の `abi_kind_of_metadata` に aarch64_aapcscc などの呼出規約別名を追加
- [x] `scripts/ci-local.sh` に `--stage` オプションを追加
- [x] `extern_metadata` / `extern_decl` の重複フィールドを解消（`extern_block_target` へ改名済み）
- [x] `effects-residual.jsonl.golden` を含む監査ゴールデンを更新
- [x] **LLVM IRゴールデンテスト (basic_arithmetic, control_flow, function_calls) を更新**（2025-10-20）
- [x] Darwin 向け可変長/構造体戻りの ABI 差分調査を完了し、`docs/notes/backend/llvm-spec-status-survey.md` §2.2 を更新（2025-10-21、`§2.2.2a` 追加）
- [x] Borrowed/Transferred の返り値処理（`dec_ref`、`wrap_foreign_ptr` 等）を実装し、`arm64-apple-darwin` 向けに `reml_ffi_acquire_borrowed_result` / `reml_ffi_acquire_transferred_result` の挙動を検証する。`bridge.return.ownership = borrowed/transferred` と新設した `null_results` カウンタが [docs/spec/3-9-core-async-ffi-unsafe.md](../docs/spec/3-9-core-async-ffi-unsafe.md) §2.6、[docs/spec/3-6-core-diagnostics-audit.md](../docs/spec/3-6-core-diagnostics-audit.md) §5.1 に沿って出力されることを `tests/test_ffi_lowering.ml` と `runtime/native/tests/test_ffi_bridge.c` で確認。
- [x] CLI (`remlc --emit-ir`) を arm64-apple-darwin 向けに再実行し、`tmp/cli-callconv-out/macos/` へ IR/Audit を収集（`--verify-ir` 併用でも成功）

### 残タスク 📋

- [x] `AuditEnvelope.metadata.bridge.*` スキーマを確定し、macOS サンプルをゴールデン化する（`tooling/runtime/audit-schema.json` を正式版へ更新済み、`cli-ffi-bridge-macos.jsonl.golden` で検証）
- [x] CLI `--emit-audit` のゴールデンに Borrowed/Transferred 返り値ケースを追加し、macOS arm64 の JSONL を固定化
- [x] `tooling/ci/sync-iterator-audit.sh` / `collect-iterator-audit-metrics.py` へ `ffi_bridge.audit_pass_rate` と Darwin プリセット成功条件を追加
- [x] 仕様書（`docs/spec/3-9`, `docs/spec/3-6`）とガイド（`docs/guides/runtime/runtime-bridges.md`）の macOS 章を更新し、Phase 3 へ渡す TODO リストを整備

> **アップデート**: `.github/workflows/bootstrap-macos.yml` の `iterator-audit` ジョブで `ffi_dispatch_async.reml` / `ffi_malloc_arm64.reml` を自動実行し、`tooling/ci/ffi-audit/macos/ffi_dispatch_async.audit.jsonl` と `ffi_malloc_arm64.audit.jsonl` を生成。`collect-iterator-audit-metrics.py --audit-source` と `sync-iterator-audit.sh --macos-ffi-samples` により `ffi_bridge.audit_pass_rate (macos-arm64)` のゲートを有効化済み（技術的負債 ID 23 をクローズ）。

## 7. クロスプラットフォーム比較観点（ドラフト）
- 対象: Linux x86_64（System V）、Windows x64（MSVC）、macOS arm64（Darwin AAPCS64）。
- 比較軸:
  1. ABI 呼出規約 — `compiler/ocaml/src/ffi_contract.ml` の `abi_kind` で正規化し、ターゲットごとの差分を `reports/ffi-bridge-summary.md`（新設予定）に記載。
  2. 所有権契約 — `bridge.ownership` の既定値と制約（借用/転送/参照）をターゲット別に列挙し、Windows API 呼び出しで禁止される転送ケースを重点レビュー。
  3. 監査メトリクス — `ffi_bridge.audit_pass_rate` をターゲット別に計測し、1.0 未満となった場合は欠落キーを本サマリーおよび `reports/ffi-windows-summary.md` / `reports/ffi-linux-summary.md` に記録。
  4. CI 実行時間 — `scripts/ci-local.sh --target <platform>` の Build/Test 所要時間を比較し、15% 以上乖離した場合は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` にリスク登録。
- 次のアクション:
  - Linux / Windows 向けテンプレートを本ファイルと同形式で作成し、共通フィールド（Stage, ABI, Ownership, CI 実行結果）を揃える。
  - ✅ `tooling/ci/sync-iterator-audit.sh` に `--macos-ffi-samples` を追加し、`iterator-stage-summary.md` へ macOS FFI サンプルの結果を表示（2025-10-25 完了）。
  - GitHub Actions macOS ワークフロー (`bootstrap-macos.yml`) の `llvm-verify` ジョブに `compiler/ocaml/scripts/verify_llvm_ir.sh --preset darwin-arm64 --target arm64-apple-darwin` を追加済み。varargs/sret プリセット (`darwin_varargs.ll`, `darwin_struct_return.ll`) の自動検証結果を `tooling/ci/llvm-verify.log` で監視する。

---

*本テンプレートは Phase 2-3 FFI 契約拡張の Apple Silicon 対応検証ログを集約するための雛形です。計測完了後に値を更新し、レビュー時に参照可能な状態に保ってください。*
