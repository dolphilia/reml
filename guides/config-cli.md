# 設定 CLI ワークフロー（Draft）

> 目的：`Nest.Config` API を活用した設定ファイルの検証・差分適用・テンプレート処理を CLI で運用する方法を示す。

## 1. コマンド概要

| コマンド | 説明 | 主要オプション |
| --- | --- | --- |
| `kestrel-config validate <config.ks>` | スキーマに基づく検証を実行 | `--format json`, `--profile prod`, `--fail-on-warning` |
| `kestrel-config diff <old.ks> <new.ks>` | スキーマ差分の表示 | `--format table`, `--apply`, `--audit` |
| `kestrel-config render --template <file>` | テンプレートを具現化 | `--env prod`, `--output rendered.ks` |

## 2. 構造化ログとの連携

- `--format json` を指定すると、`2-5-error.md` で定義した `domain` / `audit_id` / `change_set` を含む JSON が出力される。
- CI/CD では `jq` や `yq` を用いてエラーコードや変更差分を抽出可能。

## 3. 実行例

```bash
kestrel-config validate config/app.ks --profile prod --format json   | jq '.diagnostics[] | {code, message, audit_id}'

kestrel-config diff config/base.ks config/prod.ks --format table

kestrel-config render --template config/prod.ks --env staging --output generated/staging.ks
```

## 4. 監査とロールバック

- `--audit` を付与すると差分適用結果に `audit_id` が付与され、ランタイムガイド（`guides/runtime-bridges.md`）と統合できる。
- ロールバックは `Config.apply_diff` で逆差分を適用し、`audit.log("config.rollback", diff)` と組み合わせる。

> ガイド内容はフェーズ3で詳細化予定。