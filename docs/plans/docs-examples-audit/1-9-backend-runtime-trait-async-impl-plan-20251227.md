# 1.9 Backend/MIR JSON 拡張取り込み & Runtime Async 土台実装計画（2025-12-27）

`1-8-backend-runtime-trait-async-plan-20251227.md` の決定事項を実装に反映するための実行計画。Backend は MIR JSON 拡張の取り込み、Runtime は async 型の土台追加に着手する。

## 目的
- Backend が MIR JSON の `dict_refs` / `impls` / `qualified_calls` を取り込み、未解決情報を許容する動作にする。
- Runtime に `Future<T>` / `AsyncStream<T>` の opaque 型と feature gate を追加し、Prelude から参照可能にする。
- docs-examples-audit の検証フローに、拡張の未実装・未解決を明示できる状態を作る。

## 対象範囲
- Backend: `compiler/rust/backend/llvm/src/integration.rs`, `compiler/rust/backend/llvm/src/codegen.rs`
- Frontend (JSON 生成): `compiler/rust/frontend/src/semantics/mir.rs`, `compiler/rust/frontend/src/bin/reml_frontend.rs`
- Runtime: `compiler/rust/runtime/src/prelude/*`, `compiler/rust/runtime/Cargo.toml`
- 仕様・計画: `docs/spec/3-1-core-prelude-iteration.md`, `docs/spec/3-9-core-async-ffi-unsafe.md`, `docs/plans/docs-examples-audit/1-8-backend-runtime-trait-async-plan-20251227.md`

## 前提
- MIR JSON へ `dict_refs` / `impls` / `qualified_calls` を追加する（sidecar 方式は採用しない）。
- `Self` / associated type の解決は typeck で確定し、未解決は TODO として Backend で扱う。
- Runtime の async 型は opaque で提供し、ABI 影響を避ける。

## 実行計画

### フェーズ 0: スキーマ拡張の設計確定
- [ ] MIR JSON のトップレベルに追加するフィールドを定義する。
  - `dict_refs`: `TypedModule.dict_refs` の列
  - `impls`: `impl_id` をキーとする trait/impl 情報
  - `qualified_calls`: `expr_id` をキーにした解決結果
  - 作業ステップ:
    - `docs/plans/docs-examples-audit/1-8-backend-runtime-trait-async-plan-20251227.md` の決定メモと整合する JSON 形状を確認する。
    - 既存の MIR JSON 出力例（過去スナップショットやサンプル）を洗い出し、互換性のある追加方式を整理する。
    - `impl_id` / `expr_id` の命名・採番規則を明文化し、Backend 側のキー参照と衝突しないかを確認する。
    - JSON 形状の例を 2-3 パターン作成し、空/未解決時の表現を確定する。
- [ ] Backend 側の JSON デシリアライズ構造体（`MirModuleSpec`）に受け取り口を追加する。
  - 作業ステップ:
    - `compiler/rust/backend/llvm/src/integration.rs` の構造体定義を確認し、追加フィールドの型とデフォルト方針を決める。
    - `Option<T>` / `Vec<T>` のどちらが自然かを検討し、追加フィールドなし入力の許容条件を確定する。
    - 既存の JSON パーサ実装で未知フィールドを無視する挙動を再確認する。
- [ ] 既存スナップショットが壊れないよう、未知フィールドは無視されることを確認する。
  - 作業ステップ:
    - serde の `deny_unknown_fields` 設定有無を確認する。
    - 既存 JSON を入力にした場合のデシリアライズ挙動を手順として記録する。
    - スナップショット更新が不要なことを確認できる検証観点を列挙する。

### フェーズ 1: Frontend の MIR JSON 拡張
- [ ] `MirModule` 生成時に `dict_refs` / `impls` / `qualified_calls` を追加出力できるように拡張する。
  - 作業ステップ:
    - `compiler/rust/frontend/src/semantics/mir.rs` の `MirModule` 生成経路を棚卸しし、追加フィールドの生成タイミングを決める。
    - `TypedModule` / `TypecheckReport` から必要情報を収集するための参照経路を整理する。
    - `qualified_calls` の `kind` 判定ロジックと未解決時の値（空/unknown）を決める。
- [ ] `reml_frontend` の JSON 出力に追加フィールドが出ることを確認する。
  - 作業ステップ:
    - `compiler/rust/frontend/src/bin/reml_frontend.rs` の JSON 出力経路を確認し、追加フィールドの serialize が有効か確認する。
    - 代表的なサンプル（trait/impl/qualified 名を含む）で JSON 出力差分を観察する。
    - 出力例を docs-examples-audit のメモへ記録する。
