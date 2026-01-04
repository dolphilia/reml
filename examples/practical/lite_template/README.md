# Lite テンプレート回帰サンプル

学習/試作向けの最小構成テンプレートを生成する。CLI ヘルプには用途と `project.stage` 昇格の導線を含める。

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
