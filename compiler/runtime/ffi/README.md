# Reml ランタイム FFI ラッパ crate

このクレートは `compiler/runtime/native/include/reml_runtime.h` で定義された C API を Rust 側 (`compiler/runtime/ffi`) で再現し、所有権付き wrappers（`ForeignPtr`）と `AuditEnvelope.metadata.bridge` 連携を提供します。

## 実装の構成

- `ForeignPtr`：`inc_ref`/`dec_ref` を `Clone`/`Drop` で自動化したハンドル。
- `ReMlString`：C 構造体 `{ ptr, i64 }` を `repr(C)` で再現し、UTF-8 変換を補助。
- `BridgeStatus`：`reml_ffi_bridge_record_status` に送るステータス列挙。
- `runtime_panic`/`print_i64_debug`：`panic`/`print_i64` への安全ラッパー。
- `acquire_borrowed_result`/`acquire_transferred_result`：`reml_ffi_*` ブリッジを Rust 側所有権に接続。
- `Span`／`RuntimeString`：ソース位置・所有権情報を伴う文字列ラッパと、`AuditEnvelope.metadata.bridge` を構築するための `BridgeAuditMetadata`。

## bindgen と照合するための手順

1. `bindgen` バイナリ（`cargo install bindgen`）が利用可能なことを確認。
2. リポジトリルートの `scripts` ディレクトリにある `generate-runtime-ffi-bindings.sh` を実行すると、`bindings-bindgen.rs`（自動生成）を出力します。
3. `bindings-bindgen.rs` と `src/lib.rs` の `extern` 宣言を比較し、型やシグネチャの差異をドキュメント化してください（`panic` の FAT pointer など）。

## スモークテストの期待

- Rust 側で `ForeignPtr::allocate_payload` → `inc_ref`/`dec_ref` → `record_bridge_status` → `runtime_panic` を順に呼び出すテストを `ffi_signature_smoke` として用意します（計画段階）。
- `effects` チェック付きビルドでは `effect {ffi, audit, unsafe}` を保持した呼び出しシーケンスを構築し、`docs/plans/rust-migration/2-1-runtime-integration.md` の `effect` ルールと整合させます。

## 監査ログとの連携

`record_bridge_status` は `reml_ffi_bridge_record_status` への直送ですが、`Span`/`RuntimeString` と `BridgeAuditMetadata` を組み合わせたメタデータを `AuditEnvelope.metadata.bridge` に埋める仕組みを補完することで、`audit.log("ffi.call.*")` との整合を取ります。`record_bridge_with_metadata` で `BridgeStatus` を渡して `bridge.status` を記録し、`BridgeAuditMetadata::as_entries()` を通じて `bridge.ownership`/`bridge.span`/`bridge.target` などをテンプレート化できます。ドキュメント更新は `docs/spec/3-6-core-diagnostics-audit.md` を参照してください。
