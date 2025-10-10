# Reml CLI サンプル集

**対象フェーズ**: Phase 1-6 開発者体験整備  
**目的**: `remlc`（OCaml ブートストラップ版）の基本的な利用方法を手早く試せるようにする。

## 構成

| ファイル | 説明 | 主な用途 |
| --- | --- | --- |
| `add.reml` | 2 つの整数を加算する最小構成 | `--emit-ir`, `--link-runtime` の成功ケース検証 |
| `type_error.reml` | `if` 条件に整数を渡す誤りを含むサンプル | `--format=json` やカラー診断の確認 |
| `trace_sample.reml` | 複数フェーズを通る少し長めのコード | `--trace`, `--stats` の動作確認 |

> すべてのサンプルは i64 / Bool のみを使用し、Phase 1-5 の制約（文字列・タプル未対応）に従っています。

## 使い方

リポジトリ直下で以下を実行します。

```bash
opam exec -- dune exec -- remlc examples/cli/add.reml --emit-ir
```

`--link-runtime` を指定すると `build/` 直下に実行可能ファイルが生成されます。

診断やトレース機能を確認する例:

```bash
# 型エラー診断（JSON 出力）
opam exec -- dune exec -- remlc examples/cli/type_error.reml --format=json

# トレースと統計
opam exec -- dune exec -- remlc examples/cli/trace_sample.reml --trace --stats
```

各オプションの詳細や推奨ワークフローは [`docs/guides/cli-workflow.md`](../../docs/guides/cli-workflow.md) を参照してください。

## 参考資料

- [`docs/guides/cli-workflow.md`](../../docs/guides/cli-workflow.md)
- [`docs/guides/trace-output.md`](../../docs/guides/trace-output.md)
- [`docs/plans/bootstrap-roadmap/1-6-developer-experience.md`](../../docs/plans/bootstrap-roadmap/1-6-developer-experience.md)
