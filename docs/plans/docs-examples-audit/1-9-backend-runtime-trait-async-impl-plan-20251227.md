# 1.9 Backend/MIR JSON 拡張取り込み & Runtime Async 土台実装計画（2025-12-27）

`1-8-backend-runtime-trait-async-plan-20251227.md` の決定事項を実装に反映するための実行計画。Backend は MIR JSON 拡張の取り込み、Runtime は async 型の土台追加に着手する。

## 目的
- Backend が MIR JSON の `dict_refs` / `impls` / `qualified_calls` を取り込み、未解決情報を許容する動作にする。
- Runtime に `Future<T>` / `AsyncStream<T>` の opaque 型と feature gate を追加し、Prelude から参照可能にする。
- docs-examples-audit の検証フローに、拡張の未実装・未解決を明示できる状態を作る。

## 対象範囲
- Backend: `compiler/backend/llvm/src/integration.rs`, `compiler/backend/llvm/src/codegen.rs`
- Frontend (JSON 生成): `compiler/frontend/src/semantics/mir.rs`, `compiler/frontend/src/bin/reml_frontend.rs`
- Runtime: `compiler/runtime/src/prelude/*`, `compiler/runtime/Cargo.toml`
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
    - `compiler/backend/llvm/src/integration.rs` の構造体定義を確認し、追加フィールドの型とデフォルト方針を決める。
    - `Option<T>` / `Vec<T>` のどちらが自然かを検討し、追加フィールドなし入力の許容条件を確定する。
    - 既存の JSON パーサ実装で未知フィールドを無視する挙動を再確認する。
- [ ] 既存スナップショットが壊れないよう、未知フィールドは無視されることを確認する。
  - 作業ステップ:
    - serde の `deny_unknown_fields` 設定有無を確認する。
    - 既存 JSON を入力にした場合のデシリアライズ挙動を手順として記録する。
    - スナップショット更新が不要なことを確認できる検証観点を列挙する。

#### フェーズ 0 決定メモ（MIR JSON スキーマ拡張）
- **追加フィールドの配置**: MIR JSON トップレベルに `dict_refs` / `impls` / `qualified_calls` を追加する（sidecar 方式は採用しない）。
- **スキーマの互換性**: 既存の JSON（例: `reports/spec-audit/ch1/mir-output-type-sample-20251224.json` / `reports/spec-audit/ch1/mir-json-type-sample-20251224.json`）は追加フィールド欠落でも読み込み可能とする。`schema_version` は当面 `frontend-mir/0.2` を維持し、追加フィールドは optional + default で扱う。

##### 1) `dict_refs`（配列）
- **目的**: `dict_ref_ids` 参照の実体テーブルを MIR JSON で共有する。
- **型**: `dict_refs: [DictRefJson, ...]`
- **要素構造（暫定）**:
  - `id: u32`
  - `impl_id: string`
  - `requirements: [string, ...]`
  - `ty: string`
  - `span: { start: u32, end: u32 } | null`
- **空時の扱い**: 常に `[]` を出力する（欠落は許容するが推奨しない）。

##### 2) `impls`（`impl_id` をキーとする map）
- **目的**: `impl_id` から trait/impl の最小情報を引けるようにする。
- **型**: `impls: { "<impl_id>": ImplSpecJson, ... }`
- **値構造（暫定）**:
  - `trait: string | null`（inherent impl の場合は `null`）
  - `target: string`
  - `associated_types: [{ name: string, ty: string }, ...]`
  - `methods: [string, ...]`
  - `span: { start: u32, end: u32 } | null`
- **空時の扱い**: 常に `{}` を出力する（欠落は許容するが推奨しない）。

##### 3) `qualified_calls`（function+expr_id をキーとする map）
- **目的**: `Type.method` / `Type::method` / `Trait::method` の解決結果を保持し、Backend 側でシンボル解決に利用する。
- **型**: `qualified_calls: { "<fn_name>#<expr_id>": QualifiedCallJson, ... }`
- **値構造（暫定）**:
  - `kind: "type_method" | "type_assoc" | "trait_method" | "unknown"`
  - `owner: string | null`
  - `name: string | null`
  - `impl_id: string | null`
  - `span: { start: u32, end: u32 } | null`
