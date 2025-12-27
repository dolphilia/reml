# 1.6 Backend/Runtime Record レイアウト確定計画（2025-12-26）

Record リテラル `{ x: 1, y: 2 }` のフィールド順序と Runtime レイアウトを確定し、`reml_record_t` の ABI を安定させるための計画書。

## 目的
- Record のフィールド順序規則（ソース順 / 型定義順 / 文字列ソート等）を確定する。
- Runtime の `reml_record_t` と Backend の構築順序を一致させる。
- 未確定事項（`literal_record_layout`）を仕様化し、実装へ反映する。

## 対象範囲
- 仕様: `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`
- Backend: `compiler/rust/backend/llvm`
- Runtime: `runtime/native`

## 前提・現状
- AST/MIR の `LiteralKind::Record` は `fields` 配列を保持し、現状はソース順を保持する。
- Runtime の `reml_record_t` は `field_count` と `values` を持つ最小 ABI。
- フィールド順序とフィールド名の保持戦略が未確定。

## 実行計画

### フェーズ 0: 仕様・実装の現状確認
- Record リテラルの仕様記述と型推論の記載を整理する。
- Frontend の AST/MIR と Backend のリテラル解釈を確認する。
  - [ ] `docs/spec/1-1-syntax.md` / `docs/spec/1-2-types-Inference.md` の Record 記述を確認
  - [ ] 仕様中の `@unstable` / TODO / 留保記述を洗い出して一覧化する
  - [ ] Record 型の等値性（構造的/名義的）に関する記述の有無を確認する
  - [ ] フィールド順序に関する記載や暗黙ルールがないかを点検する
  - [ ] `compiler/rust/frontend` の `LiteralKind::Record` 形状を確認
  - [ ] Typed/MIR で保持される record 情報（フィールド名・順序・型注釈）を整理する
  - [ ] 型推論フェーズで record フィールドがどの順序で扱われるかを追跡する
  - [ ] Backend/Runtime の `reml_record_t` / `LiteralSummary::Record` の実装状況を確認
  - [ ] Backend で record フィールドが並べ替えられていないか（source order の維持可否）を確認する
  - [ ] Runtime 側の record 生成・破棄 API の有無と利用箇所を洗い出す

### フェーズ 1: レイアウト規則の確定
- フィールド順序（ソース順 / 型定義順 / 正規化順）の選択と理由を整理する。
- レコード型の同値性（構造的等値）とレイアウト順序の関係を整理する。
- フィールド名の保持（名前情報の保管先、Runtime での参照可否）を決める。
  - [ ] フィールド順序規則を確定
  - [ ] 順序規則における決定性（コンパイル間/プラットフォーム間）を確認する
  - [ ] 同名フィールド重複時の扱い（禁止/後勝ち/診断）を定義する
  - [ ] 型注釈付き record リテラルでの順序規則（型定義順への整列有無）を決める
  - [ ] フィールド名の扱い（保持/非保持）を確定
  - [ ] フィールド名メタ情報の保存場所（コンパイル時のみ/Runtime 常駐）を決める
  - [ ] フィールドアクセス（`record.x`）がレイアウト順序に依存する前提を整理する
  - [ ] 仕様に明記する診断と制約を整理

### フェーズ 2: 仕様反映
- `docs/spec/1-1-syntax.md` に Record のレイアウト規則を追記する。
- `docs/spec/1-2-types-Inference.md` に型推論・同値性との関係を追記する。
  - [ ] 仕様更新（構文/型推論）
  - [ ] フィールド順序規則の例（ソース順/型定義順の差異）を追加する
  - [ ] フィールド名保持方針と runtime 表現の説明を追記する
  - [ ] 診断条件（重複フィールド、注釈不整合、順序違反）を明文化する
  - [ ] 未確定事項の `@unstable` を撤去

### フェーズ 3: Backend/Runtime 反映
- Backend の構築順序を仕様に合わせて固定する。
- Runtime の `reml_record_t` が意味論に一致することを確認する（必要なら ABI の見直しを提案）。
  - [ ] Backend の構築順序を更新
  - [ ] record フィールドの並べ替えロジック（必要ならソート/整列）を実装する
  - [ ] フィールド名とインデックスの対応を Backend で確実に保持する
  - [ ] Runtime ABI の適合確認
  - [ ] `reml_record_t` のフィールド配列と Backend の順序が一致することを確認する
  - [ ] 既存 ABI の変更が必要なら互換性影響を整理する

### フェーズ 4: テストと検証
- Record リテラルの順序・構築規則のテストを追加する。
- Backend IR のスナップショットで `reml_record_*` 呼び出し順序を検証する。
  - [ ] テスト追加
  - [ ] 代表ケース（同一型/異なる順序/型注釈あり/重複フィールド）を網羅する
  - [ ] フィールド順序が期待通りに固定されることを確認するテストを用意する
  - [ ] スナップショット確認
  - [ ] 期待する IR 形状（record 生成、フィールド格納順序）を明文化する

## 進捗管理
- 本計画書作成日: 2025-12-26
- 進捗欄（運用用）:
  - [ ] フェーズ 0 完了
  - [ ] フェーズ 1 完了
  - [ ] フェーズ 2 完了
  - [ ] フェーズ 3 完了
  - [ ] フェーズ 4 完了

## 関連リンク
- `docs/spec/1-1-syntax.md`
- `docs/spec/1-2-types-Inference.md`
- `compiler/rust/frontend/src/parser/ast.rs`
- `compiler/rust/frontend/src/semantics/mir.rs`
- `compiler/rust/backend/llvm/src/codegen.rs`
- `runtime/native/include/reml_runtime.h`
