# 1.4 Backend/Runtime リテラル対応計画（2025-12-26）

フロントエンド AST/MIR には存在するが、Backend でリテラル構造の解釈がなく、Runtime でも実装が未整備なリテラルについて、将来のコード生成に備えた対応計画をまとめる。

## 目的
- MIR/JSON のリテラル表現を安定させ、Backend が構造を解釈できる状態にする。
- Runtime に必要な型タグ・ABI・実体構造を用意し、Backend と整合させる。
- 仕様と実装の差分を段階的に埋めるための優先順位を整理する。

## 対象範囲
- Backend: `compiler/backend/llvm`
- Runtime: `compiler/runtime/native`
- フロントエンド: `compiler/frontend/src/parser/ast.rs` `compiler/frontend/src/semantics/mir.rs`

## 対象リテラル（現状のギャップ）
- Float リテラル
- Char リテラル
- Tuple リテラル
- Array リテラル
- Record リテラル

## 前提・現状
- Backend は `Literal` を `serde_json::Value` のままサマリ化し、`int/string/bool/unit` 以外は「unsupported literal」扱い。
- Runtime は `REML_TAG_TUPLE` / `REML_TAG_RECORD` を持つが、破棄処理は placeholder。`Array`/`Char` は型タグ自体が未定義。

## 実行計画

### フェーズ 0: 仕様・MIR 形状の確認
- 各リテラルの JSON 形状を整理し、MIR/JSON の安定仕様としてメモ化する。
- 仕様側の記述（`docs/spec`）で意味論が確定しているか確認する。
  - [x] `compiler/frontend/src/semantics/mir.rs` の `Literal` 構造と JSON 変換箇所を洗い出す
  - [x] `Literal` ごとの JSON 形状（キー名、配列/オブジェクトの入れ子、型ヒント）を一覧化する
  - [x] 仕様書（`docs/spec`）のリテラル定義箇所を特定し、意味論の不足点をメモする
  - [x] Float/Char/Tuple/Array/Record の未確定事項（精度、表現、型推論ルール）を TODO として整理する
  - [x] まとめを本計画書に追記してレビュー待ちにする

#### フェーズ 0 メモ（2025-12-26）
- MIR の `Literal` は `compiler/frontend/src/semantics/mir.rs` の `MirExprKind::Literal(Literal)` で表現され、`Literal` 自体は `compiler/frontend/src/parser/ast.rs` の `Literal`/`LiteralKind` を `serde` 直列化した JSON がそのまま使われる。
- `Literal` は `{ "value": LiteralKind }` の 1 フィールド構造で、`MirExpr.kind` の `literal` 直下に `value` が入る（二重の `value` に注意）。
- `LiteralKind` は `serde(tag = "kind", rename_all = "snake_case")` の内部タグ形式。

```json
{
  "kind": "literal",
  "value": {
    "value": {
      "kind": "int",
      "value": 1,
      "raw": "1",
      "base": "base10"
    }
  }
}
```

**LiteralKind の JSON 形状（主要リテラル）**
- Int: `{ "kind": "int", "value": i64, "raw": "1_000", "base": "base10|base2|base8|base16" }`
- Float: `{ "kind": "float", "raw": "3.14" }`
- Char: `{ "kind": "char", "value": "A" }`（`String` として保持）
- String: `{ "kind": "string", "value": "...", "string_kind": "normal|raw|multiline" }`
- Bool: `{ "kind": "bool", "value": true }`
- Unit: `{ "kind": "unit" }`
- Tuple: `{ "kind": "tuple", "elements": [Expr, ...] }`
- Array: `{ "kind": "array", "elements": [Expr, ...] }`
- Record: `{ "kind": "record", "type_name": Ident?, "fields": [ { "key": Ident, "value": Expr }, ... ] }`
  - `Ident` は `{ "name": String, "span": Span }` で直列化される（`RecordField.key` も同様）。

