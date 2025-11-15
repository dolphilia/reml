# 2.1 ランタイム統合計画

本章は Phase P2 における Rust 実装と既存ランタイム（`runtime/native/`）の橋渡し計画を定義する。LLVM バックエンドで生成した IR を既存ランタイム API に安全に接続し、`Core Runtime & Capability Registry`（[3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md)）および `Core Async / FFI / Unsafe`（[3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md)）で規定された契約を満たすことが目的である。Rust バックエンド計画（`2-0-llvm-backend-plan.md`）と連携し、所有権・効果タグ・監査ログを一貫させる。

## 2.1.1 目的
- Rust バックエンドからランタイム (`libreml_runtime.a` / `.lib`) への FFI 層を実装し、`mem_alloc`/`inc_ref`/`dec_ref`/`panic` 等の必須 API を Rust 側から安全に呼び出せるようにする。
- Capability Registry・Audit・Security ポリシーに従った `effect {runtime, audit, ffi, unsafe}` の制約を Rust で再現し、`AuditEnvelope.metadata.bridge.*` を生成する。
- ランタイム統合を Windows・macOS・Linux で検証し、MSVC/GNU 双方の ABI 差異を `runtime/native/include/` のヘッダ準拠で吸収する。

## 2.1.2 スコープと前提
- **対象範囲**
  - ランタイム FFI バインディング定義（Rust `extern "C"` シグネチャ、`#[link(name = "reml_runtime")]`）。
  - 所有権ラッパ（`ForeignPtr` 相当）、参照カウンタ操作、文字列/スライスの `{ptr,len}` 変換、エラー型 (`FfiError`) 統合。
  - Capability Registry 連携（`CapabilityRegistry::register`/`get`）と Stage 要件（`StageRequirement::{Exact, AtLeast}`）チェック、監査ログ (`audit.log`) の統合。
  - `AuditContext`/`SecurityCapability` と連携した FFI 呼び出しラッパの導入、`effect` タグ検証 (`--effects-debug` ログ)。
- **除外**
  - ランタイム内部実装（C コード）の刷新。必要なら `runtime/native/` 側で別途計画を立てる。
  - DSL プラグインや外部 Capability の Stage 昇格判断（Chapter 4 に委譲）。
  - CI/監査メトリクスのダッシュボード構築（P3 スコープ）。
- **前提**
  - `runtime/native/` の API が Phase 1-5 時点で整備されており、ヘッダ `reml_runtime.h` `reml_os.h` `reml_platform.h` が最新。
  - `docs/guides/runtime-bridges.md`・`docs/guides/reml-ffi-handbook.md` の手順がベースラインとして共有済み。
  - `appendix/glossary-alignment.md` に Rust ↔ Reml 用語対応表が収録されている。

## 2.1.3 完了条件
- Rust 実装で提供する FFI ラッパが、`runtime/native/tests/` と同等のテストケース（メモリアロケータ・参照カウンタ・OS ラッパ）を Rust 側で実行し、`AuditEnvelope.metadata.bridge.status = "ok"` を記録する。
- `effect` 検証 (`--effects-debug`) の結果が Residual なしとなり、`effects.contract.stage_mismatch` 診断がゼロ件である。
- Windows (MSVC/GNU)、macOS、Linux の 3 プラットフォームで Rust バックエンド → ランタイム連携 E2E テストが成功し、監査ログ (`audit.log("ffi.call", ...)`) が `reports/runtime-bridge/*.json` に保存される。
- Capability 登録 (`CapabilityRegistry::register`) と Stage ポリシー (`StageRequirement`) の実装が Rust 側に存在し、`docs/spec/3-8-core-runtime-capability.md` の該当節に照らして整合が取れている。

## 2.1.4 主成果物

