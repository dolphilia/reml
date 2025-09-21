# Reml FFI ハンドブック（ドラフト）

> 目的：Reml と外部ランタイム（C/C++/Rust/システムライブラリ等）との安全な接続方法を明文化し、`a-jit.md`・`1-3-effects-safety.md`・`guides/runtime-bridges.md` に分散している知識を一本化する。

## 1. 適用範囲と位置付け
- 既定ターゲット：System V AMD64 / Windows x64。将来 ARM64 / WASM を追加予定。
- コンパイラ実装（OCaml 版／将来の self-host）とランタイム、DSL プロジェクトが共通で参照する運用ガイドとして利用。
- FFI で橋渡しする典型シナリオ：データベースドライバ、クラウド SDK、GPU ライブラリ、既存サービスとの IPC、ホットリロード可能なプラグイン。

## 2. ABI・データレイアウトの要約
- 詳細は `a-jit.md` の「5.0 ターゲット ABI / データレイアウト」を参照。
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
- `reml-backlog.md` に追記すべき課題：構造体の `repr(packed)` 対応、マルチリリースの互換テスト、C++ name mangling のガイド。

---

> **ドラフト状態**: 本ハンドブックはフェーズ0で骨子を作成した段階。各セクションはフェーズ1以降の PoC とレビュー結果に合わせて詳細化する。
