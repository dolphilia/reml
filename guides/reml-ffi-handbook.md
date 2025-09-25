# Reml FFI ハンドブック（ドラフト）

> 目的：Reml と外部ランタイム（C/C++/Rust/システムライブラリ等）との安全な接続方法を明文化し、[LLVM連携ノート](llvm-integration-notes.md)・`1-3-effects-safety.md`・`guides/runtime-bridges.md` に分散している知識を一本化する。

## 1. 適用範囲と位置付け
- 既定ターゲット：System V AMD64 / Windows x64。将来 ARM64 / WASM を追加予定。
- コンパイラ実装（OCaml 版／将来の self-host）とランタイム、DSL プロジェクトが共通で参照する運用ガイドとして利用。
- FFI で橋渡しする典型シナリオ：データベースドライバ、クラウド SDK、GPU ライブラリ、既存サービスとの IPC、ホットリロード可能なプラグイン。

## 2. ABI・データレイアウトの要約
- 詳細は [LLVM連携ノート](llvm-integration-notes.md) の「ターゲット ABI / データレイアウト」を参照。
- Reml から公開される構造体／列挙型は `repr(C)` 等価の自然境界を前提。
- 文字列・スライス：`{ ptr data, i64 len }`。所有権は RC、境界を超える場合は明示的に `inc_ref`/`dec_ref`。
- 例外／パニック伝播は定義しない。Reml → C 方向は `abort`、C++ 例外は外に逃さない。

## 3. 効果タグと `unsafe` 境界
- FFI 呼び出しは必ず `ffi` 効果を持ち、`unsafe {}` 内でのみ許可。
- `io.async` / `io.blocking` / `io.timer` の分類と接続：
  - 非同期ハンドオフ（libuv, io_uring 等）は `io.async`。
  - ブロッキング I/O やスレッド待機は `io.blocking`。
  - タイマー／イベント登録は `io.timer`。
- `@async_free` / `@no_blocking` / `@no_timer` を利用してラッパ API の静的保証を付与。

## 4. リンクとビルドの手順
1. ヘッダ生成：`remlc --emit-header foo.reml`（将来実装）で C 用シグネチャを生成。
2. ランタイムライブラリ：`libreml_runtime.a`（RC/メモリ/診断）をリンク。
3. プラットフォーム差異：
   - Linux: `clang foo.c foo.ll libreml_runtime.a -o foo`
   - Windows: `cl /Fe:foo.exe foo.c foo.ll libreml_runtime.lib`
4. デバッグ情報を有効化する場合は `-g` 付き LLVM IR を生成し、`lldb` / `windbg` で解析。

## 5. 所有権とライフタイム契約
- RC ベース API：
  - Reml → C：値を渡す前に `inc_ref`。C 側が保持をやめたタイミングで `reml_release_*`（生成予定）を呼ぶ契約。
  - C → Reml：C が所有するポインタは `unsafe` でラップする。Mutating callback は `ffi` + `mut` を持つ。
- ゼロコピー文字列は UTF-8 前提。書記素単位の操作は Reml 側で行い、FFI ではバイト列扱い。
- エラーハンドリング：`Result<T, Diagnostic>` 風の構造体を C 用 `struct` として提供し、失敗時は `span`／`trace_id` を含む。

## 6. 監査・可観測性
- すべての FFI 呼び出しを `audit.log("ffi.call", {...})` へ記録するテンプレートを提供。
- 収集項目例：`library`, `symbol`, `call_site`, `effect_flags`, `latency_ns`, `status`。
- 実行時タイムアウト／キャンセルは `CancelToken`（async 連携）経由で統一。FFI 側に伝えるためのコールバックを約束。

## 7. テストと検証
- ABI 互換性チェック：
  - `ctest/ffi-smoke.c` で基本データ型の round-trip。
  - `ctest/struct-layout.c` で構造体パッキングを確認。
- サニタイザ連携：`ASan`/`UBSan` で `inc_ref/dec_ref` の対応漏れを検出。
- マルチプラットフォーム CI で Linux/Windows のビルドログとテスト結果を保管。