| 成果物 | 内容 | 依存資料 |
| --- | --- | --- |
| `compiler/rust/runtime/ffi/` | ランタイム FFI ラッパ crate（`extern` 宣言、`Result<T, FfiError>` 変換、所有権ヘルパ、`AuditContext` 連携） | `runtime/native/include/`, `docs/guides/reml-ffi-handbook.md`, `docs/spec/3-9` |
| Capability Registry バインディング | Rust 側 `CapabilityRegistry` 実装／FFI、Stage/効果タグチェック、`verify_capability_stage` API | `docs/spec/3-8-core-runtime-capability.md`, `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` |
| 監査・診断 API | `audit.log`, `Diagnostic.extensions["bridge"]`, `security.report_violation` の Rust ラッパ | `docs/spec/3-6-core-diagnostics-audit.md`, `docs/guides/runtime-bridges.md` |
| テストハーネス | Rust 側 `cargo test` 相当の FFI/ランタイム検証、`ffi-smoke`（同期/非同期/タイムアウト） | `runtime/native/tests/`, `docs/guides/reml-ffi-handbook.md` |

## 2.1.5 マイルストーン（目安）

| 週 | マイルストーン | 主タスク | 検証方法 |
| --- | --- | --- | --- |
| W1 | ランタイム API 棚卸し | `reml_runtime.h` と OCaml バインディング (`compiler/ocaml/src/llvm_gen/runtime_stub.ml`) の差分確認、Rust `extern` 定義 | `bindgen` / 手動ヘッダ比較、`cargo test ffi_signature_smoke` |
| W2 | 所有権ラッパ実装 | `ForeignPtr`, `Span`, `RuntimeString` 等の実装、`inc_ref`/`dec_ref` 呼び出し位置決定 | `runtime.refcount.*` メトリクス比較、AddressSanitizer 実行 |
| W3 | Capability & Audit 統合 | `CapabilityRegistry` FFI、`AuditContext`, `SecurityPolicy` の Rust ラッパ、`effect` 検証 | `audit.log` 確認、`--effects-debug` 出力チェック |
| W4 | クロスプラットフォーム検証 | Windows GNU/MSVC, macOS, Linux での FFI 連携テスト・監査ログ収集、`panic`/`timeout` シナリオ確認 | GitHub Actions matrix、`reports/runtime-bridge/*.json` |
| W4.5 | P2 統合レビュー | 成果物レビュー、`2-2-adapter-layer-guidelines.md` との連携事項整理、P3 ハンドオーバー | `docs/plans/rust-migration/README.md` 更新、`docs-migrations.log` |

### W1: ランタイム API 棚卸し状況

- 予定している Rust 側 FFI 入口は `compiler/rust/runtime/ffi/`（`#[link(name = "reml_runtime")]`）に集約するが、現時点でのオーソリティは `runtime/native/include/reml_runtime.h` に記された C API と、実際に LLVM IR から呼んでいる OCaml 側の宣言です。`compiler/ocaml/src/llvm_gen/runtime_stub.ml` は存在せず、`declare_runtime_functions`（`compiler/ocaml/src/llvm_gen/codegen.ml:196-255`）が実装済みのエクスポート群になるため、これらを共通基準とします。
- 以下の表は W1 で確認したヘッダ側の API と、OCaml で宣言／利用されている関数（参照行）を照合したものです。Rust では同等の `extern "C"` を手書きし、必要に応じて `bindgen` で検証します。

