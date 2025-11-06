# front-end dual-write 成果物

このディレクトリはフロントエンド（パーサ／型推論／診断）に関する dual-write 比較結果を配置するための共通場所である。CI とローカル検証の両方で同一レイアウトを利用し、差分確認や監査メトリクス収集を容易にする。

```
front-end/
├── ocaml/   # ベースライン（OCaml 実装）出力
├── rust/    # 候補（Rust 実装）出力
└── diff/    # 整形済み差分レポート
```

## 利用ガイド
- CI では `remlc --frontend ocaml|rust` の結果を `ocaml/`・`rust/` に保存し、`diff/` に比較結果を出力する。
- 監査スクリプト `collect-iterator-audit-metrics.py` の `--baseline` / `--candidate` 引数には、それぞれ `ocaml/`・`rust/` 配下のファイルを指定する。
- 差分調査の際は `diff/` に Markdown や JSON を保存し、必要に応じて `reports/diagnostic-format-regression.md` から参照する。

成果物をアーカイブする場合は、日付・ジョブ ID などをサブディレクトリ名に付与し、レビュー後に不要なファイルを削除すること。
