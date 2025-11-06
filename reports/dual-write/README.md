# dual-write 成果物保管ディレクトリ

Rust フロントエンド移行では、OCaml 実装との dual-write 比較結果を共有ディレクトリへ集約する。CI から出力される成果物はサブディレクトリごとに用途が分かれており、手動検証時も同じ構成を用いる。

## ディレクトリ構成
- `front-end/`: パーサ／型推論／診断フローで生成される比較結果を格納
  - `ocaml/`: OCaml 実装のベースライン JSON やログ
  - `rust/`: Rust 実装の候補出力
  - `diff/`: `jq --sort-keys` 等で正規化した差分レポート

## 運用メモ
- CI ジョブは成果物を上書き保存するため、過去ログを残す場合は別ブランチまたは `reports/dual-write/archive/`（将来追加予定）へ移動する。
- 手動検証で生成した成果物は、レビュー完了後にクリーンアップする。必要に応じて `docs/notes/` 配下へ調査ノートを残す。
- 詳細な比較手順は `docs/plans/rust-migration/1-0-front-end-transition.md` および `docs/plans/rust-migration/1-2-diagnostic-compatibility.md` を参照。