**仕様での該当箇所**
- リテラル構文: `docs/spec/1-1-syntax.md`（A.4, E.1）
- 数値/文字リテラルの字句仕様: `docs/spec/2-3-lexer.md`（E, F）
- 文字（Unicode スカラ値）: `docs/spec/1-4-test-unicode-model.md`（C）
- 型・既定型: `docs/spec/1-2-types-Inference.md`（A.1, A.2, C.5/H.4 近辺の数値既定）

**TODO（意味論の未確定事項）**
- Float: `raw` のみ保持しているため、MIR/Backend での即値化タイミングと既定型（`f64` 固定か、`RunConfig` で切替可能か）を明記する必要がある。
- Char: 仕様上は Unicode スカラ値（`Char`）だが、Runtime ABI で `u32`/UTF-8/タグ付きボックスのどれを採るか未確定。
- Tuple: リテラルの ABI（レイアウト/ヒープ化）と、空タプル `()` の Runtime 表現が明文化不足。
- Array: リテラルが `[T; N]`（固定長）なのか、`[T]`（動的配列）へ自動デシュガするのか未確定。
- Record: フィールド順序は不問とされるが、Runtime のレイアウト順序/ハッシュ化戦略が未確定。

### フェーズ 1: Backend リテラル解釈の追加
- `Literal` サマリから Float/Char/Tuple/Array/Record を識別できるようにする。
- `emit_value_expr` と型推論補助の「literal 解析」を拡張する。
- 未対応の型は明示的に診断ログに残す。
  - [x] `compiler/backend/llvm/src/codegen.rs` で現状の `Literal` 解釈パスを把握する
  - [x] JSON 形状に対応した判定分岐と、内部表現（型タグ/初期化式）を設計する
  - [x] Float/Char/Tuple/Array/Record の解釈結果を `emit_value_expr` に接続する
  - [x] 未対応リテラルの診断メッセージを整理し、識別子命名規則に合わせる
  - [x] 既存リテラル（int/string/bool/unit）への影響がないことを確認する

### フェーズ 2: Runtime 型タグと ABI の定義
- `REML_TAG_*` に Char/Array のタグを追加する。
- Tuple/Record/Array の最小構造を C 側で定義する（破棄処理含む）。
- Char の表現（UTF-8 1byte or Unicode scalar）を決める。
  - [x] `compiler/runtime/native/include/reml_runtime.h` の既存タグを確認し、追加タグの値レンジを決める
  - [x] Tuple/Record/Array のレイアウト（ヘッダ、長さ、要素ポインタ）を最小構成で設計する
  - [x] Char の表現を仕様に合わせて決定し、ABI への影響点をメモする
  - [x] ABI レイアウトのコメントをヘッダに追記し、Backend が参照できる形にする
  - [x] 破棄処理のインタフェース（関数名、引数、責務）を定義する

#### フェーズ 2 メモ（2025-12-26）
- `REML_TAG_CHAR = 10` / `REML_TAG_ARRAY = 11` を追加し、既存タグの値を維持。
- Char は Unicode scalar value を `reml_char_t`（`uint32_t`）で表現する方針に確定。
  - ボックス化する場合は `REML_TAG_CHAR` を用い、payload で `reml_char_t` を保持。
- Tuple/Record/Array は最小 ABI として `{len, items}` / `{field_count, values}` を採用。
  - payload は `reml_object_header_t` 直後に配置され、要素配列は `void*` スロットを保持。
  - Record のフィールド順序は Backend で決定（現状はソース順を想定）。
- 破棄処理インタフェース `reml_destroy_tuple/record/array` を Phase 3 実装前提で宣言。

#### Backend ABI 接続メモ（2025-12-26）
- Backend (`compiler/backend/llvm/src/codegen.rs`) は現状 `float/char/tuple/array/record` を未対応として診断コメントを出すのみで、Runtime ABI との接続は未実装。
- Runtime ABI 参照は `@reml_*` intrinsic 名に限定され、`REML_TAG_*` や `reml_tuple_t` 等のレイアウト参照は存在しない。
  - 既存の ABI 依存は `mem_alloc/inc_ref/dec_ref/panic` と `reml_set_new/insert` のみ。
