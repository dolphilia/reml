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
  - [ ] `compiler/rust/frontend` の `LiteralKind::Record` 形状を確認
  - [ ] Backend/Runtime の `reml_record_t` / `LiteralSummary::Record` の実装状況を確認

### フェーズ 1: レイアウト規則の確定
- フィールド順序（ソース順 / 型定義順 / 正規化順）の選択と理由を整理する。
- レコード型の同値性（構造的等値）とレイアウト順序の関係を整理する。
- フィールド名の保持（名前情報の保管先、Runtime での参照可否）を決める。
  - [ ] フィールド順序規則を確定
  - [ ] フィールド名の扱い（保持/非保持）を確定
  - [ ] 仕様に明記する診断と制約を整理

### フェーズ 2: 仕様反映
- `docs/spec/1-1-syntax.md` に Record のレイアウト規則を追記する。
- `docs/spec/1-2-types-Inference.md` に型推論・同値性との関係を追記する。
  - [ ] 仕様更新（構文/型推論）
  - [ ] 未確定事項の `@unstable` を撤去

### フェーズ 3: Backend/Runtime 反映
- Backend の構築順序を仕様に合わせて固定する。
- Runtime の `reml_record_t` が意味論に一致することを確認する（必要なら ABI の見直しを提案）。
  - [ ] Backend の構築順序を更新
  - [ ] Runtime ABI の適合確認

### フェーズ 4: テストと検証
- Record リテラルの順序・構築規則のテストを追加する。
- Backend IR のスナップショットで `reml_record_*` 呼び出し順序を検証する。
  - [ ] テスト追加
  - [ ] スナップショット確認

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
