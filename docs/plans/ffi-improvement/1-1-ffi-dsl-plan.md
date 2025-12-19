# Phase 2: Core.Ffi.Dsl 設計・仕様化

## 背景
- `reml-bindgen` が生成する低レベル定義は `unsafe` であり、
  利用者向けには **安全な DSL レイヤ**が必要。
- OCaml `ctypes` のように Reml で宣言的に FFI を記述できる仕組みが
  DSL ファーストの方針に合致する。

## スコープ
- `Core.Ffi.Dsl` の API と型システム、効果境界の設計。
- DSL から `extern` を生成する方法（静的/動的）を定義。

## 成果物
- `Core.Ffi.Dsl` の API 仕様（型、関数、例）
- `effect {ffi}` / `effect {unsafe}` の境界規則
- `docs/spec/3-9-core-async-ffi-unsafe.md` の拡張案
- `examples/ffi` の DSL 例

## 仕様検討項目
1. **API 形状（案）**
   ```reml
   let lib = ffi.bind_library("m")
   let cos = lib.bind_fn("cos", ffi.double -> ffi.double)
   let safe_cos = ffi.wrap(cos, effect {ffi})
   ```
2. **型表現**
   - `ffi.int`, `ffi.double`, `ffi.ptr(ffi.char)` などの型 DSL
   - `struct`/`enum` の宣言 DSL
3. **安全境界の明示**
   - `ffi.bind_fn` は `effect {unsafe, ffi}`
   - `ffi.wrap` で `unsafe` を隠蔽する場合の責務を明文化
4. **エラーハンドリング**
   - `Result` ベースの変換（ヌルポインタ、戻り値検証）
   - 失敗時の診断キー定義

## 実装ステップ
1. `Core.Ffi.Dsl` の API 一覧（`bind_library` / `bind_fn` / `wrap` / 型 DSL）と型定義を `docs/spec/3-9-core-async-ffi-unsafe.md` に追記する。
2. `ffi.wrap` の責務（エラーチェック、所有権/ライフタイムの前提）と監査ログ要件を `docs/spec/3-6-core-diagnostics-audit.md` に整理する。
3. `examples/ffi` に DSL 例を追加し、`unsafe` 直呼びと `ffi.wrap` の対比サンプルを用意する。
4. `docs/guides/ffi-dsl-guide.md` に FFI DSL 導入ガイドを新設し、Phase 1 の `reml-bindgen` 生成物との併用フローを明記する。

## 進捗
- ステータス: `confirmed`
- 実装ステップ 1: `docs/spec/3-9-core-async-ffi-unsafe.md` に `Core.Ffi.Dsl` 節と API/型定義、利用例を追加済み。
- 実装ステップ 2: `docs/spec/3-6-core-diagnostics-audit.md` に `ffi.wrap` の監査メタデータ・診断キーを追加済み。
- 実装ステップ 3: `examples/ffi/dsl` に `unsafe` 直呼びと `ffi.wrap` の対比サンプルを追加済み。
- 実装ステップ 4: `docs/guides/ffi-dsl-guide.md` を更新し、`reml-bindgen` 併用フローを明記済み。

## 依存関係
- `docs/spec/3-9-core-async-ffi-unsafe.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`

## リスクと対策
- **安全境界が曖昧**: `ffi.wrap` の責務を明文化し、
  監査ログで `ffi.wrapper.*` を記録する。
- **DSL API が肥大化**: Phase 2 では必須 API に限定し、拡張は Phase 3 以降へ分割する。

## 完了判定
- `Core.Ffi.Dsl` の API と効果境界（`effect {ffi, unsafe}`）が仕様に反映されている。
- `ffi.wrap` の責務と監査ログ要件が `docs/spec/3-6-core-diagnostics-audit.md` に反映されている。
- DSL サンプルが `examples/ffi` に追加されている。
