# Core Config CLI サンプル

`examples/core_config/cli` は `remlc config lint` / `remlc config diff` コマンドの
出力ゴールデンを管理する最小構成のセットです。`reml.toml` と `schema.json` を
組み合わせて `Core.Config` の検証フローを再現し、`config_old.json` と
`config_new.json` で ChangeSet 出力のサンプルを保持します。

## 使い方

```bash
# 人間向けテキストで確認
tooling/examples/run_examples.sh --suite core_config

# ゴールデン更新（JSON 出力を lint/diff.expected.json へ保存）
tooling/examples/run_examples.sh --suite core_config --update-golden
```

## ファイル構成

| パス | 説明 |
| --- | --- |
| `reml.toml` | Config CLI 検証用 Manifest。`dsl.config_cli` エントリが存在します。 |
| `schema.json` | `Schema.version` を含む JSON 形式の Core.Data スキーマ。 |
| `config_old.json` / `config_new.json` | `config diff` の差分元/先スナップショット。 |
| `lint.expected.json` | `remlc config lint` `--format json` のゴールデン。 |
| `diff.expected.json` | `remlc config diff` `--format json` のゴールデン。 |
| `dsl/sample.reml` | Manifest から参照されるダミー DSL ファイル。 |

`README.md` 以外のファイルは `tooling/examples/run_examples.sh --suite core_config` を
実行すると自動的に検証され、`--update-golden` で JSON 出力が更新されます。
