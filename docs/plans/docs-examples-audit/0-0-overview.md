# 0.0 概要

## 背景
ドキュメント内の Reml コードブロックは、仕様更新や移設に伴い実装との差分が生じやすい。これを `.reml` として抽出・保守し、検証ログと 1:1 で追跡できる状態に整備する。

## 目的
- `docs/` 内の Reml コードブロックを棚卸しし、検証対象の一覧を確立する。
- 抽出した `.reml` を `examples/docs-examples/` に整理し、参照パスを統一する。
- `reml_frontend` による検証手順とログ保存先を標準化する。

## 対象範囲
- 対象: `docs/spec/` → `docs/guides/` → `docs/notes` / `docs/plans`
- 対象外: Reml 以外のコードブロック（`bash`/`json` など）

## 成功条件
- Reml コードブロックが `.reml` と 1:1 で対応し、参照パスが明示されている。
- 検証コマンドとログ保存先が定義され、監査ログと結び付けられている。
- `docs-migrations.log` に移設履歴が記録されている。

## 依存関係・前提
- `docs/spec/0-3-code-style-guide.md` のコードスタイルに準拠する。
- 既存の監査ログ運用（`reports/spec-audit/`）と整合する。

## TODO
- `docs/spec/1-1-syntax.md` の各サンプルと `.reml` の対応表を作成し、リンク先の粒度（節/脚注）を定義する。参考: `docs/spec/1-1-syntax.md`
