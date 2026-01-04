# 1.6 Backend/Runtime Record レイアウト確定計画（2025-12-26）

Record リテラル `{ x: 1, y: 2 }` のフィールド順序と Runtime レイアウトを確定し、`reml_record_t` の ABI を安定させるための計画書。

## 目的
- Record のフィールド順序規則（ソース順 / 型定義順 / 文字列ソート等）を確定する。
- Runtime の `reml_record_t` と Backend の構築順序を一致させる。
- 未確定事項（`literal_record_layout`）を仕様化し、実装へ反映する。

## 対象範囲
- 仕様: `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`
- Backend: `compiler/backend/llvm`
- Runtime: `compiler/runtime/native`

## 前提・現状
- AST/MIR の `LiteralKind::Record` は `fields` 配列を保持し、現状はソース順を保持する。
- Runtime の `reml_record_t` は `field_count` と `values` を持つ最小 ABI。
- フィールド順序とフィールド名の保持戦略が未確定。

## 実行計画

### フェーズ 0: 仕様・実装の現状確認
- Record リテラルの仕様記述と型推論の記載を整理する。
- Frontend の AST/MIR と Backend のリテラル解釈を確認する。
  - [x] `docs/spec/1-1-syntax.md` / `docs/spec/1-2-types-Inference.md` の Record 記述を確認
  - [x] 仕様中の `@unstable` / TODO / 留保記述を洗い出して一覧化する
  - [x] Record 型の等値性（構造的/名義的）に関する記述の有無を確認する
  - [x] フィールド順序に関する記載や暗黙ルールがないかを点検する
  - [x] `compiler/frontend` の `LiteralKind::Record` 形状を確認
  - [x] Typed/MIR で保持される record 情報（フィールド名・順序・型注釈）を整理する
  - [x] 型推論フェーズで record フィールドがどの順序で扱われるかを追跡する
  - [x] Backend/Runtime の `reml_record_t` / `LiteralSummary::Record` の実装状況を確認
  - [x] Backend で record フィールドが並べ替えられていないか（source order の維持可否）を確認する
  - [x] Runtime 側の record 生成・破棄 API の有無と利用箇所を洗い出す

#### フェーズ 0 確認結果
- 仕様: `docs/spec/1-1-syntax.md` は Record リテラルを「順序不問」と明記し、`docs/spec/1-1-syntax.md` E.1.2 で `@unstable("literal_record_layout")` として values 配列順序は Backend 決定（現状ソース順）と記載。
- 仕様: `docs/spec/1-2-types-Inference.md` で Record 型は「構造的等値」と明記、順序規則は未記載。
- Frontend AST: `compiler/frontend/src/parser/ast.rs` の `LiteralKind::Record { type_name, fields }` が `Vec<RecordField>` を保持し、`compiler/frontend/src/parser/mod.rs` のパーサはソース順で `fields` を構築（型注釈は `type_name` のみ）。
- Typed/MIR: `compiler/frontend/src/semantics/typed.rs` と `compiler/frontend/src/semantics/mir.rs` は `Literal` をそのまま保持し、Record の並べ替えや追加メタ情報は無し。
- 型推論: `compiler/frontend/src/typeck/driver.rs` の `LiteralKind::Record` 推論は `fields` 順で型を収集し、`Type::app("Record", field_types)` を生成（フィールド名・`type_name` は型に反映されない）。
- Backend: `compiler/backend/llvm/src/codegen.rs` の `LiteralSummary::Record` は `type_name` と `field_count` のみ取得し、Record リテラルは未対応で診断へフォールバック（順序は扱わない）。
- Runtime: `compiler/runtime/native/include/reml_runtime.h` の `reml_record_t` は `field_count` と `values` 配列のみで、`compiler/runtime/native/src/refcount.c` に `reml_destroy_record` が存在。生成 API は未定義で、テストは手動確保。

### フェーズ 1: レイアウト規則の確定
- フィールド順序（ソース順 / 型定義順 / 正規化順）の選択と理由を整理する。
- レコード型の同値性（構造的等値）とレイアウト順序の関係を整理する。
- フィールド名の保持（名前情報の保管先、Runtime での参照可否）を決める。
  - [x] フィールド順序規則を確定
  - [x] 順序規則における決定性（コンパイル間/プラットフォーム間）を確認する
  - [x] 同名フィールド重複時の扱い（禁止/後勝ち/診断）を定義する
  - [x] 型注釈付き record リテラルでの順序規則（型定義順への整列有無）を決める
  - [x] フィールド名の扱い（保持/非保持）を確定
  - [x] フィールド名メタ情報の保存場所（コンパイル時のみ/Runtime 常駐）を決める
  - [x] フィールドアクセス（`record.x`）がレイアウト順序に依存する前提を整理する
  - [x] 仕様に明記する診断と制約を整理

