# Core Config サンプル

`examples/core_config` は `reml.toml` の DSL セクションと `@dsl_export` が出力する `DslExportSignature` を同期させる最小ケースです。`dsl/` ディレクトリには `audit_bridge.reml` と `telemetry_bridge.reml` の 2 つの DSL エントリがあり、`dsl.audit_bridge` は Capability/Stage 情報をすべて Manifest に投影した例になっています。

## 使い方

1. JSON ダンプでステージと Capability を確認:
   ```bash
   cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- \
     manifest dump --manifest examples/core_config/reml.toml
   ```
   `dsl.audit_bridge.exports[*].signature.stage_bounds` や `capabilities` が `@dsl_export` のメタデータと一致していることを確認できます。
2. マニフェスト API のテストを実行 (`toml v0.5.11` の checksum 問題で失敗する場合は `compiler/runtime/Cargo.lock` の再取得が必要です):
   ```bash
   cargo test manifest --test manifest --manifest-path compiler/runtime/Cargo.toml
   ```
   `update_dsl_signature_records_stage_bounds` が `expect_effects_stage` を `stage_bounds.current` に投影していることを検証します。

## ファイル一覧

| ファイル | 説明 |
| --- | --- |
| `reml.toml` | Manifest 本体。`dsl.audit_bridge` が Capability/Stage/Effect の同期例。 |
| `dsl/audit_bridge.reml` | `@dsl_export` で Stage 境界と Capability を宣言する Bridge スケッチ。 |
| `dsl/telemetry_bridge.reml` | 最小限の DSL エクスポート。`manifest dump` で未同期項目の差分を確認するために残しています。 |

## CLI ゴールデン

`examples/core_config/cli/` には `remlc config lint` / `remlc config diff` の JSON 出力例を
保存しています。`tooling/examples/run_examples.sh --suite core_config --update-golden` を
実行すると `lint.expected.json` / `diff.expected.json` が再生成され、Config/Data CLI
のフォーマット変更をレビューできるようになっています。
