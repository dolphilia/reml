# 1.3 Backend/Runtime Set 対応計画（2025-12-26）

`docs/spec/2-3-lexer.md` で復元した集合リテラル `{...}` と `Set<T>` を、将来のコード生成で扱えるように Backend/Runtime 側の対応方針を整理する計画書。

## 目的
- フロントエンドが出力する MIR/JSON から `Set<T>` を安全にコード生成できる状態を作る。
- `Set<T>` の実行時表現と ABI を Runtime に定義し、Backend と一致させる。
- 仕様・実装・テストの整合を保ち、将来の最適化に備える。

## 対象範囲
- Backend: `compiler/rust/backend/llvm`
- Runtime: `runtime/native`
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
  - [ ] 仕様メモの作成
  - [ ] 既存コレクション実装の調査

### フェーズ 1: MIR/JSON 表現の安定化
- MIR JSON の `Literal` における `set` 形状を明文化する。
- 必要なら `docs/schemas` に JSON Schema を追加し、構造を固定する。
- `set` の要素順序と重複の扱い（仕様上の意味）を明文化する。
  - [ ] MIR/JSON 仕様メモ
  - [ ] Schema 追加の要否判断

### フェーズ 2: Backend 型マッピングとコード生成
- `parse_reml_type` で `Set<T>` を識別できるようにする（最初は `pointer` でも可）。
- `emit_value_expr` で `LiteralKind::Set` の構築処理を追加する。
- `Set<T>` 生成に必要なランタイム呼び出しを設計し、Backend から呼べる形にする。
  - [ ] 型マッピング方針の決定
  - [ ] セットリテラルのコード生成設計

### フェーズ 3: Runtime 実装（最小 ABI）
- `runtime/native` に Set の実装を追加する（最小 API のみ）。
- ABI 関数の命名規約と引数/戻り値を定義する。
- 将来の最適化を見据えたデータ構造の選定を記録する。
  - [ ] Set 実装の追加
  - [ ] ABI 関数の定義

### フェーズ 4: テストと検証
- Backend の差分スナップショットに `set` リテラルの例を追加する。
- Runtime 側にセット構築/要素追加の基本テストを追加する。
- Frontend → Backend → Runtime の結合確認（最小サンプル）を行う。
  - [ ] Backend スナップショット追加
  - [ ] Runtime テスト追加
  - [ ] 結合確認の手順整理

### フェーズ 5: ドキュメントの更新
- `docs/spec/2-3-lexer.md` から参照できる形で Set の実行時表現を記録する。
- stdlib 仕様に Set API がある場合は追記する。
  - [ ] 仕様メモ/参照の追加

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
- `docs/spec/2-3-lexer.md`
- `compiler/rust/frontend/src/parser/ast.rs`
- `compiler/rust/frontend/src/semantics/mir.rs`
- `compiler/rust/backend/llvm/src/codegen.rs`
- `runtime/native/README.md`
