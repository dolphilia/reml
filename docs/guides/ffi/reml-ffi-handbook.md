# Reml FFI ハンドブック（ドラフト）

> 目的：Reml と外部ランタイム（C/C++/Rust/システムライブラリ等）との安全な接続方法を明文化し、[LLVM連携ノート](../compiler/llvm-integration-notes.md)・`1-3-effects-safety.md`・`../runtime/runtime-bridges.md` に分散している知識を一本化する。

## 1. 適用範囲と位置付け
- 既定ターゲット：System V AMD64 / Windows x64。将来 ARM64 / WASM を追加予定。
- コンパイラ実装（Rust 版）とランタイム、DSL プロジェクトが共通で参照する運用ガイドとして利用。
- FFI で橋渡しする典型シナリオ：データベースドライバ、クラウド SDK、GPU ライブラリ、既存サービスとの IPC、ホットリロード可能なプラグイン。

## 2. ABI・データレイアウトの要約

> 公式仕様: [3-9 §2.1](../../spec/3-9-core-async-ffi-unsafe.md#21-abi-とデータレイアウト) を参照。ここでは実務での確認手順とツール連携のみをまとめる。

- `remlc --emit-header`（将来実装予定）で生成した C ヘッダと [guides/llvm-integration-notes.md](../compiler/llvm-integration-notes.md#ターゲット-abi--データレイアウト) の定義を突き合わせ、構造体・列挙体が `repr(C)` の制約を満たしているか `llvm-readobj --sd` で検証する。
- 文字列／スライスは 3-9 §2.1 の `{ ptr, len }` レイアウトを前提にする。ゼロコピーを要求する場合は、RC カウンタの増減を呼び出しコードで確認し、`audit.log("ffi.call", ...)` に `status = "success"` を出力できることを CI でチェックする。
- 例外は境界を越えて伝播しない。C++ 例外を扱う場合はガードレイヤで捕捉・エラーマッピングし、`FfiErrorKind::CallFailed` に変換する実装メモをここへ残す。
- ARM64 / WASM など未正式対応ターゲットに関する調査結果は `../../notes/` に記録し、仕様が更新されたら本ガイドからリンクのみを残す。

## 3. 効果タグと `unsafe` 境界

> 公式仕様: [3-9 §2.2–2.3](../../spec/3-9-core-async-ffi-unsafe.md#22-効果タグと-unsafe-境界) を参照。ここではラッパ実装時の現場メモを補足する。

- `effect` 宣言は仕様で定義された最小集合（`{ffi, unsafe}`）を基準に、I/O 性質に応じて `io.blocking` / `io.async` / `io.timer` を追加する。ガイドでは `@no_blocking` 等の属性を付与する場所（ラッパ関数 or Capability 宣言）をコードレビューで確認するチェックリストを共有する。
- `ForeignCall` 効果でスタブ化する場合は Stage を `Experimental` から始め、`reml capability stage promote` 実行ログをリポジトリに残す。仕様のステージ要件に従い、`audit.log` で `status = "stubbed"` を確認する手順を CI ワークフローに追加する。
- エフェクト違反を検出するには `--effects-debug` フラグを利用し、`Diagnostic.extensions["effects"].residual` が空であるか手元で検証する。記録した出力は本ガイドにメモとして追記し、将来の Stage 昇格レビューで参照できるようにする。

## 4. リンクとビルドの手順
1. ヘッダ生成：`remlc --emit-header foo.reml`（将来実装）で C 用シグネチャを生成。
2. ランタイムライブラリ：`libreml_runtime.a`（RC/メモリ/診断）をリンク。
3. プラットフォーム差異：
   - Linux: `clang foo.c foo.ll libreml_runtime.a -o foo`
   - Windows: `cl /Fe:foo.exe foo.c foo.ll libreml_runtime.lib`
4. デバッグ情報を有効化する場合は `-g` 付き LLVM IR を生成し、`lldb` / `windbg` で解析。

## 5. 所有権とライフタイム契約

> 公式仕様: [3-9 §2.6](../../spec/3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界)。ここでは多言語バインディングのチェックリストを共有する。

1. **Reml → C**: 値を渡す前に `inc_ref` を呼び、ホスト側で `reml_release_*`（暫定）または `ForeignPtr.release` を必ず呼ぶ。CI では `ffi-smoke` テストで `status = "leak"` が出ないことを監査ログから確認する。
2. **C / C++ → Reml**: 受け取ったポインタは `wrap_foreign_ptr` で `Ownership::Borrowed` に設定し、呼び出しスコープを抜ける前に `release_foreign_ptr` を呼ぶか、`Ownership::Transferred` として解放関数を登録する。
3. **ゼロコピー文字列**: UTF-8 前提。書記素操作が必要な場合は Reml 側で実行し、C 側にはバイト列として渡す。`Span<u8>` を使う場合は `span_from_raw_parts` の戻り値を即座に `ForeignBuffer` へ昇格し、監査ログには `effect_flags` を記録する。
4. **エラーハンドリング**: FFI 側の失敗は `FfiError` を経由して `Diagnostic.domain = Runtime` と `code = "ffi.call.failed"` を付与する。ガイドでは主なエラーパターン（NULL、整列違反など）を随時追記する。

> 多言語サンプルは継続的に追加予定。寄稿時は仕様の契約を満たすことを確認したうえで、該当するサンプルにリンクを張ってください。

## 6. 監査・可観測性

> 公式仕様: [3-9 §2.7](../../spec/3-9-core-async-ffi-unsafe.md#27-監査テンプレートと可観測性)、[3-6 §5.1](../../spec/3-6-core-diagnostics-audit.md#ffi-呼び出し監査テンプレート)。

- 監査ログは `AuditEnvelope.metadata["ffi"]` にテンプレートを格納する。ガイドでは `audit.log("ffi.call", template)` を呼び出すヘルパ関数例と、CI で `status` が期待値になっているかを検証する `jq` スニペットを掲載する。
- 実験的なサンドボックス構成（`call_with_capability` + `sandbox`）では、`capability_stage` が `Experimental` のまま本番で使われていないかメトリクスを監視する。ダッシュボード例は `monitoring/ffi-dashboard.json` を参照。
- 監査漏れを検出するには `AuditCapability.emit` をモック化し、`effect_flags` に `ffi` が含まれているかチェックするテストを用意する。判定ルールはチームごとに追記し、本ガイドではテストの雛形のみ保持する。

## 7. テストと検証

> 公式仕様: [3-9 §2.6–2.7](../../spec/3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界) の CI 要件を参照。ここでは追加で行っている検証のメモを残す。

- **ABI チェック**: `ctest/ffi-smoke.c` と `ctest/struct-layout.c` を継続利用。結果が仕様の表と一致したかを `tests/ffi/README.md` に記録し、差分が発生した場合は `../../notes/` に原因調査メモを残す。
- **サニタイザ運用**: `asan`/`ubsan` を有効化したビルド手順を `scripts/ffi-sanitized-build.sh` にまとめる。False Positive が発生した場合の抑制パターンを併記する。
- **多言語バインディング検証**: Rust ラッパや Python C-API の PoC は `examples/ffi/` 配下で管理し、仕様更新時に再実行する。期待する監査ログ（`status = "success"` など）が出力されるか `jq` ベースのスモークテストで確認する。

## 8. 今後の拡張予定
- WASM/WASI の ABI 整備とホスト関数ブリッジ。
- `async` ランタイムとの統合サンプル（io_uring / libuv）。
- Rust 向け安全ラッパ生成ツール（`reml-bindgen` 仮称）。
- 今後の課題メモ：構造体の `repr(packed)` 対応、マルチリリースの互換テスト、C++ name mangling のガイドを追跡し、必要に応じて仕様書に反映する。

## 9. unsafe ポインタ運用ガイド

> 目的：FFI 境界で露出するポインタ操作を Reml 本体の安全方針（[1-3-effects-safety.md](../../spec/1-3-effects-safety.md#unsafe-ptr-spec)）と整合させ、実装とレビューの共通基準を提供する。

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
`StructView` は `byte_offset` を利用してフィールドにアクセスする構造体ビューであり、ABI 互換性は [LLVM連携ノート](../compiler/llvm-integration-notes.md) の方針に従う。

### 9.3 寿命とリファレンスカウント

Reml ランタイムは参照カウントを使用するため、FFI に渡す前に `inc_ref`、不要になったら `reml_release_*` を呼ぶ契約を必ず明記する。
`defer` と組み合わせることで例外経路でも解放が実行されるようにし、`audit.log("ffi.ptr.release", ...)` を使って監査証跡を残す。
Rust など所有権モデルが存在する側では `ManuallyDrop` や `Box::into_raw` 相当の操作と組み合わせ、ダブルフリーを防止する。

### 9.4 メモリレイアウトと整列制約

ポインタのキャストや `copy_nonoverlapping` を行う前に、構造体が自然境界を満たすか `repr(C)` 互換かを [LLVM連携ノート](../compiler/llvm-integration-notes.md) で確認する。
アラインメント違反が懸念される場合は `read_unaligned`/`write_unaligned` を使用し、パフォーマンス影響を `benchmark/ffi/` のマイクロベンチで検証する。
Swift や Zig のように追加メタデータが付与される言語では、呼び出し側で `withUnsafePointer` や `ptrFromInt` を利用して Reml の整列に合わせる。

### 9.5 チェックリストとサンプル

1. **FFI バインディング**: `ctest/ffi-smoke.c` に `Ptr<T>`/`MutPtr<T>` の往復テストを追加し、NULL/非NULL の両ケースを検証する。
2. **GPU/IO ハンドラ**: `../runtime/runtime-bridges.md` の GPU チェックリストに従い、`effect {runtime, gpu, unsafe}` を宣言した例を `examples/gpu/` に配置する。
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

<a id="11-3-capability-handle"></a>
### 11.3 CapabilityHandle の型付き API

`CapabilityRegistry::verify_capability_stage` は型付きバリアント (`Gc`/`Io`/`Async`/`Security` 等) を返すため、FFI 呼び出しでは `match` で分岐して目的の API を直接呼べるようになりました。たとえば GC によるメモリ回収と Security ポリシー適用を同時に検証する場合:

```reml
let handle = capability_registry.verify_capability_stage(
  "ffi.capability",
  StageRequirement::Exact(StageId::Beta)
)?;

match handle {
  CapabilityHandle::Gc(gc) => gc.collect(),
  CapabilityHandle::Security(security) => security.enforce(&policy)?,
  _ => audit.log("ffi.capability.unexpected", json!({ "id": handle.descriptor().id })),
}
```

`CapabilityHandle::descriptor()` は監査用 `stage`/`effect_scope` を再利用できるので、`AuditEnvelope` を組み立てるときに再評価を避けられます。`handle.as_gc()` や `handle.as_security()` のようなヘルパも用意されており、`../runtime/runtime-bridges.md#1.3` で紹介した Bridge の運用手順と合わせて型安全な分岐条件を記録してください。

## 12. 今後の拡張ロードマップ

- フェーズ1（〜6ヶ月）: FFI ランタイムの最小構成（`encode_args`/`decode_result`, 監査テンプレ）を安定化。
- フェーズ2（6〜12ヶ月）: `async` ランタイムとの統合、タイムアウト／キャンセル連携を提供。
- フェーズ3（12ヶ月〜）: 多言語バインディング（Rust / Go / Python）の公式パッケージ化、`reml-bindgen` ツール公開、WASI/wasi-nn 等への対応。

---

> **ドラフト状態**: 本ハンドブックはフェーズ0で骨子を作成した段階。各セクションはフェーズ1以降の PoC とレビュー結果に合わせて詳細化する。
## 10. 効果ハンドラによるスタブ化（実験段階）

FFI 呼び出しをテスト用スタブへ差し替える場合は、`ForeignCall` 効果と Capability stage を併用する。

```reml
effect ForeignCall : ffi {
  operation call(name: Text, payload: Bytes) -> Result<Bytes, FfiError>
}

@handles(ForeignCall)
@requires_capability(stage="experimental")
pub fn with_foreign_stub(req: Request) -> Result<Response, FfiError> ! {} =
  handle do ForeignCall.call("service", encode(req)) with
    handler ForeignCall {
      operation call(name, payload, resume) {
        audit.log("ffi.call", { "name": name, "bytes": payload.len() })
        resume(Ok(stub_response(name, payload)))
      }
      return result { result.and_then(decode_response) }
    }
```

### 10.1 Stage 運用フロー

1. Experimental で opt-in: `reml capability enable foreign-call --stage experimental`。
2. PoC 実行: `reml run -Zalgebraic-effects test ffi::with_foreign_stub --effects-debug` で `Diagnostic.extensions["effects"].residual = []` を確認。
3. Beta 昇格: `reml capability stage promote foreign-call --to beta`。CLI やマニフェスト (`expect_effects_stage`) を更新し、監査ログに `stage: "beta"` が出力されることを確認。
4. Stable 化: 実運用で問題ないことが確認できたら `--to stable` を実行し、実験フラグなしのビルドを完了させる。

### 10.2 監査テンプレート

```json
{
  "event": "ffi.call",
  "stage": "experimental",
  "symbol": "service",
  "payload_bytes": 128,
  "status": "stubbed"
}
```

監査ログに `stage` フィールドを常に含めることで、実験環境からの呼び出しが本番へ紛れ込んでいないかを監視できる。`effects.stage.promote_without_checks` 診断が発生した場合は、Capability とマニフェスト側の stage 設定を見直す。
