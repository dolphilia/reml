# 設定 CLI ワークフロー（Draft）

> 目的：`Nest.Config` API を用いた設定ファイルの検証・差分適用・テンプレート展開を CLI から運用する手順をまとめる。

## 1. コマンド一覧

| コマンド | 説明 | 主要オプション |
| --- | --- | --- |
| `kestrel-config validate <config.ks>` | スキーマに基づく検証を実行 | `--format json`, `--profile prod`, `--fail-on-warning` |
| `kestrel-config diff <old.ks> <new.ks>` | スキーマ差分の表示 | `--format table`, `--apply`, `--audit` |
| `kestrel-config render --template <file>` | テンプレートを具現化 | `--env prod`, `--output rendered.ks` |
| `kestrel-config rollback <audit_id>` | 過去の差分をロールバック | `--dry-run`, `--confirm` |

## 2. 実行例

```bash
# 検証: JSON 出力からエラーのみ抽出
kestrel-config validate config/app.ks --profile prod --format json   | jq '.diagnostics[] | {code, message, audit_id}'

# 差分: テーブル表示でレビュー
kestrel-config diff config/base.ks config/prod.ks --format table

# テンプレート: staging 向け設定を生成
kestrel-config render --template config/prod.ks --env staging --output generated/staging.ks
```

## 3. 構造化ログと監査

- `--format json` の出力は `2-5-error.md` の `Diagnostic` 拡張に準拠し、`domain` / `audit_id` / `change_set` を含む。
- `--audit` を指定すると `audit_id` を標準出力に含め、`guides/runtime-bridges.md` のホットリロード手順と連結できる。

## 4. CI/CD 連携

| ステップ | 推奨コマンド | 成果物 |
| --- | --- | --- |
| 1. 検証 | `kestrel-config validate` | 診断 JSON（lint レポート） |
| 2. 差分レビュー | `kestrel-config diff` | 差分テーブル / JSON |
| 3. 承認後適用 | `kestrel-config diff --apply --audit` | `audit_id` 付き差分 JSON |
| 4. デプロイ | `kestrel-run reload` | ランタイム適用ログ |

## 5. TODO / 制限事項

- `Config.render` の戻り値仕様は Draft。最終的な戻り値/エラー型を確定予定。
- マージ戦略のカスタム優先順位（例: プロファイルごとの重み付け）は未定義。
- CLI の exit code ポリシー（警告発生時に非ゼロを返すか等）は要検討。

> 本ガイドはフェーズ3でさらに事例を追加する予定です。
