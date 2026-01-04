# 1.5 Backend/Runtime Array リテラル意味論確定計画（2025-12-26）

Array リテラル `[ ... ]` が `([T; N])` / `[T]` のどちらへ降りるか、および Runtime `reml_array_t` の意味論を確定し、仕様と実装の不整合を解消するための計画書。

## 目的
- Array リテラルの型推論規則と実行時表現を明確化する。
- Backend/Runtime の ABI と仕様の整合をとる。
- 未確定事項（`literal_array_semantics`）を仕様化し、実装へ反映する。

## 対象範囲
- 仕様: `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`
- Backend: `compiler/backend/llvm`
- Runtime: `compiler/runtime/native`

## 前提・現状
- AST/MIR の `LiteralKind::Array` は `elements` を保持するが、固定長/動的の区別は持たない。
- Runtime の `reml_array_t` は動的配列相当の最小 ABI として導入済み。
- 仕様上は `[T; N]` と `[T]` が型として存在するが、リテラルの降ろし方が未確定。

## 実行計画

### フェーズ 0: 仕様・実装の現状確認
- Array リテラルの仕様記述と型推論の記載を整理する。
- Frontend の AST/MIR と Backend のリテラル解釈を確認する。
  - [x] `docs/spec/1-1-syntax.md` / `docs/spec/1-2-types-Inference.md` の Array 記述を確認
  - [x] 仕様の用語（固定長配列/動的配列、リテラル、型注釈）の定義揺れを洗い出す
  - [x] 仕様中の未確定事項・注記（`@unstable`/TODO/留保）を抜き出して一覧化する
  - [x] `compiler/frontend` の `LiteralKind::Array` 形状を確認
  - [x] Array リテラルが MIR/型推論へ渡る時点の情報（要素型・長さ・注釈）を整理する
  - [x] Backend/Runtime の `reml_array_t` / `LiteralSummary::Array` の実装状況を確認
  - [x] Backend で配列がどの型として扱われているか（固定長/動的）を追跡する
  - [x] Runtime ABI（要素ポインタ、長さ、容量、アラインメント等）の現状仕様を整理する

#### フェーズ 0 調査結果メモ

**仕様**
- `docs/spec/1-1-syntax.md` は Array リテラルの構文と ABI（`reml_array_t`）を記載し、`@unstable("literal_array_semantics")` で `[T; N]` / `[T]` どちらに降ろすか未確定と明記。
- `docs/spec/1-2-types-Inference.md` は型体系として `[T; N]`（固定長）/ `[T]`（スライス・動的、{ptr,len}）を定義するが、Array リテラルの推論規則は未記載。
- 用語の揺れ: 1-1 は「配列」一般で ABI を `reml_array_t`（動的配列相当）として説明、1-2 は固定長と動的を明示。リテラルの既定型や型注釈優先度は未定義。

**Frontend**
- `compiler/frontend/src/parser/ast.rs` の `LiteralKind::Array` は `elements: Vec<Expr>` のみで長さや注釈は保持しない。
- `compiler/frontend/src/semantics/typed.rs` / `mir.rs` は `Literal` をそのまま保持し、Array 固定長/動的の区別情報は追加されない（型は `TypedExpr.ty` にのみ存在）。
- `compiler/frontend/src/typeck/driver.rs` の Array リテラル推論は `Type::slice(element_ty)` を返し、要素長さは推論に使われない。

**Backend**
- `compiler/backend/llvm/src/codegen.rs` の `LiteralSummary::Array` は `emit_unsupported_literal_value` にフォールバックしており、Array リテラルの実コード生成は未実装。
- `compiler/backend/llvm/src/integration.rs` の `parse_reml_type` は `[T]` を `RemlType::Slice` に解釈するが、`[T; N]` はサポートされない。
- `compiler/backend/llvm/src/type_mapping.rs` の `RemlType::Slice` は `{ptr, i64}` レイアウト。固定長配列の型表現は未定義。

**Runtime**
- `compiler/runtime/native/include/reml_runtime.h` の `reml_array_t` は `len` と `items`（`void**`）のみで、容量・アラインメント・要素型メタ情報は持たない。
- `compiler/runtime/native/src/refcount.c` には `reml_destroy_array` があり、要素の `dec_ref` と `items` 解放を行うが、生成 API は現時点で未定義。

### フェーズ 1: 意味論・型推論ルールの確定
- `[T; N]` / `[T]` の既定を決定する（例: 既定は `[T; N]`、明示型注釈がない場合は動的配列へ昇格など）。
- `N` の算出規則（要素数、空配列、ネスト）と、型注釈との整合を整理する。
- 既定ルールに反するケースの診断方針を定義する。
  - [x] Array リテラルの型推論規則（既定・例外・エラー条件）を確定
  - [x] 空配列 `[]` の型推論（既定型・注釈必須条件・補完戦略）を決める
  - [x] 異種要素を含むリテラルの許否と型統一規則（上限型/エラー）を定義する
  - [x] 型注釈がある場合の優先順位（注釈優先/推論優先）を明文化する
  - [x] `[T; N]` と `[T]` の相互変換可否（暗黙変換の有無、明示変換のみ）を決める
  - [x] 仕様に明記する診断名とメッセージの方向性を決める
  - [x] 代表的な診断シナリオ（要素数不一致、注釈違反、空配列注釈不足）を列挙する

