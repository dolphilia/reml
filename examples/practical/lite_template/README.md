# Lite テンプレート回帰サンプル

Lite テンプレートの最小構成を回帰資産として固定するためのサンプルです。
`templates/sample.input` と `expected/lite_template/sample.ast.expected` の対応を基準とします。

## 目的
- Lite テンプレートの最短実行フロー（入力→AST）を固定する
- 監査ログ省略でも `Diagnostic` が出力される前提を確認する

## 実行メモ
- `reml run` で `templates/sample.input` を入力し、AST 出力を `sample.ast.expected` と比較する
- `templates/sample.invalid` は `sample.invalid.diagnostic.json` を期待値とする

## 入力取り扱い
- Lite 既定では IO Capability を要求しないため、`src/main.reml` は入力を文字列リテラルで保持する
- ファイル入力に拡張する場合は `Core.IO` を利用し、`io.fs.read` の Capability を宣言する
- `src/main_io.reml` はファイル入力版のサンプルとして利用する
