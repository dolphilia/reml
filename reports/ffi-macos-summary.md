# FFI macOS (arm64) 計測サマリー（ドラフト）

> 更新日: 2025-10-18  
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
  - `REMLC_EFFECT_STAGE=beta scripts/ci-local.sh --target macos --arch arm64`（Lint ステップで停止）
  - `REMLC_EFFECT_STAGE=beta scripts/ci-local.sh --target macos --arch arm64 --skip-lint`（Build ステップで停止）
  - `scripts/verify_llvm_ir.sh --target arm64-apple-darwin`（未実行・Build 成功後に実施予定）
  - `llc -mtriple=arm64-apple-darwin`（未実行・IR 検証時に実施予定）

## 2. Capability / Stage 検証
| チェック項目 | 結果 | ログ/参照 |
|--------------|------|-----------|
| `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` | 成功（2025-10-18T03:23:33Z） | `reports/runtime-capabilities-validation.json`（`runtime_candidates` に `arm64-apple-darwin` を確認） |
| `REMLC_EFFECT_STAGE=beta scripts/ci-local.sh --target macos --arch arm64` | 失敗（Lint ステップ `dune fmt` 差分） | ログ抜粋を §2.2 に記録（`_build/default/src/*.formatted` 差分） |
| `REMLC_EFFECT_STAGE=beta scripts/ci-local.sh --target macos --arch arm64 --skip-lint` | 失敗（Build ステップ `extern_target` 重複） | `src/ast.ml` 重複フィールド警告を §2.2 に記録 |
| Capability 差分レビュー | 未実施 | `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` 更新案（診断チーム確認待ち） |

### 2.1 監査ログ抜粋
- `AuditEnvelope.metadata.bridge.*`（arm64-apple-darwin）: Build 失敗のため未取得（Typer `extern_metadata` 実装待ち）
- `Diagnostic.extensions.effect.stage_trace` 差分: 同上

### 2.2 実行ログ抜粋

```text
$ REMLC_EFFECT_STAGE=beta scripts/ci-local.sh --target macos --arch arm64
[INFO] Lint ステップ (1/5)
diff --git a/_build/default/src/cli/dune b/_build/default/src/cli/.formatted/dune
...
[ERROR] フォーマットチェックに失敗しました。'dune build @fmt --auto-promote' を実行してください。

$ REMLC_EFFECT_STAGE=beta scripts/ci-local.sh --target macos --arch arm64 --skip-lint
[INFO] Build ステップ (2/5)
File "src/ast.ml", line 291, characters 2-32:
Error (warning 30 [duplicate-definitions]): the label extern_target is defined in both types extern_metadata and extern_decl.
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
- [ ] `scripts/ci-local.sh` の `--arch arm64` 実行ログを保存し、CI 再現性を確認する（`--stage` 引数実装後に再試行）。
- [ ] Darwin 向け可変長/構造体戻りの ABI 差分調査を完了し、`docs/notes/llvm-spec-status-survey.md` §2.2 を更新。
- [ ] `AuditEnvelope.metadata.bridge.*` スキーマを確定し、macOS サンプルをゴールデン化する。
- [ ] `scripts/ci-local.sh` に `--stage` オプションを追加し、Diagnostics チームの運用手順と合わせる。
- [ ] `extern_metadata` / `extern_decl` の重複フィールドを解消し、Build ステップを通過させる。

---

*本テンプレートは Phase 2-3 FFI 契約拡張の Apple Silicon 対応検証ログを集約するための雛形です。計測完了後に値を更新し、レビュー時に参照可能な状態に保ってください。*
