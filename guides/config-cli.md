# 設定 CLI ワークフロー（Draft）

> 目的：`Core.Config` API を用いた設定ファイルの検証・差分適用・テンプレート展開を CLI から運用する手順をまとめる。

## 1. コマンド一覧

| コマンド | 説明 | 主要オプション |
| --- | --- | --- |
| `reml-config validate <config.ks>` | スキーマに基づく検証を実行 | `--format json`, `--profile prod`, `--fail-on-warning` |
| `reml-config diff <old.ks> <new.ks>` | スキーマ差分の表示 | `--format table`, `--apply`, `--audit` |
| `reml-config render --template <file>` | テンプレートを具現化 | `--env prod`, `--output rendered.ks` |
| `reml-config rollback <audit_id>` | 過去の差分をロールバック | `--dry-run`, `--confirm` |

## 2. 実行例

```bash
# 検証: JSON 出力からエラーのみ抽出
reml-config validate config/app.ks --profile prod --format json   | jq '.diagnostics[] | {code, message, audit_id}'

# 差分: テーブル表示でレビュー
reml-config diff config/base.ks config/prod.ks --format table

# テンプレート: staging 向け設定を生成
reml-config render --template config/prod.ks --env staging --output generated/staging.ks
```

## 3. 構造化ログと監査

- `--format json` の出力は `2-5-error.md` の `Diagnostic` 拡張に準拠し、`domain` / `audit_id` / `change_set` を含む。
- `--audit` を指定すると `audit_id` を標準出力に含め、`guides/runtime-bridges.md` のホットリロード手順と連結できる。

## 4. CI/CD 連携

| ステップ | 推奨コマンド | 成果物 |
| --- | --- | --- |
| 1. 検証 | `reml-config validate` | 診断 JSON（lint レポート） |
| 2. 差分レビュー | `reml-config diff` | 差分テーブル / JSON |
| 3. 承認後適用 | `reml-config diff --apply --audit` | `audit_id` 付き差分 JSON |
| 4. デプロイ | `reml-run reload` | ランタイム適用ログ |

## 5. Exit Code と制限事項

| 状態 | exit code | 備考 |
| --- | --- | --- |
| 正常終了 | 0 | 変更なし／検証成功 |
| 警告あり | 1 | `--fail-on-warning` 未指定でも警告を通知（CI で閾値調整可） |
| 検証エラー (`ValidationError`) | 2 | `ConfigError::ValidationError` を返し、`Diagnostic` を整形出力 |
| レンダリング失敗 (`RenderError`) | 3 | テンプレート内の計算・依存不足 |
| 入出力失敗 (`IoError`) | 4 | ファイルアクセス、権限不足 |

- マージ戦略のカスタム優先順位（例: プロファイルごとの重み付け）は今後の検討課題。

> 本ガイドはフェーズ3でさらに事例を追加する予定です。
