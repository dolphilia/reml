# 1.8 Backend/Runtime 対応計画（trait/impl/AsyncStream 系 / 2025-12-27）

Frontend 側で受理した `trait` / `impl` / `Self` / 修飾子付き関数名 / `AsyncStream` などの構文・型名が、Backend/Runtime の実装段階で破綻しないよう、必要な整合タスクを先行で整理するための計画書。

## 目的
- trait/impl/associated type を含む宣言を MIR/Backend で表現可能にする。
- `Self` / `Self::Error` 等の参照が Backend で解決される状態へ繋げる。
- `AsyncStream` / `Future` を Runtime/Backend で受け止めるための型枠と拡張方針を確保する。

## 対象範囲
- Backend: `compiler/backend/llvm/src/*`
- Runtime: `compiler/runtime/src/prelude/*`
- Frontend 関連: `compiler/frontend/src/semantics/*`, `compiler/frontend/src/typeck/*`
- 仕様: `docs/spec/3-1-core-prelude-iteration.md`

## 背景
- Frontend は `trait` / `impl` / associated type / `Self` / 修飾子付き関数名 / 0 引数ラムダ / `struct Foo;` を受理できるようになった。
- 仕様サンプルの通過は Frontend の範囲で完了しているが、Backend/Runtime はこれらの構文を扱う設計が未整備。
- `AsyncStream` / `Future` は仕様に登場するが、Runtime/Backend では実体型・橋渡しが未定義。

## 実装修正計画（正式）

### フェーズ 1: 影響範囲の確定（MIR/IR と Runtime）
1) MIR への表現追加要否の確認
- 目的: trait/impl/associated type を MIR に落とす必要があるか判断する。
- 作業ステップ:
  - `compiler/frontend/src/semantics` の MIR 定義と lowering 経路を整理し、該当構文がどのレイヤで失われるかを記録する。
  - `trait` / `impl` / `associated type` / `Self` のうち、codegen が必要とする情報（具体型、関連型解決結果、impl 対応表）を列挙する。
  - 既存の Backend 側 MIR 連携（`1-7-backend-runtime-*` 系計画）の出力項目と突き合わせ、追加が必要な最小メタデータを定義する。
  - 「MIR に残す vs typeck の結果参照」の判断基準を明文化し、採用方針を決定する。
  - 影響範囲を `docs/plans/docs-examples-audit/1-8-backend-runtime-trait-async-plan-20251227.md` 内に追記し、後続フェーズの入力にする。

2) Runtime での型枠の有無を確認
- 目的: `AsyncStream` / `Future` / `Collector` などの型が Runtime 側に必要か判断する。
- 作業ステップ:
  - `compiler/runtime/src/prelude` の公開型、feature gate、Stage/Capability 連動箇所を棚卸しする。
  - `AsyncStream` / `Future` / `Collector` の仕様上の責務と、Runtime 側で必要な最小 API 面を整理する。
  - 仕様と Runtime の差分（型名、メソッド名、エフェクト連動）を一覧化する。
  - 空実装（opaque 型 / trait stub / feature gate）で先に整合できるか判断し、必要な設計制約をまとめる。
  - FFI/Bridge 連携の影響（型が露出する範囲、ABI 依存の有無）を記録する。

#### フェーズ 1 調査メモ

##### MIR / Lowering の現状
- Parser AST は `TraitDecl` / `ImplDecl` / `TraitItem` / `ImplItem` を保持する（`compiler/frontend/src/parser/ast.rs`）。
- typeck の出力は `TypedModule`（`compiler/frontend/src/semantics/typed.rs`）で、関数/Active Pattern/Conductor と `DictRef` / `SchemeInfo` のみ。trait/impl 宣言や associated type 宣言は保持されない。
- `TypecheckReport` には `constraints` / `used_impls` / `dict_refs` があるが、`MirModule` 生成は `TypedModule` 由来であり、MIR 側には `dict_ref_ids` だけが残る（`compiler/frontend/src/semantics/mir.rs`）。
- MIR 型は文字列トークンのみで、`normalize_mir_type_label` は `Int`/`Unit` の正規化のみ。`Self` / `Self::Error` / associated type の元情報は失われる。
- `TypedExprKind` / `MirExprKind` に `ModulePath` や qualified 名の情報は存在せず、`Type.method` / `Type::method` / `Trait::method` の区別は MIR に残らない。
- Backend 側の MIR 取り込み（`compiler/backend/llvm/src/integration.rs`）は `MirModuleSpec` に関数情報のみを要求し、trait/impl 追加情報を受け取る経路がない。

