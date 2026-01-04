# Reml CLI サンプル集

**対象フェーズ**: Phase 1-6 開発者体験整備  
**目的**: `remlc`（Rust 版）の基本的な利用方法を手早く試せるようにする。

## 構成

| ファイル | 説明 | 主な用途 |
| --- | --- | --- |
| `add.reml` | 2 つの整数を加算する最小構成 | `--emit-ir`, `--link-runtime` の成功ケース検証 |
| `emit_suite.reml` | 条件分岐と複数関数を含むベースライン | `--emit-ast`, `--emit-tast`, `--emit-ir` の併用検証 |
| `type_error.reml` | `if` 条件に整数を渡す誤りを含むサンプル | `--format=json` やカラー診断の確認 |
| `trace_sample.reml` | 複数フェーズを通る少し長めのコード | `--trace`, `--stats` の動作確認 |

> すべてのサンプルは i64 / Bool のみを使用し、Phase 1-5 の制約（文字列・タプル未対応）に従っています。

## 使い方

リポジトリ直下で以下を実行します。

```bash
cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- examples/cli/add.reml --emit-ir
```

`--link-runtime` を指定すると `build/` 直下に実行可能ファイルが生成されます。

診断やトレース機能を確認する例:

```bash
# 型エラー診断（JSON 出力）
cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- examples/cli/type_error.reml --format=json

# トレースと統計
cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- examples/cli/trace_sample.reml --trace --stats
```

## 利用シナリオ別チェックリスト

| シナリオ | 使用ファイル | 推奨コマンド例 | 確認ポイント |
| --- | --- | --- | --- |
| 中間生成物の確認 | `emit_suite.reml` | `cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- examples/cli/emit_suite.reml --emit-ast --emit-tast --emit-ir --out-dir build/emit` | `build/emit` 配下にファイル拡張子ごとの成果物（`.ast` / `.tast` / `.ll`）が揃うこと |
| ランタイムリンク | `add.reml` | `cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- examples/cli/add.reml --link-runtime --out-dir build/bin` | 指定した `--out-dir` に実行ファイルが生成され、リンクが成功すること |
| 診断 JSON 出力 | `type_error.reml` | `cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- examples/cli/type_error.reml --format=json 2>diagnostic.json` | JSON 構造が [docs/guides/tooling/diagnostic-format.md](../../docs/guides/tooling/diagnostic-format.md) に準拠していること |
| トレースと統計 | `trace_sample.reml` | `cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- examples/cli/trace_sample.reml --trace --stats 2>trace.log` | `trace.log` にフェーズ計測と統計サマリが出力されること |
| メトリクスファイル出力（JSON） | `trace_sample.reml` | `cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- examples/cli/trace_sample.reml --metrics metrics.json` | `metrics.json` がスキーマ [`docs/schemas/remlc-metrics.schema.json`](../../docs/schemas/remlc-metrics.schema.json) に準拠すること |
| メトリクスファイル出力（CSV） | `trace_sample.reml` | `cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- examples/cli/trace_sample.reml --metrics metrics.csv --metrics-format csv` | `metrics.csv` にフェーズ別のタイミング情報が含まれること |

## 参考資料

- [`docs/plans/bootstrap-roadmap/1-6-developer-experience.md`](../../docs/plans/bootstrap-roadmap/1-6-developer-experience.md)
