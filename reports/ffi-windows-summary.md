# FFI Windows (x64 / MSVC) 計測サマリー（テンプレート）

> 更新日: <!-- YYYY-MM-DD -->  
> 対象: Phase 2-3 FFI 契約拡張（MSVC ABI 検証）

## 1. 計測環境
- ハードウェア: <!-- 例: Azure Dsv5, 8vCPU/32GB -->
- OS / Toolchain:
  - Windows Server 2022 (x64)
  - Visual Studio Build Tools 2022 / LLVM 18.x for Windows
  - OCaml 5.2.1 + dune（WSL または MSYS2）
- Reml リポジトリ commit: `<!-- git rev-parse HEAD -->`
- 実行コマンド（PowerShell 推奨）:
  - `.\scripts\ci-local.ps1 -Target windows -Stage beta`
  - `compiler\ocaml\scripts\verify_llvm_ir.ps1 -Target x86_64-pc-windows-msvc <sample.ll>`

## 2. Capability / Stage 検証
| チェック項目 | 結果 | ログ/参照 |
|--------------|------|-----------|
| `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json --cli-stage beta` | <!-- --> | `reports/runtime-capabilities-validation.json` |
| Windows CI ジョブ (`windows-latest`) での `sync-iterator-audit.ps1` | <!-- --> | `reports/ffi-windows-summary.md` §2.2 |
| `verify_llvm_ir.ps1` (MSVC) | <!-- --> | `basic_arithmetic.ll` などの検証ログ |
| Capability override (`x86_64-pc-windows-msvc`) | <!-- --> | `tooling/runtime/capabilities/default.json` |

### 2.1 監査ログ抜粋
- `bridge.platform=windows-msvc-x64` / `bridge.callconv=win64` / `bridge.abi=msvc` を確認。
- `ffi_bridge.audit_pass_rate`: <!-- 例: 1.0（`tooling/ci/collect-iterator-audit-metrics.py` から取得） -->

### 2.2 実行ログ抜粋

```text
PS> .\scripts\ci-local.ps1 -Target windows -Stage beta
[INFO] ...
[SUCCESS] ...

PS> compiler\ocaml\scripts\verify_llvm_ir.ps1 -Target x86_64-pc-windows-msvc <sample.ll>
[1/3] llvm-as ...
[2/3] opt -verify ...
[3/3] llc ...
```

## 3. ABI / 呼出規約検証
| テストケース | 概要 | 結果 | 備考 |
|--------------|------|------|------|
| `ffi_stdcall_bridge.reml` | `@ffi_callconv("win64")` のスタブ生成 | <!-- --> | <!-- --> |
| `ffi_struct_return.reml` | MSVC struct-return (`sret`) | <!-- --> | <!-- --> |
| `ffi_widechar.reml` | `MessageBoxW` 呼び出し | <!-- --> | WideChar / UTF-16 マーシャリング |

## 4. 所有権契約 / メトリクス
| テスト | 内容 | 結果 | 備考 |
|--------|------|------|------|
| `ffi_transferred_pointer.reml` | `Ownership::Transferred` の RC/監査確認 | <!-- --> | `reml_ffi_bridge_record_status` (failure/success) を併用 |
| `ffi_reference_pointer.reml` | `Ownership::Reference` | <!-- --> | 参照カウント非操作パス確認 |
| ランタイム指標 | `reml_ffi_bridge_get_metrics` / `reml_ffi_bridge_pass_rate` | <!-- --> | CI で JSON 出力に変換予定 |

## 5. TODO / リスク
- [ ] Windows 用スタブの LLVM IR を `reml.bridge.stubs` に追加し、監査ログ差分をゴールデン化。
- [ ] PowerShell 版 `sync-iterator-audit.ps1` を整備し、`ffi_bridge.audit_pass_rate` を Windows CI へ統合。
- [ ] `ffi_bridge.c` のメトリクスと CI スクリプトを連携させ、失敗時に PR チェックを失敗させる。
- [ ] Win32 API 呼び出しでの WideChar / エンコーディング差異を `docs/spec/3-9-core-async-ffi-unsafe.md` に追記。

---

*本テンプレートは Windows x64 (MSVC) 向け FFI 検証ログを整理するための雛形です。計測後に値を更新し、`reports/ffi-bridge-summary.md` と照らし合わせて整合性を保ってください。*
