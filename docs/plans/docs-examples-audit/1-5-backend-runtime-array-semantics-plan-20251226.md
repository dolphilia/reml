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
  - [ ] 仕様の用語（固定長配列/動的配列、リテラル、型注釈）の定義揺れを洗い出す
  - [ ] 仕様中の未確定事項・注記（`@unstable`/TODO/留保）を抜き出して一覧化する
  - [ ] `compiler/rust/frontend` の `LiteralKind::Array` 形状を確認
  - [ ] Array リテラルが MIR/型推論へ渡る時点の情報（要素型・長さ・注釈）を整理する
  - [ ] Backend/Runtime の `reml_array_t` / `LiteralSummary::Array` の実装状況を確認
  - [ ] Backend で配列がどの型として扱われているか（固定長/動的）を追跡する
  - [ ] Runtime ABI（要素ポインタ、長さ、容量、アラインメント等）の現状仕様を整理する

### フェーズ 1: 意味論・型推論ルールの確定
- `[T; N]` / `[T]` の既定を決定する（例: 既定は `[T; N]`、明示型注釈がない場合は動的配列へ昇格など）。
- `N` の算出規則（要素数、空配列、ネスト）と、型注釈との整合を整理する。
- 既定ルールに反するケースの診断方針を定義する。
  - [ ] Array リテラルの型推論規則（既定・例外・エラー条件）を確定
  - [ ] 空配列 `[]` の型推論（既定型・注釈必須条件・補完戦略）を決める
  - [ ] 異種要素を含むリテラルの許否と型統一規則（上限型/エラー）を定義する
  - [ ] 型注釈がある場合の優先順位（注釈優先/推論優先）を明文化する
  - [ ] `[T; N]` と `[T]` の相互変換可否（暗黙変換の有無、明示変換のみ）を決める
  - [ ] 仕様に明記する診断名とメッセージの方向性を決める
  - [ ] 代表的な診断シナリオ（要素数不一致、注釈違反、空配列注釈不足）を列挙する

### フェーズ 2: 仕様反映
- `docs/spec/1-1-syntax.md` に Array リテラルの意味論と補足を追記する。
- `docs/spec/1-2-types-Inference.md` に型推論ルールを追記する。
  - [ ] 仕様更新（構文/型推論）
  - [ ] Array リテラルの段落に推論例（固定長/動的/空配列）を追加する
  - [ ] 型推論節に診断条件と回避策（注釈の付け方）を追記する
  - [ ] 未確定事項の `@unstable` を撤去

### フェーズ 3: Backend/Runtime 反映
- 既定ルールに合わせて Backend のリテラル解釈を調整する。
- Runtime の `reml_array_t` が意味論に一致することを確認する（必要なら ABI の見直しを提案）。
  - [ ] Backend のリテラル解釈を更新
  - [ ] 固定長配列/動的配列で生成される IR の差分を整理する
  - [ ] リテラル生成時のアロケーション戦略（stack/heap）を確認する
  - [ ] Runtime ABI の適合確認
  - [ ] `reml_array_t` のフィールド定義と生成コードの整合性をチェックする
  - [ ] ABI 変更が必要なら互換性影響と移行方針をまとめる

### フェーズ 4: テストと検証
- 型推論（固定長/動的）のテストケースを追加する。
- Backend IR のスナップショットで `reml_array_*` 呼び出しを検証する。
  - [ ] テスト追加
  - [ ] 代表ケース（空/単一/複数/ネスト）を網羅するテストを用意する
  - [ ] スナップショット確認
  - [ ] 期待する IR 形状（配列生成、長さ、要素格納）を明文化する

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