| C API (`reml_runtime.h`) | OCaml 側宣言/利用（`codegen.ml`） | Rust 側 `extern` の予定 | 備考 |
| --- | --- | --- | --- |
| `mem_alloc(size_t)` (line 68) | `Llvm.declare_function "mem_alloc"` (`codegen.ml:201-205`) | `extern "C" fn mem_alloc(size: usize) -> *mut c_void` → `ForeignPtr` に変換し、ペイロードポインタを返す | ヘッダは payload 直後のポインタを返す。`type_tag` は `call_mem_alloc` 側で 4 バイト先に書き込む |
| `mem_free(void*)` (line 79) | 現在宣言・呼び出しなし | `extern "C" fn mem_free(ptr: *mut c_void)`（まずは `ForeignPtr::drop` から呼び出す予定） | OCaml は `dec_ref` 側で内部的に解放。不要な重複を避けるため Rust ではまだ呼ばないが、API 総覧には含む |
| `inc_ref(void*)` (line 92) | `Llvm.declare_function "inc_ref"` + `call_inc_ref`（`codegen.ml:206-340`） | `extern "C" fn inc_ref(ptr: *mut c_void)` → `ForeignPtr::clone`/`ResourceHandle` で利用 | 同期フェーズ 2 用。Rust でも `ForeignPtr::clone` から呼ぶ前提 |
| `dec_ref(void*)` (line 104) | `Llvm.declare_function "dec_ref"` + `call_dec_ref`（`codegen.ml:211-340`） | `extern "C" fn dec_ref(ptr: *mut c_void)` → `Drop` で呼び出す予定 | `ForeignPtr::Drop` と `AuditEnvelope.metadata.bridge.ownership` を合わせる |
| `panic(const char*)` (line 123) | `Llvm.declare_function "panic"` が `(ptr, i64) -> void` で `noreturn` 属性（`codegen.ml:216-223`） | `extern "C" fn panic(ptr: *const c_char, len: i64)` + `CStr` 化してログ出力、残りは無視 | LLVM 側が FAT pointer を渡すため長さパラメータあり。Rust では `panic` を `Abort` 相当とみなし、`effect {unsafe}` で許可 |
| `print_i64(int64_t)` (line 136) | `Llvm.declare_function "print_i64"`（`codegen.ml:224-227`） | `extern "C" fn print_i64(value: i64)` | デバッグ用途。Rust でも `--effects-debug` で呼び出すケースを確認 |
| `string_eq`/`string_compare` (lines 149-165) | `Llvm.declare_function "string_eq"`（`codegen.ml:1650-1668`）、`"string_compare"`（`codegen.ml:1731-1751`） | `extern "C" fn string_eq(a: *const ReMlString, b: *const ReMlString) -> i32` / `string_compare` 同様 | `ReMlString` 構造体は `reml_runtime.h` に記載。Rust でも `repr(C)` 構造を共有する |
| `reml_ffi_bridge_record_status(i32)` | `Llvm.declare_function "reml_ffi_bridge_record_status"`（`codegen.ml:229-237`） | `extern "C" fn reml_ffi_bridge_record_status(status: i32)` | Capability/Audit 構造との連携で、`AuditEnvelope.metadata.bridge.status` を更新 |
| `reml_ffi_acquire_borrowed_result(ptr) -> ptr` | `Llvm.declare_function "reml_ffi_acquire_borrowed_result"`（`codegen.ml:239-246`） | `extern "C" fn reml_ffi_acquire_borrowed_result(ptr: *mut c_void) -> *mut c_void` | `Borrowed` 経路で所有権を明示的に保持するための橋渡し |
| `reml_ffi_acquire_transferred_result(ptr) -> ptr` | `Llvm.declare_function "reml_ffi_acquire_transferred_result"`（`codegen.ml:248-255`） | `extern "C" fn reml_ffi_acquire_transferred_result(ptr: *mut c_void) -> *mut c_void` | `Transferred` 経路で所有権を移すための補助関数 |

- `reml_set_type_tag`/`reml_get_type_tag` は `reml_runtime.h:184-198` にあるが、`codegen.ml` では `call_mem_alloc` 側でヘッダに直接 type tag を書き込んでいる（`codegen.ml:277-315`）。Rust では `ForeignPtr::from_payload` が `type_tag` を検証するため、当面は手動でヘッダを書き換える実装のままにし、将来的に `reml_set_type_tag` を呼ぶか検討する。
- `string_eq/string_compare` 以外の文字列ビルトイン（`reml_string_t` や `REML_GET_HEADER`）は `reml_runtime.h` で定義済みの構造体/マクロなので、Rust 側で同じ `repr(C)` を再現して ABI を一致させる。
- `panic` の FAT pointer と `mem_free` の利用状況を踏まえ、Rust からの呼び出し時に `bindgen --allowlist-function ...` を走らせ、手書き `extern` のシグネチャと自動生成の結果を突き合わせる予定。`cargo test ffi_signature_smoke`（W1 の検証項目）については `compiler/rust/runtime/ffi` crate の初期実装が整い次第、`mem_alloc`/`inc_ref`/`dec_ref`/`panic` を呼び出すスモークテストとして実行する。

## 2.1.6 作業ストリーム

