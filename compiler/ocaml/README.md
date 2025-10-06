# compiler/ocaml ワークスペース（Phase 1）

Phase 1 ブートストラップ計画に基づき、OCaml 製 Reml コンパイラを構築するための作業領域です。対応するタスクは主に [`docs/plans/bootstrap-roadmap/1-x`](../../docs/plans/bootstrap-roadmap/) に定義されています。

## ディレクトリ
- `src/`: コンパイラ本体（パーサー、型推論、Core IR、LLVM 出力など）
- `tests/`: ゴールデン AST・型推論スナップショット・IR 検証などのテストコード
- `docs/`: 実装メモ、設計ノート、調査結果

## ビルド方法

### 前提条件
- OCaml >= 4.14
- Dune >= 3.0
- Menhir >= 20201216

### ビルド手順

```bash
# プロジェクトルート (compiler/ocaml) で実行
dune build

# 実行可能ファイルのパス
./_build/default/src/remlc.exe
```

### 使用方法

```bash
# AST を出力
dune exec -- remlc --emit-ast <input.reml>

# 例
dune exec -- remlc --emit-ast ../../examples/language-impl-comparison/reml/pl0_combinator.reml
```

## 現在の実装状況 (M1 マイルストーン)

### ✅ 完了
- [x] AST 定義 (`src/ast.ml`)
- [x] トークン定義 (`src/token.ml`)
- [x] Lexer 実装 (`src/lexer.mll`)
  - Unicode XID 準拠識別子 (Phase 1: ASCII のみ)
  - 整数・浮動小数・文字・文字列リテラル
  - コメント処理 (行コメント、入れ子ブロックコメント)
  - エスケープシーケンス
- [x] Parser 実装 (`src/parser.mly`)
  - 基本的な式・宣言の構文解析
  - 演算子優先順位 (Menhir %left/%right)
  - Span 情報付与
- [x] Dune ビルドシステム
- [x] CLI エントリポイント (`src/main.ml`)

### 🚧 進行中
- [ ] エラー回復戦略の実装
- [ ] 診断モデル ([2-5-error.md](../../docs/spec/2-5-error.md) 準拠)
- [ ] Golden AST テスト整備
- [ ] 完全な構文要素のカバレッジ (match, while, for, 等)
- [ ] パターンマッチの完全実装

### 📝 TODO (Phase 1 後半)
- [ ] `1-2-typer-implementation.md` で求められる Typed AST/型推論テストのひな型作成
- [ ] `1-3`〜`1-5` のタスクに合わせた Core IR/LLVM/ランタイム連携の stub 追加
- [ ] 計測フック（`1-6`, `1-7`）の連携手順を記録

## 技術詳細

設計ドキュメント: [docs/parser_design.md](docs/parser_design.md)

### AST 設計
- すべてのノードに `span: { start: int; end_: int }` を付与
- バイトオフセットで位置を記録 (行・列番号は診断時に計算)
- 仕様書 [1-1-syntax.md](../../docs/spec/1-1-syntax.md) に準拠

### 演算子優先順位
仕様書 §D.1 の固定優先順位表をMenhirの %left/%right で実装:
- 最高優先: 後置演算子 (関数呼び出し、フィールドアクセス、`?`)
- 最低優先: パイプ `|>` (左結合)

### Unicode 対応
- Phase 1: ASCII 識別子のみサポート (`[a-zA-Z_][a-zA-Z0-9_]*`)
- Phase 2 以降: Unicode XID 完全対応予定