#### フェーズ 1 決定事項（レイアウト規則）

**フィールド順序規則**
- Record の `values` 配列順序は **フィールド名の正規化順（Unicode スカラ値の昇順）**とする。
- 正規化は **識別子文字列の辞書順**で行い、**ロケールやケース折り畳みは考慮しない**（入力ソースの識別子表記がそのまま順序に影響）。
- フィールド式の評価順は **ソース順**を維持し、**格納順序のみ**正規化順へ整列する。

**決定性**
- 順序は **ソース・プラットフォーム・コンパイラ実装差に依存しない**。Unicode スカラ値順を仕様で固定する。

**同名フィールド重複の扱い**
- レコードリテラル／レコード型／レコードパターンの **同名フィールドはすべて禁止**し、診断を出す（後勝ち等は採用しない）。

**型注釈付き record リテラル**
- 明示注釈（`{...} : { ... }` など）や型名付きリテラルがある場合でも **レイアウト順序は正規化順固定**とする。
- 注釈側のフィールド集合とリテラル側のフィールド集合が一致しない場合は診断（不足・余剰）とする。

**フィールド名の扱い**
- `reml_record_t` は **値配列のみ**を保持し、フィールド名は Runtime に保持しない。
- フィールド名のメタ情報は **コンパイル時（型情報/デバッグ情報）**に保持する前提とする。

**フィールドアクセス**
- `record.x` は **コンパイル時にフィールド名→インデックスへ解決**される。
- インデックス算出は **正規化順**に基づき、リテラルのソース順に依存しない。

**診断と制約（仕様明記対象）**
- `type.record.literal.duplicate_field`: リテラル内の同名フィールド重複。
- `type.record.literal.missing_field`: 型注釈に存在するがリテラルに欠けるフィールド。
- `type.record.literal.unknown_field`: リテラルに存在するが注釈型に存在しないフィールド。
- `type.record.access.unknown_field`: 存在しないフィールドへのアクセス。

### フェーズ 2: 仕様反映
- `docs/spec/1-1-syntax.md` に Record のレイアウト規則を追記する。
- `docs/spec/1-2-types-Inference.md` に型推論・同値性との関係を追記する。
  - [x] 仕様更新（構文/型推論）
  - [x] フィールド順序規則の例（ソース順/型定義順の差異）を追加する
  - [x] フィールド名保持方針と runtime 表現の説明を追記する
  - [x] 診断条件（重複フィールド、注釈不整合、順序違反）を明文化する
  - [x] 未確定事項の `@unstable` を撤去

### フェーズ 3: Backend/Runtime 反映
- Backend の構築順序を仕様に合わせて固定する。
- Runtime の `reml_record_t` が意味論に一致することを確認する（必要なら ABI の見直しを提案）。
  - [x] Backend の構築順序を更新
  - [x] record フィールドの並べ替えロジック（必要ならソート/整列）を実装する
  - [x] フィールド名とインデックスの対応を Backend で確実に保持する
  - [x] Runtime ABI の適合確認
  - [x] `reml_record_t` のフィールド配列と Backend の順序が一致することを確認する
  - [x] 既存 ABI の変更が必要なら互換性影響を整理する

### フェーズ 4: テストと検証
- Record リテラルの順序・構築規則のテストを追加する。
- Backend IR のスナップショットで `reml_record_*` 呼び出し順序を検証する。
  - [x] テスト追加
  - [x] 代表ケース（同一型/異なる順序/型注釈あり/重複フィールド）を網羅する
  - [x] フィールド順序が期待通りに固定されることを確認するテストを用意する
  - [x] スナップショット確認
  - [x] 期待する IR 形状（record 生成、フィールド格納順序）を明文化する

#### フェーズ 4 補足（IR 形状の期待値）
- `record literal field_count=... type_name=...` コメントが生成される。
- `record field N -> <field>` コメントはソース順で出現する。
- `record slot N = <field>` コメントは正規化順（Unicode スカラ値昇順）で出現する。
- `@reml_record_from(i64 <field_count>, ...)` が生成される。

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