- **FFI シグネチャ整備**  
  - `reml_runtime.h` のシグネチャに基づき、Rust 側で `extern "C"` を手動定義。`panic` は `{ptr, len}` を受け取る LLVM 生成 IR と `const char*` を期待する C 実装の差を Rust 側で調整（`CStr` 化や長さ無視のポリシーを文書化）。  
  - `mem_alloc` は `NonNull<c_void>` を返し、`Layout` 情報を保持して呼び出し元の MIR 決定と一致させる。`inc_ref`/`dec_ref` は `unsafe` block 内で呼び出し、`AuditEnvelope.metadata.bridge.ownership` を `Borrowed` / `Transferred` などで記録する。

- **所有権ラッパ／エラーモデル**  
  - Rust の `ForeignPtr<T>`（`repr(transparent)`）を実装し、`Ownership` 列挙体（`Borrowed`, `Transferred`, `Pinned` 等）を `docs/guides/reml-ffi-handbook.md` §5 に合わせる。  
  - `FfiError` は `FfiErrorKind` + `metadata`（JSON）として保持し、`Diagnostic` への変換 (`code = "ffi.call.failed"`) を提供。`panic` 呼び出しは `AbortError` として扱い、`effect {unsafe}` を要求する。

- **Capability Registry 連携**  
  - Rust 側で `CapabilityRegistry` を `OnceLock<CapabilityRegistry>` + FFI で保持し、`register` / `get` / `describe` を `Result` 型で安全に公開。  
  - Stage 要件 (`StageRequirement::{Exact, AtLeast}`) を Rust で表し、Capability の `stage` フィールドと比較、ミスマッチ時に `effects.contract.stage_mismatch` 診断を生成する。  
  - `AuditCapability`・`SecurityCapability` を取得し、FFI 呼び出し前後で `audit.log("ffi.call", ...)`、`security.verify_signature` を必須化する。

- **監査ログと効果タグ**  
  - FFI 呼び出しラッパで `audit.log("ffi.call.start", ...)` / `audit.log("ffi.call.end", ...)` を生成し、`docs/spec/3-6-core-diagnostics-audit.md` が定めるキー（`latency_ns`, `target`, `abi`, `ownership`, `status`）を埋める。  
  - `effect` チェック用に `effects::scope("ffi")` を実装し、呼び出しスタックが `effect {ffi, audit, unsafe}` を保持しているか `--effects-debug` フラグで検証する。

- **クロスプラットフォーム調整**  
  - Windows GNU/MSVC の `CallingConvention` 差異を `resolve_calling_convention` で吸収し、`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` で記録された `conf-llvm-static` 問題を避けるため `.lib` パス設定を自動化。  
  - macOS (System V, LC_ID_DYLIB) と Linux (glibc/musl) の差異は `reml_os.h` の抽象化 (`reml_os_thread_*`, `reml_os_file_*`) を Rust 側で再利用。  
  - WASI/WASM など将来ターゲットは `adapter` 層に委譲し、`2-2-adapter-layer-guidelines.md` でポリシーを共有。

## 2.1.7 検証とメトリクス
- **FFI テスト**: `cargo test ffi_smoke`（仮）で `mem_alloc` → `inc_ref` → `dec_ref` → `panic` シナリオを実行し、`audit.log` に `status = "ok"` が記録されることを確認。  
- **効果検証**: `reml run -Zrust-backend --effects-debug fixtures/ffi/*.reml` を実行し、`Diagnostic.extensions["effects"].residual = []` を確認。  
- **監査メトリクス**: `collect-iterator-audit-metrics.py` に `ffi.call.latency_ns`, `runtime.refcount.inc`, `runtime.refcount.dec` を追加し、OCaml 実装との差分を ±5% に抑える。  
- **Windows 監査**: `audit.log("windows.runtime", { "distribution": "msys2-16" | "official-zip-19", "linker": "...", "status": ... })` を出力し、`reports/windows-env-check.json` へ追記。

## 2.1.8 リスクと対応
- **ABI ミスマッチ**  
  - *リスク*: Rust ↔ C 間で構造体配置が食い違い、メモリ破壊が発生。  
  - *対応*: `bindgen --allowlist-function` を PoC として実行、出力結果と手書き宣言を突き合わせる。`#[repr(C)]` と `assert_eq!(size_of::<T>(), ...)` を単体テストで検証。