- **キー規則**:
  - `expr_id` は `MirExpr.id` と同一（関数内で一意）。モジュール全体で衝突を避けるため `"<fn_name>#<expr_id>"` 形式にする。
  - `fn_name` は MIR JSON の関数名と一致させる（`@` 付きもそのまま使用）。
- **未解決時の表現**: `kind = "unknown"` を設定し、`owner` / `name` / `impl_id` は `null` を許容する。
- **空時の扱い**: 常に `{}` を出力する（欠落は許容するが推奨しない）。

##### `impl_id` / `expr_id` 命名規則（採番ルール）
- `impl_id`: `trait` 実装は `{TraitName}::{TargetType}`、inherent impl は `{TargetType}`。型引数（`T` 等）は文字列として含める。
- `expr_id`: 既存 MIR の `exprs` が持つ `id` をそのまま使用し、関数名と組でユニーク性を担保する。

##### 例（追加フィールドの JSON 断片）
```json
{
  "dict_refs": [
    {
      "id": 0,
      "impl_id": "Iterator::Iter<T>",
      "requirements": ["effect {io.async}"],
      "ty": "Iter<T>",
      "span": { "start": 120, "end": 160 }
    }
  ],
  "impls": {
    "Iterator::Iter<T>": {
      "trait": "Iterator",
      "target": "Iter<T>",
      "associated_types": [{ "name": "Item", "ty": "T" }],
      "methods": ["map", "filter"],
      "span": { "start": 80, "end": 118 }
    }
  },
  "qualified_calls": {
    "map_over#12": {
      "kind": "type_method",
      "owner": "Iter<T>",
      "name": "map",
      "impl_id": "Iterator::Iter<T>",
      "span": { "start": 200, "end": 220 }
    },
    "map_over#19": {
      "kind": "unknown",
      "owner": null,
      "name": null,
      "impl_id": null,
      "span": { "start": 240, "end": 252 }
    }
  }
}
```

### フェーズ 1: Frontend の MIR JSON 拡張
- [x] `MirModule` 生成時に `dict_refs` / `impls` / `qualified_calls` を追加出力できるように拡張する。
  - 作業ステップ:
    - `compiler/frontend/src/semantics/mir.rs` の `MirModule` 生成経路を棚卸しし、追加フィールドの生成タイミングを決める。
    - `TypedModule` / `TypecheckReport` から必要情報を収集するための参照経路を整理する。
    - `qualified_calls` の `kind` 判定ロジックと未解決時の値（空/unknown）を決める。
- [x] `reml_frontend` の JSON 出力に追加フィールドが出ることを確認する。
  - 作業ステップ:
    - `compiler/frontend/src/bin/reml_frontend.rs` の JSON 出力経路を確認し、追加フィールドの serialize が有効か確認する。
    - 代表的なサンプル（trait/impl/qualified 名を含む）で JSON 出力差分を観察する。
    - 出力例を docs-examples-audit のメモへ記録する。
- [x] 追加フィールドが空でも JSON 形状が安定するよう default を設ける。
  - 作業ステップ:
    - 空配列/空 map のどちらを採用するかを決め、出力時に常に含める方針を確定する。
    - `serde(default)` などの利用方針を整理し、空値でも互換性が保てるようにする。

#### フェーズ 1 決定メモ（Frontend MIR JSON 拡張）
- **設計チェック（完了）**:
  - [x] `MirModule` の拡張位置と `schema_version` 維持方針を確定。
  - [x] `dict_refs` の出力元と空配列の扱いを確定。
  - [x] `impls` / `qualified_calls` の空出力方針を確定。
  - [x] `reml_frontend` の JSON 出力経路で追加フィールドが出力対象になることを確認。

- **MirModule の拡張位置**: `compiler/frontend/src/semantics/mir.rs` の `MirModule` に `dict_refs` / `impls` / `qualified_calls` を追加する。
- **schema_version**: 現状の `MIR_SCHEMA_VERSION = "frontend-mir/0.2"` を維持し、追加フィールドは optional + default で段階導入する。

##### 1) `dict_refs` の出力元
- **取得元**: `typed::TypedModule.dict_refs`（`TypecheckDriver` が `DictRefDraft` から生成済み）。
- **変換**: `MirModule::from_typed_module` で `TypedModule.dict_refs` をそのままコピー（`MirModule.dict_refs` と同形）。
- **空時の扱い**: `[]` を常に出力（`serde(default)` + `skip_serializing_if` は使わない）。