##### codegen が必要とする最小メタデータ（候補）
- `dict_ref_ids` の実体（`impl_id` / `requirements` / `ty` / `span`）と、`impl_id` に対応する trait/impl 情報。
- `Self` / associated type の解決結果（具体型名の確定、`Self::Error` 等の展開結果）。
- qualified 関数名の解決結果（`Type.method` / `Type::method` / `Trait::method` を区別するための種別と元名）。

##### 「MIR に残す vs typeck の結果参照」判断と方針
- Backend は MIR JSON のみを入力にするため、**typeck 内部の `constraints` / `used_impls` を参照する前提は置けない**。
- 採用方針: **MIR へ最小限の trait/impl メタデータを追加する**（または MIR と同じスナップショットに sidecar で `dict_refs`/`impls` を出力し、Backend が読み取れる形にする）。どちらの場合も JSON で完結させる。

##### Runtime の現状（Async/Collector）
- Runtime Prelude は `Iter<T>` と Stage/Capability/Efffect ラベルを実装済み（`compiler/runtime/src/prelude/iter/mod.rs`）。`io_async` / `async_pending` などの効果ラベルは存在するが、非同期型は未定義。
- `Collector` trait は `compiler/runtime/src/prelude/collectors/mod.rs` に存在し、`type Error` / `new` / `with_capacity` / `push` / `reserve` / `finish` / `into_inner` / `iter_error` を持つ。仕様にない `iter_error` を追加している点が差分。
- `AsyncStream` / `Future` の型定義は `compiler/runtime/src/prelude` 配下に存在しない。`compiler/runtime/Cargo.toml` に `core_async` などの feature gate も未設定。
- 仕様側（`docs/spec/3-1-core-prelude-iteration.md`, `docs/spec/3-9-core-async-ffi-unsafe.md`）の非同期型と Runtime の差分は **型宣言が欠落**している点に集約される。先行整合には opaque 型 + feature gate の追加が必要。

### フェーズ 2: Backend 側の表現・コード生成
1) MIR 型表現の拡張
- 目的: trait/impl/associated type を Backend で扱える型情報へ変換する。
- 作業ステップ:
  - フェーズ 1 の判断に基づき、MIR への追加フィールド（trait 名、impl 対応表、associated type 解決結果）を定義する。
  - `Self` の解決ルール（impl の具体型、型引数束縛）を文書化し、MIR または typeck 参照のどちらで担保するか決定する。
  - Backend 側で使用する型 ID/参照のライフサイクルを整理し、IR 生成時の参照方法を確定する。
  - データの持ち方が決定したら、既存の型レイアウト計画との整合チェックリストを作成する。

2) 修飾子付き関数名のコード生成ルール
- 目的: `Type.method` / `Type::method` の IR 名規約を統一する。
- 作業ステップ:
  - Backend の命名規則（既存の symbol 名エンコード）を確認し、qualified 名の分解単位を確定する。
  - `Type.method` / `Type::method` / `Trait::method` の差分が識別できる命名ルールを定義する。
  - LLVM のサニタイズ規則と衝突しない文字集合を選定する。
  - 既存の symbol 名と衝突しないか、想定例でチェックする。

#### フェーズ 2 決定メモ

##### MIR 拡張方針（JSON 追加）
- **採用方針**: MIR JSON のトップレベルに `dict_refs` / `impls` / `qualified_calls` を追加し、Backend は追加フィールドを読み取れるよう拡張する（sidecar 方式は使わない）。
- `dict_refs`: `TypedModule.dict_refs` をそのまま出力する。
  - 形: `{ "id": 0, "impl_id": "iterator::map::Iter", "requirements": ["effect {mem}"], "ty": "Iter<Int>", "span": {...} }`
