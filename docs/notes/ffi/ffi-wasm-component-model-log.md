# WIT / WASM Component Model 連携 調査ログ

## 目的
WASM Component Model（WIT）の導入可能性を調査し、Reml FFI の将来拡張に向けた論点と型対応表（一次案）を残す。

## 関連資料
- 設計計画: `docs/plans/ffi-improvement/1-3-wasm-component-model-plan.md`
- 既存仕様: `docs/spec/3-9-core-async-ffi-unsafe.md`
- 調査ガイド: `docs/guides/ffi/ffi-wit-poc.md`
- 既存調査: `docs/notes/ffi/ffi-improvement-survey.md`

## WIT 型 → Reml 型 対応表（一次案）

| WIT 型 | Reml 型案 | 補足 |
| --- | --- | --- |
| `bool` | `Bool` | そのまま対応。 |
| `s8` / `u8` | `i8` / `u8` | 整数幅は固定。 |
| `s16` / `u16` | `i16` / `u16` | 同上。 |
| `s32` / `u32` | `i32` / `u32` | 同上。 |
| `s64` / `u64` | `i64` / `u64` | 同上。 |
| `float32` / `float64` | `f32` / `f64` | NaN の扱いは Canonical ABI 仕様に依存。 |
| `char` | `Char` | Unicode scalar value を前提とする。 |
| `string` | `Str` | Canonical ABI での UTF-8 変換と境界コピーが前提。 |
| `list<T>` | `List<T>` | `T = u8` の場合は `Core.Text.Bytes` も候補。 |
| `record` | `{ field: T, ... }` | フィールド名は WIT を優先。 |
| `variant` | `enum` | タグ順序とラベル名の保持が必須。 |
| `flags` | `Set<FlagEnum>` | `FlagEnum` は WIT の flags 名を列挙した enum。生ビット保持は要検討。 |
| `enum` | `enum` | 文字列表現ではなくタグ順を維持する。 |
| `union` | `variant`（暫定） | 共有レイアウト前提のため、境界検証とタグ付けの方式を要検討。 |
| `option<T>` | `Option<T>` | `None` は WIT の `null` 相当。 |
| `result<Ok, Err>` | `Result<Ok, Err>` | `Err` は FFI 監査ログへ付随情報を保持。 |
| `tuple<T...>` | `(T1, T2, ...)` | 要素順を維持する。 |
| `resource` | `FfiResourceHandle` | 不透明ハンドル型として扱い、内部表現は Capability と結合する。 |
| `own<T>` | `Owned<T>` | `Ownership::Owned` と対応させ、解放責務を Reml 側へ移譲する。 |
| `borrow<T>` | `Borrow<T>` | 既存の借用型に合わせ、`Ownership::Borrowed` と対応させる。 |
| `future<T>` | `Future<T>` | `Core.Async` の型と効果タグに接続する。 |
| `stream<T>` | `AsyncStream<T>` | `Core.Async` のストリーム型へ写像する。 |

## 境界安全性とメモリモデルの論点
- Shared Nothing 前提のため、境界を跨ぐデータはコピーまたはシリアライズを伴う。
- `string` / `list<T>` は所有権の移動と寿命管理を WIT 側に合わせて明示する必要がある。
- Reml 側では `effect {ffi}` と監査ログを維持し、WIT 経由であっても監査対象とする。
- 例外・パニックは境界外に伝播しない前提のため、`Result` へのマッピング規約を明文化する。

## Canonical ABI の取り込み観点
- 文字列は UTF-8 を前提とし、境界でのバッファ確保と解放責務を明示する。
- `record` / `variant` の 레イアウトは Canonical ABI の lift/lower ルールへ合わせる。
- `list<T>` は長さと要素サイズの検証を必須とし、上限超過時の診断キーを設計する。

## Canonical ABI と既存 FFI の差分（Shared Nothing 観点）
- 既存 FFI はプロセス内 ABI 前提でポインタ共有が可能だが、WIT は境界越えをコピー前提とする。
- WIT は `resource` をハンドルで管理し、ライフサイクルが Canonical ABI で明文化される。既存 FFI の生ポインタとは扱いが異なる。
- `string` / `list<T>` は境界でのメモリ確保と解放責務が規定され、所有権移譲が暗黙にならない。
- 例外・パニックは境界を越えないため、`result` を前提としたエラー変換が必須になる。
- ABI 互換は言語間の安定性を優先し、プラットフォーム差異は ABI 層で吸収する必要がある。

## ツール連携の想定フロー（一次案）
1. WIT 定義を受け取り、Reml 向けのバインディング草案を生成する。
2. 生成物は `Core.Ffi.Dsl` でラップし、`unsafe` 境界を局所化する。
3. 監査ログには `ffi.wit` 系キーで WIT 由来の型情報を記録できるよう拡張する（要検討）。

## `resource` / `own` / `borrow` の扱い（一次案）
- `resource` は Reml からは不透明なハンドル型として扱い、型名を `resource` 名から派生させる。内部実装は Capability と紐づく実体を保持する想定。
- `own<T>` は所有権の移譲を意味し、Reml 側が解放責務を持つ。`drop` 相当の解放フックを `ffi` 監査ログへ記録する。
- `borrow<T>` は借用であり、呼び出し境界を超えて保持しない。`borrow` は `unsafe` 境界内でのみ許可し、参照はスコープ外へ逃がさない制約を追加する。
- `own` / `borrow` の変換規約は `Result` のエラー分岐で必ず監査情報を残し、境界での解放漏れを検知できるようにする。
- `resource` を `Core.Ffi.Dsl` でラップする場合は、`ffi.wrap_resource` のような専用 API を追加する前提で検討する（調査段階）。

## 追加調査 TODO
- `future` / `stream` など非同期型を `Core.Async` と接続する場合の効果タグ設計。
- 診断キーを `docs/spec/3-6-core-diagnostics-audit.md` の命名規則へ揃える方針。