##### 2) `impls` の出力元
- **現状**: Frontend 側で trait/impl 宣言の一覧（`impl_id` → trait/target/associated_types/methods）を保持していない。
- **方針**: フェーズ1では `impls = {}` を出力し、フェーズ2以降の Typeck/Resolver 拡張で埋める前提とする。
- **TODO（実装分解）**:
  - [x] `parser::ast::ImplDecl` から `impl_id` と trait/target を抽出するテーブルを typeck 内に追加。
  - [x] `TypecheckReport.mir.impls` へ impl_registry を載せて JSON 直列化する。
  - [x] `impl_registry` から `associated_types` / `methods` を埋める基準を定義する。
  - [ ] 既存 `used_impls` との突合ルール（未登録 impl の扱い）を決める。
  - **埋め込みルール案**:
    - `impl_id`: trait impl は `{TraitName}::{TargetType}`、inherent impl は `{TargetType}`（`TypeAnnot.render()` を使用）。
    - `trait`: `impl <TraitRef> for <Target>` の場合は `TraitRef.name.name` を採用し、引数は `impl_id` の `TargetType` 側に残す。
    - `target`: `impl` の `target.render()` をそのまま利用。
    - `associated_types`: `ImplItem::Decl` の `DeclKind::Type` を対象にし、`type alias`/`newtype` のみ採用（`sum`/`opaque`/未定義は未解決扱い）。
    - `methods`: `ImplItem::Function` と `ImplItem::Decl` の `DeclKind::Fn` を収集（`signature.name.name` を採用）。
    - 重複 `impl_id`: 先勝ち（最初に出現したものを採用）とし、重複は `impl_registry.duplicate` TODO を残す。
    - 未解決扱い: `target` が空文字の場合は `impl_registry.unresolved` とし、`impl_id` を `"<unknown>"` にフォールバックする。

##### 3) `qualified_calls` の出力元
- **現状**: `TypedExprKind::Call` は修飾子（`Type.method` / `Type::method` / `Trait::method`）の判定情報を保持していない。
- **方針**: フェーズ1では `qualified_calls = {}` を出力し、識別が可能になった時点で `kind` / `owner` / `impl_id` を埋める。
- **キー規則**: `"<fn_name>#<expr_id>"` を前提とし、`expr_id` は `MirExpr.id` をそのまま使用する。
- **TODO（実装分解）**:
  - [ ] name resolution に `QualifiedName` テーブルを追加し、`Call` ノードの解決結果を保持する。
  - [ ] `MirExprBuilder` が `QualifiedName` を参照できるよう、式 ID の対応表を用意する。
  - [ ] `TypecheckReport` に `qualified_call_table` を追加し、MIR JSON 生成時に転写する。
  - [ ] 未解決時に `kind = "unknown"` を設定する経路を定義する。
  - **埋め込みルール案**:
    - `Type.method`（フィールドアクセス経由の呼び出し）: `Type.method(x)` 形式と判定できる場合は `type_method`。`owner` は `TypePath` を `::` で連結、`name` はメソッド名。
    - `Type::method`: `ModulePath` の最終セグメントを `name`、それ以外を `owner` にして `type_assoc` とする（`owner` が型っぽい識別子のみで構成される場合）。
    - `Trait::method`: `ModulePath` の `owner` の末尾セグメントが `DeclKind::Trait` で定義された trait 名に一致する場合は `trait_method`。`foo::Bar::baz` のようなモジュールパスでも `Bar` を trait 名として判定する。
    - その他: `unknown` とし、`owner`/`name` は可能なら埋めるが `impl_id` は未解決として `null` を許容。

##### `impl_id` 命名規則と `qualified_calls` 対応表（暫定）
- `impl_id` の基本規則は既定どおり `trait` 実装は `{TraitName}::{TargetType}`、inherent impl は `{TargetType}`。
- `qualified_calls.impl_id` は暫定で以下の対応とする（解決情報が揃った時点で更新）:
  - `type_method` / `type_assoc`: `owner` をそのまま `impl_id` に採用（`{TargetType}` 想定）。
  - `trait_method`: `impls` テーブルとレシーバ型推論結果で照合できる場合のみ `impl_id` を付与し、未解決は `null`。
  - `unknown`: `impl_id` は `null` を維持。

