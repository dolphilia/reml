# 1.3 Backend/Runtime Set 対応計画（2025-12-26）

`docs/spec/2-3-lexer.md` で復元した集合リテラル `{...}` と `Set<T>` を、将来のコード生成で扱えるように Backend/Runtime 側の対応方針を整理する計画書。

## 目的
- フロントエンドが出力する MIR/JSON から `Set<T>` を安全にコード生成できる状態を作る。
- `Set<T>` の実行時表現と ABI を Runtime に定義し、Backend と一致させる。
- 仕様・実装・テストの整合を保ち、将来の最適化に備える。

## 対象範囲
- Backend: `compiler/backend/llvm`
- Runtime: `compiler/runtime/native`
- 仕様: `docs/spec/2-3-lexer.md` と関連する stdlib 仕様（必要に応じて）

## 前提・現状
- フロントエンドの AST/MIR には `LiteralKind::Set` が存在し JSON に出力される。
- Backend は `Literal` を `serde_json::Value` のままサマリ化しており、集合リテラルの構造を解釈していない。
- Runtime には Set の実装/API が未整備。

## 対応方針の論点（先に決めること）
- **実行時表現**: `Set<T>` を「ランタイムオブジェクト（不透明ポインタ）」で持つか、構造体で持つか。
- **構築コスト**: `{...}` の要素をどのタイミングで構築するか（即時構築 vs 遅延構築）。
- **API 形状**: `set_new`, `set_insert`, `set_from_array` 等の最低限 ABI。
- **型制約**: `Set<T>` の `T` に求める制約（ハッシュ/比較、または参照同一性）。

## 実行計画

### フェーズ 0: 仕様・既存実装の確認
- `docs/spec/2-3-lexer.md` の集合リテラル記述を再確認し、`Set<T>` の意味論をメモする。
- stdlib 仕様（`docs/spec/3-x`）で集合型の想定があるか確認する。
- Runtime/Backend の既存コレクション実装（配列/スライス）との整合点を洗い出す。
  - [x] 仕様メモの作成
  - [x] 既存コレクション実装の調査

### フェーズ 0 実施メモ

#### 仕様メモ
- `docs/spec/2-3-lexer.md` では `reserved(profile, set: Set<Str>)` と `reservedSet = {"fn", ...}` が登場し、`Set<Str>` と `{...}` リテラル、および `contains` の利用例が示されている（字句レイヤのレシピ内で使用）。
- `docs/spec/3-2-core-collections.md` で `Set<T> = PersistentSet<T>` が定義され、`empty_set`/`contains`/`insert`/`diff`/`partition` を提供する。`Set` は `Map<T, Unit>` のラッパで `Ord` 制約を前提に順序付き反復を共有する。
- `docs/spec/3-1-core-prelude-iteration.md` に `collect_set<T: Ord>(iter: Iter<T>) -> Result<Set<T>, CollectError>` が定義され、重複キーは `CollectError::DuplicateKey` で扱う方針が明記されている。

#### 既存コレクション実装の調査
- Runtime (`compiler/runtime/native/include/reml_runtime.h`) には `reml_list_node_t`（暫定 List）と `reml_index_access`（List/Str 対応）があり、`Set`/`Slice`/`Array` 相当の ABI は未定義。
- Runtime の `reml_type_tag_t` は基本型のみで、`Set` 系のタグは未追加。
- Backend (`compiler/backend/llvm/src/type_mapping.rs`) は `RemlType::Slice` を `{ptr, i64}` として扱うが、`Set` の型マッピングは未定義。
- Backend (`compiler/backend/llvm/src/codegen.rs`) のリテラル解釈は `bool/int/string` のみで、集合リテラルの構造は未対応（それ以外は "unsupported literal" 扱い）。

### フェーズ 1: MIR/JSON 表現の安定化
- MIR JSON の `Literal` における `set` 形状を明文化する。
- 必要なら `docs/schemas` に JSON Schema を追加し、構造を固定する。
- `set` の要素順序と重複の扱い（仕様上の意味）を明文化する。
  - [x] MIR/JSON 仕様メモ
  - [x] Schema 追加の要否判断