#### フェーズ 1 決定事項（意味論・型推論）

**既定ルール**
- Array リテラルは **既定で `[T]`（動的配列）** として型付けする。
- **期待型（コンテキスト）または明示注釈が `[T; N]` の場合のみ** 固定長配列として型付けする。
- `[T; N]` / `[T]` は **別型** とし、**暗黙変換は行わない**。明示変換（`as` や標準 API）は将来仕様で定義する。

**`N` の算出規則**
- `N` は **リテラル内の要素数**で決定する（末尾カンマは無視）。
- ネスト配列は **各リテラルごとに独立して `N` を算出**し、フラット化は行わない。
- 空配列 `[]` は要素数 `0` とみなす。

**型推論の流れ（優先順位）**
- **明示注釈 > 期待型 > 既定推論** の順で適用する。
- 明示注釈がある場合は **注釈を制約として型付け**し、期待型と矛盾する場合はエラーとする。

**空配列 `[]`**
- 期待型または注釈が **`[T]` もしくは `[T; 0]` の場合のみ許可**する。
- 期待型が不明な場合は **注釈必須**とし、未指定なら診断を出す。

**異種要素の扱い**
- すべての要素は **単一の型に単一化**される必要がある。
- サブタイピングや上限型は採用しないため、単一化に失敗する組み合わせはエラーとする。
- 数値リテラルは既存の多相リテラル規則（`Num<T>`）に従って単一化し、曖昧なら既定型へ落とす。

**診断名とメッセージ方針（案）**
- `type.array.literal.empty_requires_annotation`: 空配列に型注釈または期待型が必要であることを明示。
- `type.array.literal.length_mismatch`: 期待する長さ `N` と実際の要素数の不一致を指摘。
- `type.array.literal.element_mismatch`: 要素型が単一化できないことを提示（期待型/実際型を列挙）。
- `type.array.literal.annotation_conflict`: 注釈と期待型が矛盾していることを提示。

**代表的な診断シナリオ**
- `[T; N]` 注釈のリテラルで要素数が一致しない（例: `let xs: [i64; 2] = [1, 2, 3]`）。
- `[]` を注釈・期待型なしで使用（例: `let xs = []`）。
- 要素型が統一できない（例: `let xs = [1, "a"]`）。
- 注釈と期待型が衝突（例: `let xs: [i64] = ([1, 2] : [i64; 2])`）。

### フェーズ 2: 仕様反映
- `docs/spec/1-1-syntax.md` に Array リテラルの意味論と補足を追記する。
- `docs/spec/1-2-types-Inference.md` に型推論ルールを追記する。
  - [x] 仕様更新（構文/型推論）
  - [x] Array リテラルの段落に推論例（固定長/動的/空配列）を追加する
  - [x] 型推論節に診断条件と回避策（注釈の付け方）を追記する
  - [x] 未確定事項の `@unstable` を撤去

### フェーズ 3: Backend/Runtime 反映
- 既定ルールに合わせて Backend のリテラル解釈を調整する。
- Runtime の `reml_array_t` が意味論に一致することを確認する（必要なら ABI の見直しを提案）。
  - [x] Backend のリテラル解釈を更新
  - [x] 固定長配列/動的配列で生成される IR の差分を整理する
  - [x] リテラル生成時のアロケーション戦略（stack/heap）を確認する
  - [x] Runtime ABI の適合確認
  - [x] `reml_array_t` のフィールド定義と生成コードの整合性をチェックする
  - [x] ABI 変更が必要なら互換性影響と移行方針をまとめる

#### フェーズ 3 実施メモ
- Backend は `reml_array_from` を呼び出し、Array リテラルを `reml_array_t` に降ろすよう更新。
- 固定長配列注釈は IR コメントで明示し、現時点では動的配列と同じ生成経路を採用（差分はコメントで追跡）。
- Runtime は `reml_array_from` でヒープ確保（`mem_alloc` + `calloc`）を行い、要素は `inc_ref` で保持。
- ABI の基本構造（`len` + `items`）は変更不要。新規 API 追加のみで互換性影響は限定的。

### フェーズ 4: テストと検証
- 型推論（固定長/動的）のテストケースを追加する。
- Backend IR のスナップショットで `reml_array_*` 呼び出しを検証する。
  - [x] テスト追加
  - [x] 代表ケース（空/単一/複数/ネスト）を網羅するテストを用意する
  - [x] スナップショット確認
  - [x] 期待する IR 形状（配列生成、長さ、要素格納）を明文化する

## 進捗管理
- 本計画書作成日: 2025-12-26
- 進捗欄（運用用）:
  - [ ] フェーズ 0 完了
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [x] フェーズ 3 完了
  - [x] フェーズ 4 完了

## 関連リンク
- `docs/spec/1-1-syntax.md`
- `docs/spec/1-2-types-Inference.md`
- `compiler/frontend/src/parser/ast.rs`
- `compiler/frontend/src/semantics/mir.rs`
- `compiler/backend/llvm/src/codegen.rs`
- `compiler/runtime/native/include/reml_runtime.h`
