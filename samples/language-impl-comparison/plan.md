# 言語実装比較サンプル：未実装ファイルと実装タスク計画

## 📊 現状サマリー

**実装済み**: 
- Reml: 15ファイル（完全）
- 比較言語: 基本3種（mini_lisp, json_parser, pl0）中心に44ファイル実装済み
- parser-combinator: megaparsec 3個、nom 3個
- parser-generator: ANTLR PL/0のみ

**未実装合計**: **約126個**
- 各言語ディレクトリ: 96個
- parser-combinator (megaparsec + nom): 16個
- parser-generator (ANTLR): 14個

---

## 🎯 実装タスクの優先度別分類

### 【最優先】フェーズ1: コア3言語の補完（24ファイル）✅ 完了
比較の土台となる3種を全言語で揃える

1. **markdown_parser 実装** (8言語: elixir, elm, fsharp, koka, nim, ocaml, scala3, swift) ✅ 完了
2. **pl0 実装** (2言語: raku, ocaml5) ✅ 完了
3. **json_parser 実装** (1言語: ocaml5) ✅ 完了

### 【高優先】フェーズ2: 効果システム比較（11ファイル）✅ 完了
Remlの代数的効果と他言語の効果ハンドリングを比較

4. **algebraic_effects 実装** (10言語: elixir, elm, fsharp, haskell, nim, ocaml, ocaml5, raku, rust, scala3, swift) ✅ 完了
   - kokaは実装済み（効果システム言語として重要）

### 【中優先】フェーズ3: 高度なパーサー実装（32ファイル）
複雑な構文解析での記述性を比較

5. **sql_parser 実装** (全12言語: 各1ファイル) ✅ 完了
6. **yaml_parser 実装** (全12言語: 各1ファイル) ✅ 完了
7. **toml_parser 実装** (全12言語: 各1ファイル)

### 【通常】フェーズ4: 応用実装（29ファイル）

8. **regex_engine 実装** (全12言語)
9. **template_engine 実装** (全12言語)
10. **json_extended 実装** (全12言語)

### 【補完】フェーズ5: パーサーライブラリ比較（30ファイル）

11. **megaparsec拡張** (8ファイル: markdown以降の8種)
12. **nom拡張** (8ファイル: markdown以降の8種)
13. **ANTLR文法定義** (7セット14ファイル: MiniLisp, JSON, Markdown, SQL, YAML, TOML, Template)

---

## 📋 実装スムーズ化のための推奨アプローチ

### ステップ1: テンプレート準備
- Reml実装を各言語の慣用句に翻訳するガイドライン作成
- 言語別ボイラープレート（パッケージ宣言、import、型定義）準備

### ステップ2: 並行実装戦略
- **横展開**: 1つの小規模言語を全言語で実装（例: markdown_parserを8言語で）
- **縦展開**: 1つの言語で複数の小規模言語を実装（例: haskellで残り7種）

### ステップ3: 品質保証
- 各実装にサンプル入力と期待出力を付与
- reml-improvement-matrix.mdを更新し、発見した知見を記録

### ステップ4: ドキュメント更新
- 各言語のREADME.mdに実装状況と特徴的な記述を追記
- 比較分析レポートの作成（実装後）

---

## 🔧 技術的考慮事項

- **言語特性の尊重**: 各言語の慣用的な書き方を優先（例: Elixirはパイプライン、Haskellはモナド）
- **依存管理の省略**: ドキュメント用途のため、単一ファイル実装を基本とする
- **エラーハンドリング**: 各言語の標準的なエラー型を使用（Result/Either/Maybe等）
- **Unicode対応**: 必要に応じて各言語のUnicodeライブラリを参照

---

提案: まずフェーズ1の24ファイルから着手し、基本3種の完全なクロス言語比較を確立することをお勧めします。