- [ ] 追加フィールドが空でも JSON 形状が安定するよう default を設ける。
  - 作業ステップ:
    - 空配列/空 map のどちらを採用するかを決め、出力時に常に含める方針を確定する。
    - `serde(default)` などの利用方針を整理し、空値でも互換性が保てるようにする。

### フェーズ 2: Backend の MIR JSON 取り込み
- [ ] `integration.rs` に `dict_refs` / `impls` / `qualified_calls` を取り込む型定義を追加する。
  - 作業ステップ:
    - Frontend 側 JSON 形状に合わせた struct を定義し、serde の型一致を確認する。
    - `impls` の内部構造（trait 名、target、associated_types、methods）の必須/任意項目を決める。
    - `qualified_calls` の `kind` を enum 化し、未知値の fallback 方針を決める。
- [ ] `qualified_calls` が未解決の場合は TODO 診断を出す（IR 生成は継続）。
  - 作業ステップ:
    - 未解決判定の条件（`kind` 欠落/unknown、owner 不明など）を定義する。
    - TODO 診断のメッセージと識別子（既存の診断規約に合わせる）を決める。
    - 生成継続のための fallback（既存のシンボル解決ルールへの委譲）を明示する。
- [ ] `impls` に associated type が欠落している場合も TODO 診断を出す。
  - 作業ステップ:
    - `associated_types` が空/欠落のときの取り扱い条件を定義する。
    - `Self::X` 展開が未完了である旨を診断に含めるか検討する。
- [ ] 既存の JSON 入力が追加フィールドなしでも動作することを確認する。
  - 作業ステップ:
    - 追加フィールドが空/欠落の入力を流すテストケースを想定し、確認手順を記録する。
    - `Option`/default が機能しない場合の修正方針（serde 属性追加）を整理する。

### フェーズ 3: Runtime async 型の土台追加
- [ ] `compiler/rust/runtime/Cargo.toml` に `core_async` / `core-async` feature を追加する。
  - 作業ステップ:
    - 既存 feature 名との衝突有無を確認し、命名規約に合わせた定義を追加する。
    - `core_async` と `core-async` の alias 方針（features 別名）を決める。
- [ ] `compiler/rust/runtime/src/prelude/async.rs` を追加し、`Future<T>` / `AsyncStream<T>` を opaque 型で定義する。
  - 作業ステップ:
    - `PhantomData` を用いた opaque 型の最小定義を作成し、公開 API を最小限に留める。
    - 仕様書の型名と一致することを確認し、将来の拡張余地を注記する。
- [ ] `compiler/rust/runtime/src/prelude/mod.rs` に `#[cfg(feature = "core_async")] pub mod async;` を追加する。
  - 作業ステップ:
    - 既存の prelude モジュール構成を確認し、露出順と命名衝突がないことを確認する。
- [ ] `core_async` 無効時に既存ビルドが壊れないことを確認する。
  - 作業ステップ:
    - feature 未指定時のコンパイルに影響しないよう `cfg` の適用範囲を確認する。
    - 既存 tests/build コマンドの影響範囲を整理する。

### フェーズ 4: docs-examples-audit 連携
- [ ] async 連携サンプル（`docs/spec/3-1-core-prelude-iteration.md` の `from_async_stream` / `to_async_stream`）に対し、Runtime 未実装の TODO を `1-9` へ記録する。
  - 作業ステップ:
    - 仕様内の対象サンプルを特定し、未実装事項の粒度（型未定義 / 実装未提供）を整理する。
    - TODO 記載のフォーマット（見出し/チェック項目/リンク）を決める。
- [ ] trait/impl/qualified 名サンプルを Backend 未対応チェック対象として整理する。
  - 作業ステップ:
    - 仕様の該当サンプルを列挙し、Backend 側で未解決になる理由をメモ化する。
    - 検証対象の一覧に落とし込み、後続タスクへ引き継げる形に整形する。

## 受け入れ基準
- MIR JSON に `dict_refs` / `impls` / `qualified_calls` が追加され、Backend が読み込み可能。
- Backend は未解決の qualified call / associated type を TODO 扱いで通過できる。
- Runtime に `core_async` feature と opaque 型が追加され、Prelude から参照できる。
- docs-examples-audit で未対応箇所が記録され、後続の実装計画に引き継げる。

## 進捗管理
- 本計画書作成日: 2025-12-27
- 進捗欄（運用用）:
  - [ ] フェーズ 0 完了
  - [ ] フェーズ 1 完了
  - [ ] フェーズ 2 完了
  - [ ] フェーズ 3 完了
  - [ ] フェーズ 4 完了

## 関連リンク
- `docs/plans/docs-examples-audit/1-8-backend-runtime-trait-async-plan-20251227.md`
- `docs/spec/3-1-core-prelude-iteration.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`
