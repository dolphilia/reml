# Reml ランタイム FFI ラッパ crate

このクレートは `runtime/native/include/reml_runtime.h` で定義された C API を Rust 側 (`compiler/rust/runtime/ffi`) で再現し、所有権付き wrappers（`ForeignPtr`）と `AuditEnvelope.metadata.bridge` 連携を提供します。

## 実装の構成

- `ForeignPtr`：`inc_ref`/`dec_ref` を `Clone`/`Drop` で自動化したハンドル。
- `ReMlString`：C 構造体 `{ ptr, i64 }` を `repr(C)` で再現し、UTF-8 変換を補助。
- `BridgeStatus`：`reml_ffi_bridge_record_status` に送るステータス列挙。
- `runtime_panic`/`print_i64_debug`：`panic`/`print_i64` への安全ラッパー。
- `acquire_borrowed_result`/`acquire_transferred_result`：`reml_ffi_*` ブリッジを Rust 側所有権に接続。

## bindgen と照合するための手順

1. `bindgen` バイナリ（`cargo install bindgen`）が利用可能なことを確認。
2. リポジトリルートの `scripts` ディレクトリにある `generate-runtime-ffi-bindings.sh` を実行すると、`bindings-bindgen.rs`（自動生成）を出力します。
3. `bindings-bindgen.rs` と `src/lib.rs` の `extern` 宣言を比較し、型やシグネチャの差異をドキュメント化してください（`panic` の FAT pointer など）。

## スモークテストの期待

- Rust 側で `ForeignPtr::allocate_payload` → `inc_ref`/`dec_ref` → `record_bridge_status` → `runtime_panic` を順に呼び出すテストを `ffi_signature_smoke` として用意します（計画段階）。
- `effects` チェック付きビルドでは `effect {ffi, audit, unsafe}` を保持した呼び出しシーケンスを構築し、`docs/plans/rust-migration/2-1-runtime-integration.md` の `effect` ルールと整合させます。

## 監査ログとの連携

`record_bridge_status` は `reml_ffi_bridge_record_status` への直送ですが、`audit.log("ffi.call.*")` を補完するラッパーは別途 `crate` もしくは `caps` モジュールとして追加します。ドキュメント更新は `docs/spec/3-6-core-diagnostics-audit.md` を参照してください。
