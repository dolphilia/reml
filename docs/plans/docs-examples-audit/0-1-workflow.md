# 0.1 抽出・運用ワークフロー

## 1. 棚卸し
- `docs/` 内の ```reml コードブロックを抽出し、出典（ファイル/節）と対応する `.reml` を記録する。
- 出力は `docs/plans/docs-examples-audit/` 配下で管理する。
- `docs/spec/` の棚卸し表は `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` を正本とする。

### 棚卸しテンプレート
```text
| 優先度 | ドキュメント | 節 | コード名 | .reml パス | 状態 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| P0 | docs/spec/1-1-syntax.md | §B.1 | use_nested | examples/docs-examples/spec/1-1-syntax/use_nested.reml | ok | 正準サンプル |
```

## 2. 抽出ルール
- 配置先は `examples/docs-examples/<kind>/<doc-dir>/`。
- `<kind>` は `spec` / `guides` / `notes` / `plans`。
- `<doc-dir>` は `docs/` からの相対パスを採用し、ファイル名は `snake_case` とする。
- 同一節に複数例がある場合は `-a`, `-b` のサフィックスを付ける。

### 例
- `docs/spec/1-1-syntax.md` §B.1 → `examples/docs-examples/spec/1-1-syntax/use_nested.reml`

## 3. 参照更新
- 本文・脚注の参照を `.reml` へ付け替える。
- `docs/spec/0-0-overview.md` / `docs/spec/0-3-code-style-guide.md` のポリシー記述も更新する。

## 4. 監査ログ更新
- 実行ログの保存先は `reports/spec-audit/` を基本とする。
- 変更履歴は `docs-migrations.log` に記録する。

## TODO
- 例外命名（旧来サンプル・退避サンプル）を定義する。参考: `docs/spec/0-3-code-style-guide.md`
