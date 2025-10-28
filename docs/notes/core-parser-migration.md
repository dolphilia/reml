# Core Parser Migration メモ

## TODO: RunConfig フラグの未実装点
- [ ] `--packrat` / `RunConfig.packrat` は警告のみで実装待ち。`PARSER-003` のメモ化シム導入後に CLI/LSP 両方で挙動を検証する。
- [ ] `--left-recursion=<mode>` の `on/auto` はシムが未着手のため警告を伴う。左再帰テーブル構築手順を `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md` と同期する。
- [ ] LSP 側の設定ファイル読み込み処理はドラフト段階。`tooling/lsp/config/default.json` を更新した場合は CLI 側の `extensions["config"]` と突合し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にフォローアップを残す。

## 参考リンク
- `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`
- `docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md`
- `docs/plans/bootstrap-roadmap/2-5-review-log.md`