##### `impl_id` / `receiver_ty` の正規化方針（Int/i64）
- `impl_id` と `impls.target` は `TypeAnnot.render()` の表記を維持する（例: `Int`）。
- `qualified_calls.receiver_ty` は `normalize_mir_type_label` により `i64` へ正規化する。
- 照合は `normalize_impl_target_for_match` で `Int`/`Unit` のみ正規化して一致させる。
- `impl_id` の命名自体は正規化しない（既存ログ・仕様との整合を優先）。

##### `trait_method` の `impl_id` 解決ルール（設計案）
**解決場所の方針**: `typeck` 側で解決候補を生成し、`qualified_calls` に追加情報を載せて Backend が確定/診断する二段構えを採用する。

**理由**:
- レシーバ型や `Self` 展開は typeck が最も正確に把握できる。
- Backend は MIR JSON のみを入力にする前提のため、判定に必要な情報を JSON に載せる必要がある。

**追加する情報（MIR JSON 拡張案）**:
- `qualified_calls[].receiver_ty: string | null`
  - `Type.method` / `Trait::method` の呼び出しで、レシーバ型が推論できた場合に記録する。
- `qualified_calls[].impl_candidates: [string, ...]`
  - `impls` テーブルから `trait` + `receiver_ty` で絞り込んだ `impl_id` 候補を列挙する。

**解決フロー**:
1) typeck で `qualified_calls` を作成する際、`receiver_ty` を記録。
2) typeck で `impls` テーブルを参照し、`impl_candidates` を作成。
3) Backend は `impl_candidates` が単一なら `impl_id` を確定し、0 件/複数なら TODO 診断へ回す。

**フォールバック**:
- `receiver_ty` 不明 or `impl_candidates` 未記録の場合は `impl_id = null` として扱う。
- `impls` 未出力のフェーズでは `impl_candidates` を空配列にして TODO を維持。

##### フェーズ 1 実施メモ（qualified_calls 拡張）
- `qualified_calls` に `receiver_ty` / `impl_candidates` を追加し、`MirExprBuilder` で `receiver_ty` を埋める。
- `Trait::method` を `trait_method` として識別し、trait 名は `DeclKind::Trait` 定義と照合する。
- `impls` が空のフェーズでは `impl_candidates` を空配列にし、候補が単一なら `impl_id` を確定する。
- pipe (`|>`) 展開後の `Call` でも `qualified_calls` が付与されるよう、typeck の desugar 経路で解決を実行する。
- `impl_candidates` 照合時のみ `Int` / `Unit` を `i64` / `unit` に正規化する（`impl_id` 命名は維持）。

##### フェーズ 1 実施メモ（default 安定化）
- `MirModule` は `dict_refs` / `impls` / `qualified_calls` を必ず保持し、空の場合でも `[]` / `{}` を出力する。
- `skip_serializing_if` を付与していないため、空値でも JSON に常に含まれることを確認。

##### フェーズ 1 実装メモ（impl_registry 抽出）
- 実装箇所: `compiler/frontend/src/typeck/driver.rs` の `collect_impl_specs`。
- `impl_id` は `trait_name + "::" + target.render()` を採用し、未解決 target は `"<unknown>"` を付与。
- `associated_types` は `DeclKind::Type` の alias/newtype のみ抽出する。
- `methods` は `ImplItem::Function` と `ImplItem::Decl(DeclKind::Fn)` を収集する。
- 重複 `impl_id` は `impl_registry_duplicates` に記録する。

##### 実出力検証メモ（MIR JSON）
- 検証サンプル: `examples/docs-examples/spec/3-1-core-prelude-iteration/sec_4_2.reml`
  - `qualified_calls` に `Histogram::new` / `HistogramError::OutOfRange` が入り、`impl_id` は `Histogram` / `HistogramError` になっていることを確認。
- 検証サンプル: `examples/docs-examples/spec/3-1-core-prelude-iteration/sec_3_5.reml`
  - pipe 展開後に `Iter.from_list` / `Iter.map` / `Iter.try_fold` / `Diagnostic::invalid_value` が `qualified_calls` に記録され、`impl_id` は `Iter` / `Diagnostic` を確認。
