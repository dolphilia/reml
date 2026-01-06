# 調査メモ: 第17章 FFI とネイティブ連携

## 対象モジュール

- `compiler/runtime/src/ffi/mod.rs`
- `compiler/runtime/src/ffi/dsl/mod.rs`
- `compiler/runtime/src/native/mod.rs`
- `compiler/runtime/src/embedding.rs`（埋め込み ABI と `native.embed.*` 監査）

## 入口と全体像

- `runtime/src/ffi/mod.rs` は `ffi::dsl` を公開する最小の入口で、実装の中心は `ffi/dsl/mod.rs` に集約されている。
  - `compiler/runtime/src/ffi/mod.rs:1-3`
- `ffi/dsl` は **FFI 型/シグネチャ/呼び出し/ラッパー** と **監査メタデータ** をまとめた最小ランタイム実装。動的ライブラリのロードではなく、呼び出し仕様と監査キーの整備が中心。
  - `compiler/runtime/src/ffi/dsl/mod.rs:1-805`
- `native/mod.rs` は intrinsic / inline asm / LLVM IR / embed の監査メタデータを記録するユーティリティと、簡易 intrinsic を実装する。
  - `compiler/runtime/src/native/mod.rs:1-172`
- 埋め込み ABI は `embedding.rs` にあり、C ABI エントリポイントから `native.embed.*` の監査メタデータを記録する。
  - `compiler/runtime/src/embedding.rs:1-231`

## データ構造

- **FfiType / FfiFnSig / FfiCallSpec**: 型表現・シグネチャ・MIR 由来の呼び出し仕様。
  - `compiler/runtime/src/ffi/dsl/mod.rs:24-105`
- **FfiStruct / FfiEnum / FfiRepr / FfiIntRepr**: 構造体/列挙の FFI 表現。
  - `compiler/runtime/src/ffi/dsl/mod.rs:170-219`
- **FfiLibraryHandle / FfiLibrary**: ライブラリ識別子と関数バインドの入口。
  - `compiler/runtime/src/ffi/dsl/mod.rs:221-294`
- **FfiRawFn / FfiWrappedFn**: 低レベル呼び出しと安全ラッパー。
  - `compiler/runtime/src/ffi/dsl/mod.rs:296-471`
- **FfiWrapSpec / Ownership**: ラッパー設定と所有権表現。
  - `compiler/runtime/src/ffi/dsl/mod.rs:381-406`
- **FfiValue**: FFI への入力値表現（ポインタ/構造体/列挙/関数ポインタを含む）。
  - `compiler/runtime/src/ffi/dsl/mod.rs:566-623`
- **FfiError / FfiErrorKind**: エラー種別と Guard 診断への変換。
  - `compiler/runtime/src/ffi/dsl/mod.rs:648-713`
- **native の監査キー**: `native.intrinsic.*` / `native.inline_asm.*` / `native.llvm_ir.*` / `native.embed.*` を生成するヘルパ。
  - `compiler/runtime/src/native/mod.rs:18-126`
- **RemlEmbedContext / RemlEmbedStatus**: 埋め込み ABI で共有される最小コンテキスト。
  - `compiler/runtime/src/embedding.rs:20-35`

## コアロジック

- **FFI 型パース**: `parse_mir_ffi_type` が MIR 由来の文字列型（`&mut T` や `[T]`）を `FfiType` に正規化する。スライスはポインタへ降格する。
  - `compiler/runtime/src/ffi/dsl/mod.rs:107-167`
- **ライブラリ解決とバインド**: `bind_library` と `FfiLibrary::bind_fn` がシンボル/シグネチャを束ねた `FfiRawFn` を生成する。
  - `compiler/runtime/src/ffi/dsl/mod.rs:258-294`
- **呼び出し経路**: `FfiRawFn::call` は `call_handler` があればそれを優先し、なければ `FFI_CALL_EXECUTOR` に委譲する。未登録時は `ffi.call.executor_missing` を返す。
  - `compiler/runtime/src/ffi/dsl/mod.rs:331-342`
- **監査付き呼び出し**: `call_with_audit` と `insert_call_audit_metadata` が `ffi.call` のメタデータを構築する。
  - `compiler/runtime/src/ffi/dsl/mod.rs:345-368`
  - `compiler/runtime/src/ffi/dsl/mod.rs:773-805`
- **安全ラッパー**: `FfiWrappedFn::call` が引数/戻り値の型一致、`null_check`、`ownership` を検証し、違反時は `FfiError` を生成する。
  - `compiler/runtime/src/ffi/dsl/mod.rs:438-505`
- **監査メタデータ**: `insert_wrapper_audit_metadata` と `mark_call_wrapper` がラッパー情報を付与する。
  - `compiler/runtime/src/ffi/dsl/mod.rs:724-758`
- **native intrinsic の監査**: `sqrt_f64` / `ctpop_*` / `memcpy` が `native.intrinsic.*` を記録して実処理を行う。
  - `compiler/runtime/src/native/mod.rs:128-172`
- **埋め込み ABI の監査**: `record_embed_audit` が `insert_embed_entrypoint_audit_metadata` を通じて `native.embed.*` を生成する。
  - `compiler/runtime/src/embedding.rs:214-230`

## エラー処理

- **FFI 署名/引数/所有権の検証エラー**: `FfiErrorKind` と診断コード (`ffi.signature.invalid`, `ffi.wrap.*`) が `GuardDiagnostic` に落ちる。
  - `compiler/runtime/src/ffi/dsl/mod.rs:648-713`
  - `compiler/runtime/src/ffi/dsl/mod.rs:508-551`
- **実行エンジン未登録**: `FfiRawFn::call` が `ffi.call.executor_missing` を返す。
  - `compiler/runtime/src/ffi/dsl/mod.rs:331-342`
- **埋め込み ABI エラー**: `RemlEmbedStatus` が ABI 不一致/未対応ターゲット/無効引数を列挙する。
  - `compiler/runtime/src/embedding.rs:20-87`

## 仕様との対応メモ

- FFI / Unsafe / Async の仕様は `docs/spec/3-9-core-async-ffi-unsafe.md` が対象。実装はそのうち FFI 呼び出しと監査メタデータの最小セットに集中している。
- `native.embed.*` の監査キーは `3-9` と `3-6`（診断監査）双方に跨るため、章末で整理する必要がある。

## TODO / 不明点

- `ffi/dsl` は動的ライブラリのロードや ABI 互換性検証を持たないため、`docs/spec/3-9` の FFI Capability と対応づける際に「未実装/最小実装」の線引きを明記する必要がある。
- `FfiCallExecutor` がどのレイヤで登録される想定か、`runtime` 側の利用例が不足しているため導線を確認したい。
- `native.embed.*` の監査キーが `AuditEnvelope::validate` の必須キーに含まれるかは別章と突き合わせが必要。