- `type_mapping.rs` の `RowTuple` はサイズ概算のみで、Runtime Tuple/Record/Array と直接対応しない。
- 接続方針:
  - Backend からは **Runtime の公開 API でヒープオブジェクトを構築**する前提とし、
    直接レイアウトを書き込むのは Phase 3 以降に限定する。
  - Tuple/Record/Array/Char の構築用に `@reml_*` intrinsic を追加し、
    `reml_runtime.h` の ABI と一致する C 実装を用意する。
  - 直接構築を行う場合は `mem_alloc` + `reml_set_type_tag` を必須とし、
    items 配列の確保/解放責務を Runtime 側で統一する。

#### Runtime API 候補（@reml_*）メモ（2025-12-26）
- 目的: Backend が ABI レイアウトを直接書かずに、リテラル構築の責務を Runtime に委譲する。
- 署名候補（LLVM IR での呼び出しを想定）:
  - `@reml_char_new(i32) -> ptr`
    - `reml_char_t` をボックス化して `REML_TAG_CHAR` を設定したヒープポインタを返す。
  - `@reml_tuple_new(i64 len, ptr items) -> ptr`
    - `items` は `void**` 相当（要素のヒープポインタ配列）。
  - `@reml_record_new(i64 field_count, ptr values) -> ptr`
    - `values` は `void**` 相当（フィールド値配列）。順序は Backend 側で決定。
  - `@reml_array_new(i64 len, ptr items) -> ptr`
    - `items` は `void**` 相当。
  - `@reml_array_from_span(i64 len, ptr items, i1 take_ownership) -> ptr`（将来候補）
    - 既存バッファの所有権を Runtime 側に移す場合に利用。
- 破棄 API は `dec_ref` に統合し、明示的な `@reml_*_destroy` を Backend からは呼ばない。
- 文字列同様に、非ポインタ要素は Backend がボックス化して `items` に格納する方針。

#### Backend emit_value_expr 構築パスの設計メモ（2025-12-26）
- `LiteralSummary::Char/Tuple/Array/Record` に対して以下の構築パスを追加する想定。
- Char:
  - `reml_char_t` 相当の `i32` を生成 → `@reml_char_new` 呼び出し → `ptr` を返す。
- Tuple/Array:
  - 各要素を `emit_value_expr` で評価し、`ptr` へ正規化（非 ptr は boxing）。
  - `mem_alloc` で `void**` 配列を確保して格納 → `@reml_tuple_new` / `@reml_array_new` を呼ぶ。
  - `items` の所有権は Runtime 側が保持し、Backend は解放しない。
- Record:
  - フィールド順序は MIR の `fields` 順で固定し、`values` 配列へ格納。
  - `@reml_record_new` を呼ぶ。
- Boxing 方針（候補）:
  - 既存の `INTRINSIC_VALUE_I64/BOOL/STR` 相当の補助関数に合わせ、
    `@reml_box_i64` / `@reml_box_bool` / `@reml_box_f64` 等の追加を検討。
  - 追加が間に合わない場合は `mem_alloc` + `reml_set_type_tag` を Backend から行う。

### フェーズ 3: Runtime 実装（最小機能）
- Tuple/Record/Array の破棄処理を最低限実装する。
- Float/Char のボックス化/アンボックス化の補助関数を追加する。
- 参照カウント管理の適用範囲を明文化する。
  - [x] `compiler/runtime/native/src/refcount.c` の既存実装を確認し、タグ別の分岐を追加する
  - [x] Tuple/Record/Array の要素走査と参照カウント減算を実装する
  - [x] Float/Char 用の boxing/unboxing API を追加し、ヘッダに宣言する
  - [x] 参照カウント対象（ヒープ/即値）の区分を文書化する
  - [x] Backend が呼び出す補助関数の利用例をメモに残す