- 検証サンプル: `tmp/trait_method_sample.reml`
  - `Trait::method` が `trait_method` として記録され、`impl_candidates = ["Show::Int"]` になり、候補が単一のため `impl_id = "Show::Int"` になることを確認。

##### 4) `reml_frontend` の JSON 出力経路
- **対象**: `compiler/frontend/src/bin/reml_frontend.rs` の `TypeckArtifacts` / `TypeckDebugFile` で `mir` を JSON 出力している。
- **方針**: `mir::MirModule` に新フィールドを追加すれば CLI 側は自動的に出力対象になる（追加の serialize 経路は不要）。

##### 出力例（空の場合）
```json
{
  "schema_version": "frontend-mir/0.2",
  "functions": [],
  "active_patterns": [],
  "conductors": [],
  "dict_refs": [],
  "impls": {},
  "qualified_calls": {}
}
```

### フェーズ 2: Backend の MIR JSON 取り込み
- [x] `integration.rs` に `dict_refs` / `impls` / `qualified_calls` を取り込む型定義を追加する。
  - 作業ステップ:
    - Frontend 側 JSON 形状に合わせた struct を定義し、serde の型一致を確認する。
    - `impls` の内部構造（trait 名、target、associated_types、methods）の必須/任意項目を決める。
    - `qualified_calls` の `kind` を enum 化し、未知値の fallback 方針を決める。
- [x] `qualified_calls` が未解決の場合は TODO 診断を出す（IR 生成は継続）。
  - 作業ステップ:
    - 未解決判定の条件（`kind` 欠落/unknown、owner 不明など）を定義する。
    - TODO 診断のメッセージと識別子（既存の診断規約に合わせる）を決める。
    - 生成継続のための fallback（既存のシンボル解決ルールへの委譲）を明示する。
- [x] `qualified_call_table` を参照した TODO 診断の規約を確定する。
  - 作業ステップ:
    - 診断キー案を `backend.todo.qualified_call_unresolved` とし、`expr_id` / `owner` / `name` / `kind` を `metadata` に記録する。
    - `kind = "unknown"` の場合は診断レベル `TODO` で通過し、`owner` が trait 名と一致するが `impl_id` が欠落している場合は `TODO: trait impl unresolved` に細分化する。
    - `qualified_calls` に該当キーが存在しない場合は「未解決・未記録」として `backend.todo.qualified_call_missing` を出す方針にする。
- [x] `impls` に associated type が欠落している場合も TODO 診断を出す。
  - 作業ステップ:
    - `associated_types` が空/欠落のときの取り扱い条件を定義する。
    - `Self::X` 展開が未完了である旨を診断に含めるか検討する。
- [x] Backend 診断フォーマットの整合方針を確定する。
  - 作業ステップ:
    - 既存 `BackendDiffSnapshot.diagnostics` が文字列配列である点を前提とし、`domain.code: message` 形式を維持する。
    - `Diagnostic` 構造体への移行は `verify`/`integration` の両方に波及するため別タスクとして扱う。
- [x] 既存の JSON 入力が追加フィールドなしでも動作することを確認する。
  - 作業ステップ:
    - 追加フィールドが空/欠落の入力を流すテストケースを想定し、確認手順を記録する。
    - `Option`/default が機能しない場合の修正方針（serde 属性追加）を整理する。

### フェーズ 3: Runtime async 型の土台追加
- [x] `compiler/runtime/Cargo.toml` に `core_async` / `core-async` feature を追加する。
  - 作業ステップ:
    - 既存 feature 名との衝突有無を確認し、命名規約に合わせた定義を追加する。
    - `core_async` と `core-async` の alias 方針（features 別名）を決める。
- [x] `compiler/runtime/src/prelude/async.rs` を追加し、`Future<T>` / `AsyncStream<T>` を opaque 型で定義する。
  - 作業ステップ:
    - `PhantomData` を用いた opaque 型の最小定義を作成し、公開 API を最小限に留める。
    - 仕様書の型名と一致することを確認し、将来の拡張余地を注記する。
- [x] `compiler/runtime/src/prelude/mod.rs` に `#[cfg(feature = "core_async")] pub mod r#async;` を追加する。
  - 作業ステップ:
    - 既存の prelude モジュール構成を確認し、露出順と命名衝突がないことを確認する。
