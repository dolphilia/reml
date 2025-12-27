# 1.5 Backend/Runtime Array リテラル意味論確定計画（2025-12-26）

Array リテラル `[ ... ]` が `([T; N])` / `[T]` のどちらへ降りるか、および Runtime `reml_array_t` の意味論を確定し、仕様と実装の不整合を解消するための計画書。

## 目的
- Array リテラルの型推論規則と実行時表現を明確化する。
- Backend/Runtime の ABI と仕様の整合をとる。
- 未確定事項（`literal_array_semantics`）を仕様化し、実装へ反映する。

## 対象範囲
- 仕様: `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`
- Backend: `compiler/rust/backend/llvm`
- Runtime: `runtime/native`

## 前提・現状
- AST/MIR の `LiteralKind::Array` は `elements` を保持するが、固定長/動的の区別は持たない。
- Runtime の `reml_array_t` は動的配列相当の最小 ABI として導入済み。
- 仕様上は `[T; N]` と `[T]` が型として存在するが、リテラルの降ろし方が未確定。

## 実行計画

### フェーズ 0: 仕様・実装の現状確認
- Array リテラルの仕様記述と型推論の記載を整理する。
- Frontend の AST/MIR と Backend のリテラル解釈を確認する。
  - [ ] `docs/spec/1-1-syntax.md` / `docs/spec/1-2-types-Inference.md` の Array 記述を確認
  - [ ] `compiler/rust/frontend` の `LiteralKind::Array` 形状を確認
  - [ ] Backend/Runtime の `reml_array_t` / `LiteralSummary::Array` の実装状況を確認

### フェーズ 1: 意味論・型推論ルールの確定
- `[T; N]` / `[T]` の既定を決定する（例: 既定は `[T; N]`、明示型注釈がない場合は動的配列へ昇格など）。
- `N` の算出規則（要素数、空配列、ネスト）と、型注釈との整合を整理する。
- 既定ルールに反するケースの診断方針を定義する。
  - [ ] Array リテラルの型推論規則（既定・例外・エラー条件）を確定
  - [ ] 仕様に明記する診断名とメッセージの方向性を決める

### フェーズ 2: 仕様反映
- `docs/spec/1-1-syntax.md` に Array リテラルの意味論と補足を追記する。
- `docs/spec/1-2-types-Inference.md` に型推論ルールを追記する。
  - [ ] 仕様更新（構文/型推論）
  - [ ] 未確定事項の `@unstable` を撤去

### フェーズ 3: Backend/Runtime 反映
- 既定ルールに合わせて Backend のリテラル解釈を調整する。
- Runtime の `reml_array_t` が意味論に一致することを確認する（必要なら ABI の見直しを提案）。
  - [ ] Backend のリテラル解釈を更新
  - [ ] Runtime ABI の適合確認

### フェーズ 4: テストと検証
- 型推論（固定長/動的）のテストケースを追加する。
- Backend IR のスナップショットで `reml_array_*` 呼び出しを検証する。
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
