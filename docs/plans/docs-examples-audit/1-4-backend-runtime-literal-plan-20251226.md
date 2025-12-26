# 1.4 Backend/Runtime リテラル対応計画（2025-12-26）

フロントエンド AST/MIR には存在するが、Backend でリテラル構造の解釈がなく、Runtime でも実装が未整備なリテラルについて、将来のコード生成に備えた対応計画をまとめる。

## 目的
- MIR/JSON のリテラル表現を安定させ、Backend が構造を解釈できる状態にする。
- Runtime に必要な型タグ・ABI・実体構造を用意し、Backend と整合させる。
- 仕様と実装の差分を段階的に埋めるための優先順位を整理する。

## 対象範囲
- Backend: `compiler/rust/backend/llvm`
- Runtime: `runtime/native`
- フロントエンド: `compiler/rust/frontend/src/parser/ast.rs` `compiler/rust/frontend/src/semantics/mir.rs`

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
  - [ ] MIR/JSON 形状の整理
  - [ ] 仕様参照メモの作成

### フェーズ 1: Backend リテラル解釈の追加
- `Literal` サマリから Float/Char/Tuple/Array/Record を識別できるようにする。
- `emit_value_expr` と型推論補助の「literal 解析」を拡張する。
- 未対応の型は明示的に診断ログに残す。
  - [ ] Backend 解析ロジックの設計
  - [ ] Backend の最小実装

### フェーズ 2: Runtime 型タグと ABI の定義
- `REML_TAG_*` に Char/Array のタグを追加する。
- Tuple/Record/Array の最小構造を C 側で定義する（破棄処理含む）。
- Char の表現（UTF-8 1byte or Unicode scalar）を決める。
  - [ ] 型タグ追加
  - [ ] ABI/構造体の定義

### フェーズ 3: Runtime 実装（最小機能）
- Tuple/Record/Array の破棄処理を最低限実装する。
- Float/Char のボックス化/アンボックス化の補助関数を追加する。
- 参照カウント管理の適用範囲を明文化する。
  - [ ] 破棄処理の最小実装
  - [ ] ボックス化/補助関数の追加

### フェーズ 4: テストと結合検証
- Backend スナップショットに各リテラルの例を追加する。
- Runtime のユニットテスト（破棄/参照カウント）を追加する。
- Frontend → Backend → Runtime の最小経路を確認する。
  - [ ] Backend テスト追加
  - [ ] Runtime テスト追加
  - [ ] 結合確認の手順整理

### フェーズ 5: ドキュメント更新
- 実装済みのリテラル表現と ABI を仕様に反映する。
- 未対応項目は `@unstable` 等で明示する。
  - [ ] 仕様追記

## 進捗管理
- 本計画書作成日: 2025-12-26
- 進捗欄（運用用）:
  - [ ] フェーズ 0 完了
  - [ ] フェーズ 1 完了
  - [ ] フェーズ 2 完了
  - [ ] フェーズ 3 完了
  - [ ] フェーズ 4 完了
  - [ ] フェーズ 5 完了

## 関連リンク
- `compiler/rust/frontend/src/parser/ast.rs`
- `compiler/rust/frontend/src/semantics/mir.rs`
- `compiler/rust/backend/llvm/src/codegen.rs`
- `runtime/native/include/reml_runtime.h`
- `runtime/native/src/refcount.c`
