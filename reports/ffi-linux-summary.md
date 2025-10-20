# FFI Linux (x86_64) 監査サマリー（ドラフト）

> 更新日: 2025-10-24  
> 対象: Phase 2-3 FFI 契約拡張（Linux デフォルトターゲット）

## 1. 実行環境とコマンド
- ホスト: macOS (Apple Silicon) 上の開発環境  
  ※ Linux ターゲット向け CLI 検証を `x86_64-unknown-linux-gnu` で実施。
- コンパイラ: `dune exec -- remlc`（OCaml 実装）
- 実行コマンド:
  ```bash
  dune exec -- remlc \
    tests/samples/ffi/cli-callconv-sample.reml \
    --emit-ir \
    --emit-audit reports/tmp/linux/cli-callconv.audit.jsonl \
    --out-dir reports/tmp/linux \
    --runtime-capabilities tooling/runtime/capabilities/default.json \
    --verify-ir
  ```
- `--verify-ir` は stub エントリ無終端バグ修正（`llvm_gen/codegen.ml` 2025-10-24）後に成功。

## 2. 監査ログ（`ffi.bridge`）

| extern_name        | target                     | callconv | ownership    | return.status      | bridge.platform    |
|--------------------|----------------------------|----------|--------------|--------------------|--------------------|
| `ffi_macos_probe`  | `arm64-apple-darwin`       | aarch64  | borrowed     | wrap               | `macos-arm64`      |
| `ffi_win_probe`    | `x86_64-pc-windows-msvc`   | win64    | transferred  | wrap_and_release   | `windows-msvc-x64` |

- 監査ログは `compiler/ocaml/tests/golden/audit/cli-ffi-bridge-linux.jsonl.golden` に固定。  
- いずれのエントリも `bridge.return.{wrap,release_handler,rc_adjustment}` を出力し、`ffi_bridge.audit_pass_rate` の必須キーを満たす。

## 3. LLVM IR スナップショット
- ゴールデン: `compiler/ocaml/tests/golden/ffi/cli-linux.ll.golden`
- 特記事項:
  - `__reml_stub_ffi_win_probe_1`（callconv=win64）、`__reml_stub_ffi_macos_probe_2`（callconv=aarch64_aapcscc）を含む。
  - Named metadata `!reml.bridge.stubs` に `bridge.platform`, `bridge.return.*`, Darwin Register Save Area 情報を埋め込み。
  - メイン関数は `ret i64 0` のシンプルなプレースホルダ。CLI から生成される IR と一致。

## 4. メトリクス状況
- `ffi_bridge.audit_pass_rate` は `collect-iterator-audit-metrics.py` のデフォルト計測対象に追加済み。
- `tooling/ci/sync-iterator-audit.sh` で pass_rate < 1.0 または欠落キーが検出された場合に CI を失敗させる。
- Linux ワークフロー（`bootstrap-linux.yml`）で `--source` 引数に FFI 診断ゴールデンを指定し、メトリクス収集を自動化。

## 5. フォローアップ
1. `reports/ffi-bridge-summary.md` と連動する形で Windows/macOS 含む 3 ターゲット分のサマリーを随時更新。
2. CI で生成した `cli-callconv` 監査ログをアーティファクト化し、`reports/ffi-linux-summary.md` の表と差分を比較するスクリプトを追加予定。
3. `docs/spec/3-9-core-async-ffi-unsafe.md` に `bridge.return.*` / `bridge.platform` の定義とサンプルを反映（Phase 2-3 TODO）。