- `impls`: `impl_id` をキーに、trait/impl の最小情報を保持する。
  - 形: `{ "impl_id": "iterator::map::Iter", "trait": "Iterator", "target": "Iter<T>", "associated_types": [{ "name": "Item", "ty": "T" }], "methods": ["map", "filter"], "span": {...} }`
  - `trait` は trait impl のみ必須。inherent impl は `trait = null` とする。
- `qualified_calls`: `Type.method` / `Type::method` / `Trait::method` の解決結果を保持し、Backend のシンボル解決で利用する。
  - 形: `{ "expr_id": 120, "kind": "type_method", "owner": "Iter<T>", "name": "map", "impl_id": "iterator::map::Iter" }`
  - `kind` は `type_method` / `type_assoc` / `trait_method` の 3 種を想定。

##### `Self` / associated type の解決ルール
- `Self` / `Self::X` は **typeck で具体型へ解決した結果を MIR に出力**する。未解決（ジェネリクス残り）の場合は `Self` を残し、Backend 側で `TODO` 診断を出す。
- `associated_types` は **型名文字列で確定**させる。`Self::Error` のような表記は MIR へ残さない。

##### 型 ID / 参照のライフサイクル
- `impl_id` は `dict_refs` / `impls` / `qualified_calls` の共通キーとして扱う。
- `impl_id` の命名規則: `trait` 実装は `{TraitName}::{TargetType}`、inherent impl は `{TargetType}`。型引数は `T` を含めた文字列で保持する。

##### qualified 関数名の IR 命名規約
- 既存の `sanitize_llvm_ident` を前提に、**種別と所有者を埋め込んだ記号名**を採用する。
- 形式: `@reml_fn__{kind}__{owner}__{name}`
  - `kind`: `type_method` / `type_assoc` / `trait_method`
  - `owner`: `Iter<T>` / `Option<T>` / `TraitName` など
- 例:
  - `Iter.map` -> `@reml_fn__type_method__Iter_T__map`
  - `Iter::from_list` -> `@reml_fn__type_assoc__Iter_T__from_list`
  - `Iterator::next` -> `@reml_fn__trait_method__Iterator__next`
- `owner` / `name` は `sanitize_llvm_ident` でエンコードする（`<`/`>`/`::` は `_uXXXX` へ変換）。

### フェーズ 3: Runtime 側の拡張準備
1) Async 系の型定義（暫定）
- 目的: `AsyncStream<T>` / `Future<T>` を Runtime で参照可能にする。
- 作業ステップ:
  - `compiler/runtime/src/prelude` に async 系型の宣言位置を決め、feature gate の運用ルールを定義する。
  - `AsyncStream<T>` / `Future<T>` の最小 API（型名、型パラメータ、必要なトレイト境界）を整理する。
  - Stage/Capability の gate と `effect {io.async}` の対応表を作成し、監査ログのキー案を決める。
  - 将来の実装に備えた拡張余地（poll 互換、ランタイム実装差）を整理する。

2) Collector/Iter の trait 化検討
- 目的: 仕様の `trait Collector` に合わせ、Runtime のコレクタ実装を trait ベースへ移行できるか確認する。
- 作業ステップ:
  - `compiler/runtime/src/prelude/collectors` の現行 API を棚卸しし、trait 化の差分を洗い出す。
  - 仕様の `Collector` 要件（メソッド、関連型、関連エフェクト）をチェックリスト化する。
  - 移行パターン（互換層を挟む / 新 API を追加する / 既存 API を置換）の比較表を作成する。
  - 移行による破壊的変更の有無を明記し、必要なら段階移行の方針を決める。

#### フェーズ 3 決定メモ

