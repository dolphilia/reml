# FFI Linux (x86_64) 計測サマリー（テンプレート）

> 更新日: <!-- YYYY-MM-DD -->  
> 対象: Phase 2-3 FFI 契約拡張（System V ABI 検証）

## 1. 計測環境
- ハードウェア: <!-- 例: AMD EPYC / Intel Xeon, RAM -->
- OS / Toolchain:
  - Ubuntu 22.04 LTS / Debian 12 等
  - LLVM 18.x（`/usr/lib/llvm-18`） / Clang 18.x
  - OCaml 5.2.1 / dune 3.x
- Reml リポジトリ commit: `<!-- git rev-parse HEAD -->`
- 実行コマンド:
  - `./scripts/ci-local.sh --target linux --arch x86_64 --stage beta`
  - `compiler/ocaml/scripts/verify_llvm_ir.sh --target x86_64-unknown-linux-gnu <sample.ll>`

## 2. Capability / Stage 検証
| チェック項目 | 結果 | ログ/参照 |
|--------------|------|-----------|
| `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` | <!-- 成功/失敗 --> | `reports/runtime-capabilities-validation.json` |
| `./scripts/ci-local.sh --target linux --arch x86_64 --stage beta` | <!-- 成功/失敗 --> | `reports/ffi-linux-summary.md` §2.2 |
| `compiler/ocaml/scripts/verify_llvm_ir.sh --target x86_64-unknown-linux-gnu ...` | <!-- 成功/失敗 --> | LLVM IR / object 生成ログ |
| Capability 差分レビュー | <!-- 進行中/完了 --> | `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` |

### 2.1 監査ログ抜粋
- `AuditEnvelope.metadata.bridge.*`（System V）: <!-- 例: `bridge.platform=linux-x86_64`, `bridge.callconv=ccc`, `bridge.abi=system_v` -->
- `ffi_bridge.audit_pass_rate`: <!-- 記録値、`tooling/ci/collect-iterator-audit-metrics.py` 実行結果 -->

### 2.2 実行ログ抜粋

```text
$ ./scripts/ci-local.sh --target linux --arch x86_64 --stage beta
[INFO] ...
[SUCCESS] ...

$ compiler/ocaml/scripts/verify_llvm_ir.sh --target x86_64-unknown-linux-gnu <sample.ll>
[1/3] llvm-as ...
[2/3] opt -verify ...
[3/3] llc ...
```

## 3. ABI / 呼出規約検証
| テストケース | 概要 | 結果 | 備考 |
|--------------|------|------|------|
| `ffi_systemv_struct.reml` | System V struct-by-value 呼出し | <!-- --> | <!-- --> |
| `ffi_varargs_printf.reml` | 可変長引数 (`ccc`) | <!-- --> | <!-- --> |
| `ffi_errno_capture.reml` | `ffi.bridge` + errno 共有 | <!-- --> | <!-- --> |

## 4. 所有権契約 / メトリクス
| テスト | 内容 | 結果 | 備考 |
|--------|------|------|------|
| `ffi_borrowed_pointer.reml` | `Ownership::Borrowed` | <!-- --> | `reml_ffi_bridge_record_success` で RC + 監査確認 |
| `ffi_transferred_pointer.reml` | `Ownership::Transferred` | <!-- --> | `reml_ffi_bridge_record_failure` を含む失敗時の挙動を計測 |
| ランタイム計測 API | `reml_ffi_bridge_get_metrics` / `reml_ffi_bridge_pass_rate` | <!-- --> | `runtime/native/src/ffi_bridge.c` 実装を利用して CI 指標を取得 |

## 5. TODO / リスク
- [ ] System V スタブの LLVM IR を `reml.bridge.stubs` メタデータに追加し、`reports/ffi-bridge-summary.md` の Linux 行を更新。
- [ ] 監査ログのゴールデン (`compiler/ocaml/tests/golden/audit/ffi-bridge-linux.jsonl`) を作成し、`ffi_bridge.audit_pass_rate` を 1.0 で固定。
- [ ] `tooling/ci/sync-iterator-audit.sh` に Linux ログ収集テンプレートを統合し、CI アーティファクトへ保存。
- [ ] 所有権違反時の診断 (`ffi.contract.ownership_mismatch`) が CLI/Audit 両方で揃うかを確認。

---

*本テンプレートは Linux x86_64 向け FFI 検証の記録用フォーマットです。計測完了後に各項目を更新し、監査ログおよび CI レポートと整合させてください。*