- [x] `core_async` 無効時に既存ビルドが壊れないことを確認する。
  - 作業ステップ:
    - feature 未指定時のコンパイルに影響しないよう `cfg` の適用範囲を確認する。
    - 既存 tests/build コマンドの影響範囲を整理する。

### フェーズ 4: docs-examples-audit 連携
- [x] async 連携サンプル（`docs/spec/3-1-core-prelude-iteration.md` の `from_async_stream` / `to_async_stream`）に対し、Runtime 未実装の TODO を `1-9` へ記録する。
  - 作業ステップ:
    - 仕様内の対象サンプルを特定し、未実装事項の粒度（型未定義 / 実装未提供）を整理する。
    - TODO 記載のフォーマット（見出し/チェック項目/リンク）を決める。
- [x] trait/impl/qualified 名サンプルを Backend 未対応チェック対象として整理する。
  - 作業ステップ:
    - 仕様の該当サンプルを列挙し、Backend 側で未解決になる理由をメモ化する。
    - 検証対象の一覧に落とし込み、後続タスクへ引き継げる形に整形する。

#### フェーズ 4 実施メモ（docs-examples-audit）

##### Async 連携サンプル TODO
- 対象サンプル: `examples/docs-examples/spec/3-1-core-prelude-iteration/sec_6_2.reml`
  - 仕様参照: `docs/spec/3-1-core-prelude-iteration.md`
  - 未実装事項: Runtime 側の `AsyncStream<T>` / `Future<T>` は opaque 型のみで、`from_async_stream` / `to_async_stream` の API 実装と bridge が未提供。
  - TODO 方針: `docs-examples-audit` では `core_async` feature 依存の未実装として記録し、Runtime 実装完了まで検証対象から除外する。

##### Backend 未対応チェック対象（trait/impl/qualified 名）
- `examples/docs-examples/spec/3-1-core-prelude-iteration/sec_3_4.reml`
  - `trait Collector<T, C>` + `Self::Error` の associated type があり、`impls`/`associated_types` 未出力のため Backend で未解決 TODO になる前提。
- `examples/docs-examples/spec/3-1-core-prelude-iteration/sec_4_2.reml`
  - `impl Collector<...> for HistogramCollector` と `Self::Error` が未解決前提。
  - `Map::empty` / `HistogramError::OutOfRange` / `Histogram::new` は `qualified_calls` が空のため Backend 側で未解決 TODO を想定。
- `examples/docs-examples/spec/3-1-core-prelude-iteration/sec_3_5.reml`
  - `Iter.from_list` / `Iter.map` / `Diagnostic::invalid_value` が `qualified_calls` 未解決の対象。
- `examples/docs-examples/spec/3-1-core-prelude-iteration/sec_3_7.reml`
  - `List::empty` が `qualified_calls` 未解決の対象。
- `examples/docs-examples/spec/3-1-core-prelude-iteration/sec_5_2.reml`
  - `Iter.buffered` / `Summary::empty` が `qualified_calls` 未解決の対象。
- docs-examples-audit 検証表の更新:
  - `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` に `sec_3_4` / `sec_3_5` / `sec_3_7` / `sec_4_2` / `sec_5_2` / `sec_6_2` の TODO/除外区分を追記。

## 受け入れ基準
- MIR JSON に `dict_refs` / `impls` / `qualified_calls` が追加され、Backend が読み込み可能。
- Backend は未解決の qualified call / associated type を TODO 扱いで通過できる。
- Runtime に `core_async` feature と opaque 型が追加され、Prelude から参照できる。
- docs-examples-audit で未対応箇所が記録され、後続の実装計画に引き継げる。

## 進捗管理
- 本計画書作成日: 2025-12-27
- 進捗欄（運用用）:
  - [ ] フェーズ 0 完了
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [x] フェーズ 3 完了
  - [x] フェーズ 4 完了
- 進捗メモ:
  - フェーズ 1 は `qualified_calls`/`receiver_ty`/`impl_candidates` の実装と出力確認、`impls` 出力、default 安定化まで完了。
  - `used_impls` との突合ルールは未対応。

## 関連リンク
- `docs/plans/docs-examples-audit/1-8-backend-runtime-trait-async-plan-20251227.md`
- `docs/spec/3-1-core-prelude-iteration.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`