### フェーズ 1 実施メモ（MIR/JSON 形状）

#### MIR/JSON 仕様メモ
- MIR の `Literal` は `compiler/frontend/src/semantics/mir.rs` で `MirExprKind::Literal(Literal)` として直列化される。`Literal` は `compiler/frontend/src/parser/ast.rs` の `Literal` をそのまま保持するため、JSON では `value` フィールドが二重になる。
- セットリテラルの JSON 形状（概略）は以下の通り。

```json
{
  "kind": "literal",
  "value": {
    "value": {
      "kind": "set",
      "elements": [ /* Expr JSON */ ]
    }
  }
}
```

- `elements` は `Vec<Expr>` の直列化結果であり、MIR の `MirExprId` ではなく AST 由来の式ノードが埋め込まれる。
- `elements` の順序はソース順を保持し、重複の除去や正規化は行われない（重複判定は `Set` 実装側の `Ord` 制約/収集時に委ねられる前提）。

#### Schema 追加の要否判断
- `docs/schemas` には MIR/JSON 用の Schema が存在しないため、フェーズ 1 ではスキーマ追加は見送り（必要ならフェーズ 2 以降で MIR/JSON 全体の schema 追加と合わせて検討する）。

### フェーズ 2: Backend 型マッピングとコード生成
- `parse_reml_type` で `Set<T>` を識別できるようにする（最初は `pointer` でも可）。
- `emit_value_expr` で `LiteralKind::Set` の構築処理を追加する。
- `Set<T>` 生成に必要なランタイム呼び出しを設計し、Backend から呼べる形にする。
  - [x] 型マッピング方針の決定
  - [x] セットリテラルのコード生成設計

### フェーズ 2 設計メモ

#### 型マッピング方針（Backend）
- `Set<T>` は Runtime オブジェクトとして扱い、Backend 側は `ptr` 相当（`RemlType::Pointer`）で通す方針。
- `Set<T>` の要素型 `T` の `Ord` 制約は型検査で担保済みとし、Backend は実体構築時に `T` の比較/ハッシュ実装の存在を仮定する（ABI で比較関数を渡す設計は Phase 3 以降の拡張余地）。
- `parse_reml_type` で `Set<T>` を識別し、型マッピングは `pointer` としつつ `TypeLayout::description` に `set<...>` を残す案を記録しておく（ログ/デバッグ用）。

#### セットリテラルのコード生成設計（方向性）
- `{...}` は `LiteralKind::Set { elements }` として MIR に残るため、Backend は要素式を評価してから Runtime の `set_insert` を順に呼ぶシンプルな構築手順を採用する。
- 要素順はソース順を維持し、重複排除は Runtime の `set_insert` に委譲する。
- 生成フロー案（擬似）:
  - `ptr = reml_set_new()`
  - `for elem in elements: ptr = reml_set_insert(ptr, elem)`
  - `return ptr`

#### Runtime 側の最小 ABI 形状案（先行整理）
- オブジェクト表現: `reml_set_t` はヒープ上の不透明ポインタ（`void*`）として扱い、`reml_type_tag_t` に `REML_TAG_SET` を追加する。
- 最小 ABI 案（C シグネチャ）:
  - `void* reml_set_new(void);`
  - `void* reml_set_insert(void* set_ptr, void* value_ptr);`
  - `int32_t reml_set_contains(void* set_ptr, void* value_ptr);`
  - `int64_t reml_set_len(void* set_ptr);`
- `set_insert` は永続構造前提で新しいセットを返す想定。可変セットに寄せる場合は `void reml_set_insert_mut(void* set_ptr, void* value_ptr);` を別途検討。
- 比較/順序づけは当面 `Ord` 制約に対応するランタイムの比較フック（例: `reml_ord_compare`）に委譲する前提で設計し、Phase 3 以降の ABI 拡張で具体化する。

