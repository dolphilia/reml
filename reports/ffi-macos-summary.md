# FFI macOS (arm64) 計測サマリー（ドラフト）

> 更新日: 2025-10-19  
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
  - `./scripts/ci-local.sh --target macos --arch arm64 --stage beta`（Lint ステップでフォーマット差分が発生し停止。詳細は §2 を参照）
  - `compiler/ocaml/scripts/verify_llvm_ir.sh --target arm64-apple-darwin compiler/ocaml/tests/llvm-ir/golden/basic_arithmetic.ll`

## 2. Capability / Stage 検証
| チェック項目 | 結果 | ログ/参照 |
|--------------|------|-----------|
| `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` | 成功（2025-10-18T03:23:33Z） | `reports/runtime-capabilities-validation.json`（`runtime_candidates` に `arm64-apple-darwin` を確認） |
| `./scripts/ci-local.sh --target macos --arch arm64 --stage beta` | 失敗（Lint フォーマット差分で停止） | `dune build @fmt --auto-promote` を未実行。`src/llvm_gen/dune`・`tests/test_ffi_contract.ml` ほかで差分 |
| `compiler/ocaml/scripts/verify_llvm_ir.sh --target arm64-apple-darwin compiler/ocaml/tests/llvm-ir/golden/basic_arithmetic.ll` | 成功 | `.bc`/`.o` 生成ログを §2.2 に記録 |
| Capability 差分レビュー | 進行中 | `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `ffi_bridge.audit_pass_rate` を追記済み（Diagnostics チームレビュー待ち） |

### 2.1 監査ログ抜粋
- `AuditEnvelope.metadata.bridge.*`（arm64-apple-darwin）: `ci-local` が Lint で停止したため今回は取得できず。Typer `extern_metadata` → `AuditEnvelope` 伝搬は従来どおり次回計測で確認予定。
- `Diagnostic.extensions.effect.stage_trace`: `effects-residual` ゴールデン更新後は Typer/Runtime の `stage_trace` が一致（`compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden` を 2025-10-18 に更新）

### 2.2 実行ログ抜粋

```text
$ ./scripts/ci-local.sh --target macos --arch arm64 --stage beta
[INFO] ホストアーキテクチャ: arm64
[INFO] ターゲットプラットフォーム: macos
[INFO] Lint ステップ (1/5)
[INFO] コードフォーマットをチェック中...
File "src/llvm_gen/dune", line 1, characters 0-0:
diff --git a/_build/default/src/llvm_gen/dune b/_build/default/src/llvm_gen/.formatted/dune
...
[ERROR] フォーマットチェックに失敗しました。'dune build @fmt --auto-promote' を実行してください。