- **参照カウンタの不整合**  
  - *リスク*: `inc_ref`/`dec_ref` 呼び出し回数がズレ、リークや二重解放が発生。  
  - *対応*: Rust 側ラッパで `Drop` 実装に `dec_ref` を集約し、`audit.log("runtime.refcount", ...)` で回数を記録。AddressSanitizer を CI へ追加（`DEBUG=1`）。

- **監査ログ欠落**  
  - *リスク*: `audit.log` 呼び出し忘れで監査指標が欠落し、P3 CI で検出される。  
  - *対応*: Rust ラッパを `AuditContext` 必須に設計し、`CallOptions` に `audit: AuditSink` を含める。ログ欠落は `lint`（CI スクリプト）で検知。

- **Windows ツールチェーン差異**  
  - *リスク*: MSVC と GNU のハンドラ呼び出し規約が一致せずクラッシュ。  
  - *対応*: `resolve_calling_convention(platform_info(), metadata)` で `CallingConvention::X86_64_Win64` を選択し、GNU path では `llvm::CallingConv::X86_64_SYSV` を明示。`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` の fallback 手順で `.lib` の存在をチェック。

## 2.1.9 関連ドキュメント更新
- 新しい Capability / Stage 運用ルールを導入した場合は `docs/spec/3-8-core-runtime-capability.md` に脚注を提案し、用語差異を `appendix/glossary-alignment.md` へ反映する。  
- ランタイム API 更新時は `runtime/native/README.md` と `compiler/ocaml/docs/runtime-api-integration-status.md` を確認し、差分があれば追記する。  
- `docs/plans/rust-migration/README.md`・`docs/plans/README.md` の P2 セクションに本章追加を反映する。

## 2.1.10 P1 W4.5 ハンドオーバー
- `reports/dual-write/front-end/P1_W4.5_frontend_handover/diag/effects/` に `type_condition_*`, `effect_residual_leak`, `ffi_*` の Run ID（`20280418-w4-diag-effects-r3`, `20280601-w4-diag-type-effect-rust-typeck-r7`）を集約。Stage/Audit JSON（`effect.stage.*`, `bridge.stage.*`）と `effects-metrics.{ocaml,rust}.json` を FFI/Capability 実装の既知欠陥として受領し、`StageAuditPayload` 実装と `--runtime-capabilities` 伝播を最優先で補完する。
- Streaming ケース (`diag/streaming/20280410-w4-diag-streaming-r21`) の `runconfig.extensions.stream.*` / `flow.backpressure.max_lag_bytes`、CLI/LSP ケース (`diag/cli-lsp/20280430-w4-diag-cli-lsp`) の `extensions.config.*` / `extensions.cli.*` / `extensions.lsp.*` をランタイム統合テストへ導入し、Rust CLI がアダプタ層と同じメタデータを生成できるかを検証する。
- `p1-front-end-checklists.csv` と `appendix/w4-diagnostic-case-matrix.md#W4.5-ハンドオーバーメモ` に追加した `HandedOver` 情報を参照し、Parser Recover（`W4.5:Pass`）以外のカテゴリは `Pending(W4.5)` であることを P2 のリスク登録に記載する。
- `w4-diagnostic-cases.txt` の `#handed_over` コメントで Streaming / TypeEffect / CLI ケースが明示されているため、ランタイム統合テストでは該当ケースの再実行をデフォルトにする。

## 2.1.11 次フェーズとの接続
- P3 CI 統合では本章で整備した監査ログと FFI テストハーネスを `3-0-ci-and-dual-write-strategy.md` に組み込み、dual-write 監査の自動比較を実現する。  
- P4 リスク登録 (`4-0-risk-register.md`) へ移行する際、FFI・Capability 関連の残存リスク（権限昇格・未検証ターゲット）を共有する。  
- Adapter 層設計 (`2-2-adapter-layer-guidelines.md`) と連携し、プラットフォーム差異を FFI 呼び出し前に吸収するための API 要件を同期する。

---
**参照**: `docs/plans/rust-migration/2-0-llvm-backend-plan.md`, `docs/guides/runtime-bridges.md`, `docs/guides/reml-ffi-handbook.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/spec/3-9-core-async-ffi-unsafe.md`, `runtime/native/README.md`, `compiler/ocaml/docs/runtime-api-integration-status.md`
