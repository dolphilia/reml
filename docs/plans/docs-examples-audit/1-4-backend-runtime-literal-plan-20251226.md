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
  - [ ] `compiler/rust/frontend/src/semantics/mir.rs` の `Literal` 構造と JSON 変換箇所を洗い出す
  - [ ] `Literal` ごとの JSON 形状（キー名、配列/オブジェクトの入れ子、型ヒント）を一覧化する
  - [ ] 仕様書（`docs/spec`）のリテラル定義箇所を特定し、意味論の不足点をメモする
  - [ ] Float/Char/Tuple/Array/Record の未確定事項（精度、表現、型推論ルール）を TODO として整理する
  - [ ] まとめを本計画書に追記してレビュー待ちにする

### フェーズ 1: Backend リテラル解釈の追加
- `Literal` サマリから Float/Char/Tuple/Array/Record を識別できるようにする。
- `emit_value_expr` と型推論補助の「literal 解析」を拡張する。
- 未対応の型は明示的に診断ログに残す。
  - [ ] `compiler/rust/backend/llvm/src/codegen.rs` で現状の `Literal` 解釈パスを把握する
  - [ ] JSON 形状に対応した判定分岐と、内部表現（型タグ/初期化式）を設計する
  - [ ] Float/Char/Tuple/Array/Record の解釈結果を `emit_value_expr` に接続する
  - [ ] 未対応リテラルの診断メッセージを整理し、識別子命名規則に合わせる
  - [ ] 既存リテラル（int/string/bool/unit）への影響がないことを確認する

### フェーズ 2: Runtime 型タグと ABI の定義
- `REML_TAG_*` に Char/Array のタグを追加する。
- Tuple/Record/Array の最小構造を C 側で定義する（破棄処理含む）。
- Char の表現（UTF-8 1byte or Unicode scalar）を決める。
  - [ ] `runtime/native/include/reml_runtime.h` の既存タグを確認し、追加タグの値レンジを決める
  - [ ] Tuple/Record/Array のレイアウト（ヘッダ、長さ、要素ポインタ）を最小構成で設計する
  - [ ] Char の表現を仕様に合わせて決定し、ABI への影響点をメモする
  - [ ] ABI レイアウトのコメントをヘッダに追記し、Backend が参照できる形にする
  - [ ] 破棄処理のインタフェース（関数名、引数、責務）を定義する

### フェーズ 3: Runtime 実装（最小機能）
- Tuple/Record/Array の破棄処理を最低限実装する。
- Float/Char のボックス化/アンボックス化の補助関数を追加する。
- 参照カウント管理の適用範囲を明文化する。
  - [ ] `runtime/native/src/refcount.c` の既存実装を確認し、タグ別の分岐を追加する
  - [ ] Tuple/Record/Array の要素走査と参照カウント減算を実装する
  - [ ] Float/Char 用の boxing/unboxing API を追加し、ヘッダに宣言する
  - [ ] 参照カウント対象（ヒープ/即値）の区分を文書化する
  - [ ] Backend が呼び出す補助関数の利用例をメモに残す

### フェーズ 4: テストと結合検証
- Backend スナップショットに各リテラルの例を追加する。
- Runtime のユニットテスト（破棄/参照カウント）を追加する。
- Frontend → Backend → Runtime の最小経路を確認する。
  - [ ] Backend のスナップショットテストに Float/Char/Tuple/Array/Record の入力例を追加する
  - [ ] Literal の JSON 形状が想定どおりに解釈されることを検証する
  - [ ] Runtime の破棄処理で参照カウントが正しく減少するテストを追加する
  - [ ] Char/Float の boxing/unboxing が往復で一致するテストを追加する
  - [ ] 最小経路の手順（対象ソース、コマンド、期待出力）を計画書に記載する

### フェーズ 5: ドキュメント更新
- 実装済みのリテラル表現と ABI を仕様に反映する。
- 未対応項目は `@unstable` 等で明示する。
  - [ ] `docs/spec` の該当章に JSON 形状と ABI を追記する
  - [ ] Runtime 側のタグ/構造体定義を仕様に反映する
  - [ ] 未対応・実験中の項目を `@unstable` で明示する
  - [ ] 仕様と実装の差分が残る場合は後続タスクを記録する

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