##### Async 系の型定義（暫定）
- **追加位置**: `compiler/runtime/src/prelude/mod.rs` 直下に `async` モジュールを追加し、`pub mod async;` を公開する。
- **feature gate**: `core_async` / `core-async` を `compiler/runtime/Cargo.toml` に追加し、`prelude::async` は `#[cfg(feature = "core_async")]` で公開する。
- **暫定型**:
  - `pub struct Future<T> { _opaque: PhantomData<T> }`
  - `pub struct AsyncStream<T> { _opaque: PhantomData<T> }`
- **最小 API**（宣言のみ / 実装は空）:
  - `impl<T> Future<T> { pub fn new_opaque() -> Self }`
  - `impl<T> AsyncStream<T> { pub fn new_opaque() -> Self }`
- **Stage/Capability**: `effect {io.async}` に対応する監査キーは `runtime.async.*` を予約し、Collector/Iter の `io_async` と衝突しない命名にする。

##### Collector/Iter の trait 化方針
- Runtime 側は **既存の `Collector` trait を維持し、仕様との差分は明記**する（`iter_error` は互換のため残す）。
- 仕様準拠の署名を満たすため、将来的に `Collector` に `impl Collector<T, C> for ...` の互換層を追加する計画とし、API 破壊は発生させない。
- 既存 `Iter.try_collect` は `Collector::iter_error` 依存のため、仕様版 `Collector` を導入する場合は `try_collect` 側にアダプタを追加する。

### フェーズ 4: 検証とサンプル連携
- Backend/Runtime の暫定拡張が Frontend のサンプル受理に影響しないことを確認する。
- MIR/IR での表現が欠落する場合は、診断や TODO を残し将来タスクへつなぐ。
- 仕様サンプルの想定入力から、Backend で落ちるケースを洗い出しリスト化する。
- 成果物のドキュメント化（影響範囲、命名規約、型枠仕様）を完了させる。

#### フェーズ 4 決定メモ

##### 検証観点（Backend）
- MIR JSON で `dict_refs` / `impls` / `qualified_calls` が追加されても、既存の Backend スナップショット読み込みが失敗しないことを確認する（未知フィールドは無視される前提）。
- `qualified_calls` が欠落している場合は、Backend 側で `TODO: qualified call unresolved` の診断（もしくはログ）を出して失敗しない扱いにする。
- `impls` の `associated_types` が未解決の場合は、`Self` 展開が未完了として TODO を記録し、IR 生成は継続する。

##### 検証観点（Runtime）
- `core_async` feature が無効のとき、既存 Prelude/Iter のビルドとテストが継続できることを確認する。
- `core_async` 有効時に `prelude::async` が公開され、型名の衝突が起きないことを確認する（Opaque 型のみのため ABI 影響は発生しない想定）。

##### サンプル連携（docs-examples-audit）
- `docs/spec/3-1-core-prelude-iteration.md` の async 連携サンプル（`from_async_stream` / `to_async_stream`）を対象に、Runtime 側が未実装であることを明記した TODO を `docs/plans/docs-examples-audit/1-8-backend-runtime-trait-async-plan-20251227.md` に残す。
- trait/impl/qualified 名を含むサンプル（`docs/spec` の trait/impl 節）を「Backend 未対応のため IR で未解決扱い」として `docs-examples-audit` のチェック対象に追加する。

## 受け入れ基準
- Backend 側で `Self` / qualified function 名 / trait 情報の保持方針が文書化されている。
- Runtime 側で `AsyncStream` / `Future` / Collector trait 化の方針が定義されている。
- MIR/Runtime の差分一覧が本計画書に反映され、後続作業が参照できる状態になっている。
- 既存の Backend/Runtime テストにリグレッションがない（実装段階で確認）。

## 進捗管理
- 本計画書作成日: 2025-12-27
- 進捗欄（運用用）:
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [x] フェーズ 3 完了
  - [x] フェーズ 4 完了

## 関連リンク
- `docs/spec/3-1-core-prelude-iteration.md`
- `docs/plans/docs-examples-audit/1-7-frontend-mir-type-token-plan-20251227.md`
- `docs/plans/docs-examples-audit/1-7-backend-runtime-type-decl-layout-plan-20251227.md`
