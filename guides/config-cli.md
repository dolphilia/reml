# 設定 CLI ワークフロー

> 目的：`Core.Config` API を用いた設定ファイルの検証・差分適用・テンプレート展開を CLI から運用する手順をまとめる。

## 1. コマンド一覧

| コマンド | 説明 | 主要オプション |
| --- | --- | --- |
| `reml-config validate <config.ks>` | スキーマに基づく検証を実行 | `--format json`, `--profile prod`, `--fail-on-warning` |
| `reml-config diff <old.ks> <new.ks>` | スキーマ差分の表示 | `--format table`, `--apply`, `--audit` |
| `reml-config render --template <file>` | テンプレートを具現化 | `--env prod`, `--output rendered.ks` |
| `reml-config approve <audit_id>` | レビュー済み差分を確定 | `--assign owner`, `--note <msg>` |
| `reml-config rollback <audit_id>` | 過去の差分をロールバック | `--dry-run`, `--confirm` |

## 2. 実行例

```bash
# 検証: JSON 出力からエラーのみ抽出
reml-config validate config/app.ks --profile prod --format json   | jq '.diagnostics[] | {code, message, audit_id}'

# 差分: テーブル表示でレビューし承認まで実行
reml-config diff config/base.ks config/prod.ks --format table --audit | tee diff.json
reml-config approve "$(jq -r '.audit_id' diff.json)" --assign sre-team --note "prod rollout"

# テンプレート: staging 向け設定を生成
reml-config render --template config/prod.ks --env staging --output generated/staging.ks
```

## 3. 構造化ログと監査

- `--format json` の出力は `2-5-error.md` の `Diagnostic` 拡張に準拠し、`domain` / `audit_id` / `change_set` / `severity_hint` を含む。
- `--audit` を指定すると `audit_id` を標準出力に含め、`guides/runtime-bridges.md` のホットリロード手順と連結できる。

## 4. CI/CD 連携

| ステップ | 推奨コマンド | 成果物 |
| --- | --- | --- |
| 1. 検証 | `reml-config validate` | 診断 JSON（lint レポート） |
| 2. 差分レビュー | `reml-config diff --audit` | 差分テーブル / JSON（仮 `audit_id` 付与） |
| 3. 承認確定 | `reml-config approve` | `audit_id` と責任者の確定（監査ログに記録） |
| 4. 適用 | `reml-config diff --apply --audit` | 承認済み差分を適用し最終 `audit_id` を出力 |
| 5. デプロイ | `reml-run reload` | ランタイム適用ログ |

## 5. Exit Code と制限事項

| 状態 | exit code | 備考 |
| --- | --- | --- |
| 正常終了 | 0 | 変更なし／検証成功 |
| 警告あり | 1 | `--fail-on-warning` 未指定でも警告を通知（CI で閾値調整可） |
| 検証エラー (`ValidationError`) | 2 | `ConfigError::ValidationError` を返し、`Diagnostic` を整形出力 |
| レンダリング失敗 (`RenderError`) | 3 | テンプレート内の計算・依存不足 |
| 入出力失敗 (`IoError`) | 4 | ファイルアクセス、権限不足 |

`reml-config` の JSON 出力は `Diagnostic`／`Change`／`RuntimeMetrics` と同じキー（`audit_id`, `domain`, `change_set`, `severity_hint`）を共有し、IDE・ランタイム・データ品質ガイドと一貫した監査パイプラインを構築する。