#### Backend 改修ポイント（洗い出し）
- `compiler/backend/llvm/src/integration.rs` の `parse_reml_type` は `Set<T>` のトークンを未対応（`ptr` へフォールバック）なので、`Set<...>` を認識する分岐を追加する必要がある。
- `compiler/backend/llvm/src/type_mapping.rs` に `RemlType::Set(Box<RemlType>)` を追加する場合は `layout_of` を `ptr` 相当で扱い、`description` だけを `set<...>` に寄せる（Phase 2 では ABI を固定しない前提）。
- `compiler/backend/llvm/src/integration.rs` の `value_summary` は `Literal` を JSON 文字列化して `MirExprKind::Literal { summary }` に渡すため、`emit_value_expr` 側で `summary` が JSON であることを前提に `LiteralKind::Set` を判別する必要がある。
- `compiler/backend/llvm/src/codegen.rs` の `emit_value_expr` は `extract_literal_operand` が `int/string/bool` だけを扱っているため、`set` を検出したら `reml_set_new`/`reml_set_insert` の呼び出し列を生成する分岐を追加する（要素評価の順序と SSA 値の保持が必要）。

#### Runtime 型・破棄方針（reml_runtime.h 反映案）
- `reml_type_tag_t` に `REML_TAG_SET` を追加し、`dec_ref` の型別デストラクタに `destroy_set` を追加する。
- `reml_set_t` の C 側表現は Phase 2 では不透明ポインタのままにし、`destroy_set` の責務として「内部要素の `dec_ref`」と「内部ノードの解放」を持たせる設計を前提にする。
- 破棄時に要素を順に `dec_ref` できるよう、最小 ABI には `reml_set_iter_begin`/`reml_set_iter_next` を設けるか、ランタイム実装内で直接反復できる構造を選ぶ必要がある（Phase 3 で詳細設計）。

### フェーズ 3: Runtime 実装（最小 ABI）
- `compiler/runtime/native` に Set の実装を追加する（最小 API のみ）。
- ABI 関数の命名規約と引数/戻り値を定義する。
- 将来の最適化を見据えたデータ構造の選定を記録する。
  - [x] Set 実装の追加
  - [x] ABI 関数の定義

### フェーズ 3 実施メモ

#### Runtime Set 実装（最小 ABI）
- `compiler/runtime/native/include/reml_runtime.h` に `REML_TAG_SET` と `reml_set_*` の ABI を追加。
- `compiler/runtime/native/src/set.c` で永続 Set の最小実装を追加（要素配列 + ポインタ同値）。
- `reml_set_insert` は常に新しい Set を返し、要素は `inc_ref` で保持する。

#### データ構造選定（暫定）
- 現段階では `len/capacity/items` を持つ単純配列ベースの実装。
- 重複判定はポインタ同値で行い、`Ord`/比較フックの導入は Phase 4 以降で検討。

### フェーズ 4: テストと検証
- Backend の差分スナップショットに `set` リテラルの例を追加する。
- Runtime 側にセット構築/要素追加の基本テストを追加する。
- Frontend → Backend → Runtime の結合確認（最小サンプル）を行う。
  - [x] Backend スナップショット追加
  - [x] Runtime テスト追加
  - [x] 結合確認の手順整理

### フェーズ 4 実施メモ
1. Backend スナップショット: `reports/backend-ir-diff/reml-set-literal-mir.json` と `reports/backend-ir-diff/reml-set-literal-log.json` を追加。
2. Runtime テスト: `compiler/runtime/native/tests/test_set.c` で `set_new`/`set_insert`/`set_contains`/`set_len` を検証。
3. 結合確認（最小）手順: `compiler/frontend` で MIR JSON を出力 → `MIR_PATH=... cargo test --manifest-path compiler/backend/llvm/Cargo.toml dump_llvm_ir_from_mir_path -- --ignored --nocapture` で `@reml_set_new`/`@reml_set_insert` を確認 → `make test` (`compiler/runtime/native`) で ABI を検証。

### フェーズ 5: ドキュメントの更新
- `docs/spec/2-3-lexer.md` から参照できる形で Set の実行時表現を記録する。
- stdlib 仕様に Set API がある場合は追記する。
  - [x] 仕様メモ/参照の追加

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
- `docs/spec/2-3-lexer.md`
- `compiler/frontend/src/parser/ast.rs`
- `compiler/frontend/src/semantics/mir.rs`
- `compiler/backend/llvm/src/codegen.rs`
- `compiler/runtime/native/README.md`
