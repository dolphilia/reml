# W2 AST/IR dual-write alignment

`w2-ast-alignment/` は W2 タスクで利用した dual-write 入力ケースを
ケース単位で整理した検証用データセットです。各サブディレクトリには

- `input.reml`: 実行に使用したソース
- `diagnostics.{ocaml,rust}.json`: CLI 出力から抽出した診断
- `parse-debug.{ocaml,rust}.json`: Packrat / span_trace / run_config 付きのデバッグ情報
- `ast.{ocaml,rust}.json`, `typed-ast.{ocaml,rust}.json`: AST/Typed AST の一次データ
- `dualwrite.bundle.json`: `collect-iterator-audit-metrics.py` に渡せる baseline/candidate まとめ

が含まれます。`metrics/` ディレクトリには `collect-iterator-audit-metrics.py`
の実行結果を配置してください。
