# SchemaDiff サンプル（Run ID: 20251203-schema-core-data）

- 目的: `compiler/rust/runtime/src/data/schema.rs` で実装した `SchemaDiff` の JSON 形式を確認する
- 実行予定コマンド: `cargo run --manifest-path compiler/rust/runtime/Cargo.toml --example schema_diff_demo`
- 備考: 現状は `toml v0.5.11` のチェックサム検証で Cargo が停止するため、上記コマンドは未実行。`docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md` §3.1 に記載したとおり、チェックサム問題が解消され次第このサンプルを再取得する。

## 想定される差分 JSON

以下は `schema_diff_demo.rs` の入出力仕様を元に `SchemaDiff` をシリアライズした例。`added`/`removed`/`changed` の 3 つの配列を持ち、変更済みフィールドには `attributes` が付与される。

```json
{
  "added": [
    {
      "name": "timeout_ms",
      "data_type": {
        "kind": "integer"
      },
      "required": false,
      "description": null,
      "default_value": 1500,
      "examples": [],
      "rules": [],
      "metadata": {}
    }
  ],
  "removed": [],
  "changed": [
    {
      "name": "endpoint",
      "previous": {
        "name": "endpoint",
        "data_type": {
          "kind": "string"
        },
        "required": true,
        "description": "Config service endpoint",
        "default_value": null,
        "examples": [],
        "rules": [],
        "metadata": {}
      },
      "current": {
        "name": "endpoint",
        "data_type": {
          "kind": "string"
        },
        "required": true,
        "description": "Config service endpoint (https only)",
        "default_value": null,
        "examples": [],
        "rules": [
          {
            "id": "config.endpoint.scheme",
            "kind": {
              "rule": "regex",
              "pattern": "^https://"
            },
            "severity": "error",
            "message": "https:// で始まる URL を指定してください",
            "params": {}
          }
        ],
        "metadata": {}
      },
      "attributes": [
        "description",
        "rules"
      ]
    },
    {
      "name": "retries",
      "previous": {
        "name": "retries",
        "data_type": {
          "kind": "integer"
        },
        "required": true,
        "description": null,
        "default_value": 3,
        "examples": [],
        "rules": [],
        "metadata": {}
      },
      "current": {
        "name": "retries",
        "data_type": {
          "kind": "integer"
        },
        "required": true,
        "description": null,
        "default_value": 5,
        "examples": [],
        "rules": [],
        "metadata": {}
      },
      "attributes": [
        "default_value"
      ]
    }
  ]
}
```

## フォローアップ

1. Cargo のチェックサム問題を解消し、`schema_diff_demo` の実行ログを本ファイルに再掲載する。
2. `tooling/scripts/generate-schema-diff.sh` を追加し、CI で差分サンプルを再生成できるようにする（次フェーズの TODO）。