$ compiler/ocaml/scripts/verify_llvm_ir.sh --target arm64-apple-darwin compiler/ocaml/tests/llvm-ir/golden/basic_arithmetic.ll
/opt/homebrew/Library/Homebrew/cmd/shellenv.sh: line 18: /bin/ps: Operation not permitted
[1/3] llvm-as ... ✓
[2/3] opt -verify ... ✓
[3/3] llc ... ✓
生成物: compiler/ocaml/tests/llvm-ir/golden/basic_arithmetic.{bc,o}
```

## 3. ABI / 呼出規約検証
| テストケース | 概要 | 結果 | 備考 |
|--------------|------|------|------|
| `ffi_malloc_arm64.reml` | System V → Darwin 引数マーシャリング比較 | 未実施（Build 失敗） | struct-by-value / pointer 戻り値 |
| `ffi_dispatch_async.reml` | `dispatch_async_f` 呼び出し（libSystem） | 未実施（Build 失敗） | `ffi.bridge` Capability Required |
| 可変長引数 (`printf`) | `ffi.callconv("ccc")` → varargs 挙動 | 未実施（Build 失敗） | Darwin target 固有 |
| 構造体戻り値 | `extern` returning `{f64, f64}` | 未実施（Build 失敗） | SRet 要否確認 |

### 3.1 LLVM IR スナップショット
- `build/ir/macos-arm64/<sample>.ll` … TBD
- `llc` 出力オブジェクト: TBD（`codesign --verify` 実行ログ）

## 4. 所有権契約 / 監査
| テスト | 内容 | 結果 | 備考 |
|--------|------|------|------|
| `ffi_borrowed_pointer.reml` | `@ffi_ownership("borrowed")` の解析 | 未実施（Typer 実装待ち） | 監査ログ `bridge.ownership = borrowed` 期待 |
| `ffi_transferred_pointer.reml` | `Ownership::Transferred` | 未実施（Typer 実装待ち） | RC インクリメント挙動確認 |
| 診断 (`ffi.contract.missing`) | 不足注釈の検証 | 未実施（Typer 実装待ち） | CLI JSON & Audit の整合 |

## 5. パフォーマンス指標
| メトリクス | 指標 | 現状値 | 備考 |
|------------|------|--------|------|
| `ffi_bridge_call_latency_ns` | 1 回あたりの平均呼出時間 | 未計測 | `bench/ffi_dispatch_async.txt` |
| `ffi_stub_codegen_time_ms` | LLVM IR → object 生成時間 | 未計測 | `metrics/ffi_codegen.json` |

## 6. TODO / リスク
- [x] Capability override (`arm64-apple-darwin`) を `tooling/runtime/capabilities/default.json` に追加し、Windows 同等セットから開始する。（PR 化・レビューは未完）
- [ ] `dune build @fmt --auto-promote` を実行して `src/llvm_gen/dune`・`tests/test_ffi_contract.ml` などのフォーマット差分を解消する。
- [ ] フォーマット差分解消後に `scripts/ci-local.sh --target macos --arch arm64 --stage beta` を再実行し、Lint → Runtime まで完走したログを保存する。
- [ ] Darwin 向け可変長/構造体戻りの ABI 差分調査を完了し、`docs/notes/llvm-spec-status-survey.md` §2.2 を更新。
- [ ] `AuditEnvelope.metadata.bridge.*` スキーマを確定し、macOS サンプルをゴールデン化する（ドラフトは `tooling/runtime/audit-schema.json` に追加済み、Typer 実装後に本番値を取得）。
- [x] `scripts/ci-local.sh` に `--stage` オプションを追加し、Diagnostics チームの運用手順と合わせる。
- [x] `extern_metadata` / `extern_decl` の重複フィールドを解消し、Build ステップを通過させる（`extern_block_target` へ改名済み）。
- [x] `effects-residual.jsonl.golden` を含む監査ゴールデンを更新し、テストステップを再度有効化する（2025-10-18、`dune runtest` / `scripts/ci-local.sh` で検証）。

## 7. クロスプラットフォーム比較観点（ドラフト）
- 対象: Linux x86_64（System V）、Windows x64（MSVC）、macOS arm64（Darwin AAPCS64）。
- 比較軸:
  1. ABI 呼出規約 — `compiler/ocaml/src/ffi_contract.ml` の `abi_kind` で正規化し、ターゲットごとの差分を `reports/ffi-bridge-summary.md`（新設予定）に記載。
  2. 所有権契約 — `bridge.ownership` の既定値と制約（借用/転送/参照）をターゲット別に列挙し、Windows API 呼び出しで禁止される転送ケースを重点レビュー。
  3. 監査メトリクス — `ffi_bridge.audit_pass_rate` をターゲット別に計測し、1.0 未満となった場合は欠落キーを本サマリーおよび `reports/ffi-windows-summary.md` / `reports/ffi-linux-summary.md` に記録。
  4. CI 実行時間 — `scripts/ci-local.sh --target <platform>` の Build/Test 所要時間を比較し、15% 以上乖離した場合は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` にリスク登録。
- 次のアクション:
  - Linux / Windows 向けテンプレートを本ファイルと同形式で作成し、共通フィールド（Stage, ABI, Ownership, CI 実行結果）を揃える。
  - `tooling/ci/sync-iterator-audit.sh` に FFI ブリッジサマリ出力オプションを追加し、`iterator-stage-summary.md` と同じレイアウトで `ffi-bridge-summary.md` を生成する。

---

*本テンプレートは Phase 2-3 FFI 契約拡張の Apple Silicon 対応検証ログを集約するための雛形です。計測完了後に値を更新し、レビュー時に参照可能な状態に保ってください。*