#### フェーズ 3 メモ（2025-12-26）
- `compiler/runtime/native/src/refcount.c` で `REML_TAG_ARRAY` を追加し、Tuple/Record/Array の破棄 API を実装した。
  - `reml_destroy_tuple/record/array` が `items/values` を走査して `dec_ref` し、配列バッファを `free` で解放する。
  - `REML_TAG_CHAR` はプリミティブ扱いでデストラクタ不要とする。
- `compiler/runtime/native/src/boxing.c` を追加し、`reml_box_float` / `reml_unbox_float` / `reml_box_char` / `reml_unbox_char` を実装した。
  - Char は Unicode scalar value として妥当性チェックを行い、不正値は `panic` で停止する。
- 参照カウント対象の区分を `compiler/runtime/native/include/reml_runtime.h` に追記した。
- Backend での呼び出し例（想定）:
  - Float: `@reml_box_float(f64)` → `ptr` をリテラル値として保持
  - Char: `@reml_box_char(i32)` → `ptr` をリテラル値として保持
  - Tuple/Record/Array: `@reml_destroy_*` は `dec_ref` の破棄パスから呼ばれる（Backend は直接呼ばない）。

### フェーズ 4: テストと結合検証
- Backend スナップショットに各リテラルの例を追加する。
- Runtime のユニットテスト（破棄/参照カウント）を追加する。
- Frontend → Backend → Runtime の最小経路を確認する。
  - [x] Backend のスナップショットテストに Float/Char/Tuple/Array/Record の入力例を追加する
  - [x] Literal の JSON 形状が想定どおりに解釈されることを検証する
  - [x] Runtime の破棄処理で参照カウントが正しく減少するテストを追加する
  - [x] Char/Float の boxing/unboxing が往復で一致するテストを追加する
  - [x] 最小経路の手順（対象ソース、コマンド、期待出力）を計画書に記載する

#### 最小経路メモ（2025-12-26）
- 対象ソース: Float/Char/Tuple/Array/Record を含む `.reml`
- Frontend: `cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json <source>.reml > tmp/mir-literals-backend.json`
  - 期待: JSON に `kind: "float"|"char"|"tuple"|"array"|"record"` が出力される
- Backend: `cargo test --manifest-path compiler/backend/llvm/Cargo.toml llvm_ir_literal_snapshots_cover_complex_literals`
  - 期待: `@reml_box_float`/`@reml_box_char` と `diag backend.literal.unsupported.*` が IR に含まれる
- Runtime: `make test -C compiler/runtime/native`
  - 期待: `test_refcount` と `test_boxing` が完走する

### フェーズ 5: ドキュメント更新
- 実装済みのリテラル表現と ABI を仕様に反映する。
- 未対応項目は `@unstable` 等で明示する。
  - [x] `docs/spec` の該当章に JSON 形状と ABI を追記する
  - [x] Runtime 側のタグ/構造体定義を仕様に反映する
  - [x] 未対応・実験中の項目を `@unstable` で明示する
  - [x] 仕様と実装の差分が残る場合は後続タスクを記録する

#### フェーズ 5 メモ（2025-12-26）
- `docs/spec/1-1-syntax.md` に MIR/JSON 形状と Runtime ABI を追記し、未確定項目は `@unstable` で明示した。
- 後続タスク:
  - `literal_array_semantics`: `[T; N]` / `[T]` の確定と MIR からの降ろし方の確定
  - `literal_record_layout`: Record のフィールド順序規則（ソース順固定か別の正規化か）
  - `literal_float_parse`: `raw` 文字列の即値化タイミングと既定型ルールの明文化

## 進捗管理
- 本計画書作成日: 2025-12-26
- 進捗欄（運用用）:
  - [x] フェーズ 0 完了
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [x] フェーズ 3 完了
  - [x] フェーズ 4 完了
  - [x] フェーズ 5 完了

## 関連リンク
- `compiler/frontend/src/parser/ast.rs`
- `compiler/frontend/src/semantics/mir.rs`
- `compiler/backend/llvm/src/codegen.rs`
- `compiler/runtime/native/include/reml_runtime.h`
- `compiler/runtime/native/src/refcount.c`
