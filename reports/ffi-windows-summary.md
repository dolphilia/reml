# FFI Windows (x86_64-pc-windows-msvc) 監査サマリー（ドラフト）

> 更新日: 2025-10-24  
> 対象: Phase 2-3 FFI 契約拡張（Windows ターゲット）

## 1. 実行環境とコマンド
- ホスト: macOS (Apple Silicon) - Windows ターゲット指定で CLI を実行。
- コマンド:
  ```bash
  dune exec -- remlc \
    tests/samples/ffi/cli-callconv-sample.reml \
    --emit-ir \
    --emit-audit reports/tmp/windows/cli-callconv.audit.jsonl \
    --out-dir reports/tmp/windows \
    --runtime-capabilities tooling/runtime/capabilities/default.json \
    --target x86_64-pc-windows-msvc \
    --verify-ir
  ```
- `--verify-ir` は stub エントリ修正後に成功。Windows callconv=win64 の IR を Linux 環境で生成・検証できることを確認。

## 2. 監査ログ（`ffi.bridge`）

| extern_name        | target                     | callconv | ownership    | return.status      | bridge.platform    |
|--------------------|----------------------------|----------|--------------|--------------------|--------------------|
| `ffi_macos_probe`  | `arm64-apple-darwin`       | aarch64  | borrowed     | wrap               | `macos-arm64`      |
| `ffi_win_probe`    | `x86_64-pc-windows-msvc`   | win64    | transferred  | wrap_and_release   | `windows-msvc-x64` |

- ゴールデン: `compiler/ocaml/tests/golden/audit/cli-ffi-bridge-windows.jsonl.golden`  
- 監査ログ内の `bridge.platform` と `expected_abi` は Capability Registry (`tooling/runtime/capabilities/default.json`) と一致。
- PowerShell 版サマリー (`tooling/ci/Sync-AuditMetrics.ps1`) でも `ffi_bridge.audit_pass_rate = 1.0` を確認。

## 3. LLVM IR スナップショット
- ゴールデン: `compiler/ocaml/tests/golden/ffi/cli-windows.ll.golden`
- 主要確認点:
  - `__reml_stub_ffi_win_probe_1` が `win64cc` を使用し、`reml_ffi_bridge_record_status` を呼び出す。
  - `!reml.bridge.stubs` に Windows 向けメタデータ（`bridge.callconv=win64`, `bridge.platform=windows-msvc-x64`）を保持。
  - macOS 向けの register save area 情報も同一モジュール内に保持し、クロスプラットフォーム stub 成功を一括検証。

## 4. CI 連携
- Linux ワークフローの `collect-iterator-audit-metrics.py` で FFI 診断ゴールデンを解析し、`ffi_bridge.audit_pass_rate` を算出。
- Windows ワークフロー（`bootstrap-windows.yml`）で PowerShell スクリプトを実行し、Linux 生成のメトリクスと `llvm-verify.log` を用いて差分を監視。
- `Sync-AuditMetrics.ps1` は pass_rate < 1.0 または Stage トレース不一致時に非ゼロ終了し、GitHub Actions を失敗させる。

## 5. 今後の作業
1. Windows ネイティブ環境での `remlc --emit-ir` 実行（MSYS2/LLVM 16）を自動化し、サマリー生成を完全に Windows ランナーへ移行。
2. `ffi_bridge.audit_pass_rate` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の定例メトリクスに追加し、pass_rate 低下時の対応フローを整理。
3. Stage override (`overrides.x86_64-pc-windows-msvc`) の検証ログを CI アーティファクト化し、`reports/ffi-windows-summary.md` から直接参照できるようにする。