## 8. 今後の拡張予定
- WASM/WASI の ABI 整備とホスト関数ブリッジ。
- `async` ランタイムとの統合サンプル（io_uring / libuv）。
- Rust 向け安全ラッパ生成ツール（`reml-bindgen` 仮称）。
- 今後の課題メモ：構造体の `repr(packed)` 対応、マルチリリースの互換テスト、C++ name mangling のガイドを追跡し、必要に応じて仕様書に反映する。

## 9. unsafe ポインタ運用ガイド

> 目的：FFI 境界で露出するポインタ操作を Reml 本体の安全方針（[1-3-effects-safety.md](../1-3-effects-safety.md#unsafe-ptr-spec)）と整合させ、実装とレビューの共通基準を提供する。

### 9.1 ポインタ型マッピング

| Reml | C | Rust | Swift | Zig | 備考 |
| --- | --- | --- | --- | --- | --- |
| `Ptr<T>` | `const T*` | `*const T` | `UnsafePointer<T>` | `[*]const T` | NULL 許容で読み取り専用 |
| `MutPtr<T>` | `T*` | `*mut T` | `UnsafeMutablePointer<T>` | `[*]T` | 書き込み可能、データ競合に注意 |
| `NonNullPtr<T>` | `T*` | `NonNull<T>` | `UnsafePointer<T>` | `*T` | 非NULL保証。`Span<T>` の基盤 |
| `Ptr<void>` | `void*` | `*mut c_void` | `OpaquePointer` | `*anyopaque` | 型情報なし。ダウンキャスト必須 |
| `FnPtr<A,R>` | `R (*)(A...)` | `extern "C" fn` | `@convention(c) (A) -> R` | `fn(A) callconv(.C) R` | クロージャ無しのコードポインタ |

FFI 宣言ではこの対応表を基にシグネチャを決定し、`extern "C"` ブロック内で `Ptr<T>` 系を直接利用する。

### 9.2 安全ラッパ設計指針

低レベルポインタは `Span<T>` / `Buffer` / `StructView` 等の安全ラッパからのみ取得できるようにし、公開 API は可能な限りこれらラッパ型を返す。
`Span<T>` は長さを保持するため、境界チェック付きの `read_exact`/`write_exact` を提供し、内部で `Ptr<T>` へ降格する箇所を局所化する。
`StructView` は `byte_offset` を利用してフィールドにアクセスする構造体ビューであり、ABI 互換性は [LLVM連携ノート](llvm-integration-notes.md) の方針に従う。

### 9.3 寿命とリファレンスカウント

Reml ランタイムは参照カウントを使用するため、FFI に渡す前に `inc_ref`、不要になったら `reml_release_*` を呼ぶ契約を必ず明記する。
`defer` と組み合わせることで例外経路でも解放が実行されるようにし、`audit.log("ffi.ptr.release", ...)` を使って監査証跡を残す。
Rust など所有権モデルが存在する側では `ManuallyDrop` や `Box::into_raw` 相当の操作と組み合わせ、ダブルフリーを防止する。

### 9.4 メモリレイアウトと整列制約

ポインタのキャストや `copy_nonoverlapping` を行う前に、構造体が自然境界を満たすか `repr(C)` 互換かを [LLVM連携ノート](llvm-integration-notes.md) で確認する。
アラインメント違反が懸念される場合は `read_unaligned`/`write_unaligned` を使用し、パフォーマンス影響を `benchmark/ffi/` のマイクロベンチで検証する。
Swift や Zig のように追加メタデータが付与される言語では、呼び出し側で `withUnsafePointer` や `ptrFromInt` を利用して Reml の整列に合わせる。

### 9.5 チェックリストとサンプル

1. **FFI バインディング**: `ctest/ffi-smoke.c` に `Ptr<T>`/`MutPtr<T>` の往復テストを追加し、NULL/非NULL の両ケースを検証する。
2. **GPU/IO ハンドラ**: `guides/runtime-bridges.md` の GPU チェックリストに従い、`effect {runtime, gpu, unsafe}` を宣言した例を `examples/gpu/` に配置する。
3. **テストベンチ用スタブ**: `tests/ffi/mock_host.reml` で `FnPtr` コールバックを使ったスタブを用意し、`audit` ログが記録されることを確認する。

これらのサンプルは `Core.Unsafe.Ptr` の API ドキュメントと連携させ、CI でリグレッションテストを行う。


## 10. `FfiArgs` / `FfiValue` とシリアライズヘルパ

Reml 3.9 章では FFI 呼び出しの引数・戻り値を `Span<u8>` で表現する `FfiArgs` / `FfiValue` が定義された。ハンドブックでは次の約束を採用する。

```reml
let args = ffi::encode_args(&(u32::from(42), "hello".to_bytes()));
let raw  = foreign_fn.call(args)?;        // FfiValue = Span<u8>
let reply: (u32, Bool) = ffi::decode_result(raw)?;
```

- `encode_args` は Reml のタプル/レコードを `Span<u8>` に直列化するユーティリティであり、構造体レイアウトは `FfiSignature` に従う。
- `decode_result` は戻り値を同じ `FfiSignature` に基づいて復元する。失敗時は `FfiErrorKind::InvalidSignature` を返し、`audit.log("ffi.decode_failed", ...)` に詳細を残す。
- 手動で `Span<u8>` を構築する場合は `span_from_raw_parts(ptr, len)` を利用し、`capability.effect_scope` に `memory` が含まれることを確認する。

## 11. 監査テンプレートと Capability 連携

FFI 呼び出しは `call_with_capability` を介することで `CapabilityRegistry` と連携し、監査ログとセキュリティポリシーが適用される。

```reml
fn call_db(cap: FfiCapability, handle: SymbolHandle, params: DbParams, audit: AuditSink) -> Result<DbResult, FfiError> = {
  let args = ffi::encode_args(&params);
  let ctx  = AuditContext::new("ffi", handle.symbol_name())?;
  ctx.log("ffi.call.start", json!({ "library": handle.library_path(), "symbol": handle.symbol_name() }))?;

  let value = cap.call_function(handle, args)?;     // effect {ffi, security, audit}
  ctx.log("ffi.call.end", json!({ "latency_ns": ctx.elapsed()?.as_nanos(), "status": "ok" }))?;

  ffi::decode_result(value)
}
```

- `call_function` は `effect {ffi, security, audit}` を持ち、`SecurityCapability` と `AuditContext` を通じて許可・記録を行う。
- 署名検証が有効な場合、`FfiCapability.verify_abi` によって `FfiSignature` と実際のシンボルが一致するか確認される。
- 失敗時は `FfiErrorKind::SecurityViolation` や `FfiErrorKind::CallFailed` が返るため、`Diagnostic` へ変換して CLI/LSP に通知する。

### 11.1 Capability 登録チェックリスト

1. `CapabilitySecurity.effect_scope` に `{ffi, audit, security}` を含める。
2. サンドボックスが必要な場合、`FfiSecurity.sandbox_calls = true` とし、CPU/メモリ制限やシステムコールホワイトリストを設定する。
3. `call_sandboxed` を利用するラッパでは `FFI Sandbox` の制約（`memory_limit`, `syscall_whitelist` など）を監査ログへ残す。
4. プラットフォーム差異：`resolve_calling_convention` の結果を `CapabilitySecurity.policy` と照合し、非対応時は `unsupported_target` 診断を発行する。

### 11.2 プラットフォーム別注意事項

- **Linux**: `libdl` を利用した遅延ロード。`RTLD_DEEPBIND` は未使用推奨。System V ABI を前提。
- **Windows**: `LoadLibraryW` + `GetProcAddress`。`__stdcall` (`StdCall`) を既定とし、`ForeignFunction` 取得時にキャッシュする。
- **macOS**: `dlopen` (`NSAddImage`) を利用。コードサイン制約に注意。
- **WASI/WASM**: 現在はホワイトリスト方式のみサポート。`call_with_capability` は `SecurityPolicy` に定義されたホスト関数へルーティングする。

## 12. 今後の拡張ロードマップ

- フェーズ1（〜6ヶ月）: FFI ランタイムの最小構成（`encode_args`/`decode_result`, 監査テンプレ）を安定化。
- フェーズ2（6〜12ヶ月）: `async` ランタイムとの統合、タイムアウト／キャンセル連携を提供。
- フェーズ3（12ヶ月〜）: 多言語バインディング（Rust / Go / Python）の公式パッケージ化、`reml-bindgen` ツール公開、WASI/wasi-nn 等への対応。

---

> **ドラフト状態**: 本ハンドブックはフェーズ0で骨子を作成した段階。各セクションはフェーズ1以降の PoC とレビュー結果に合わせて詳細化する。
