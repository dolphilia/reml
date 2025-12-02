# Core Diagnostics サンプル

`examples/core_diagnostics/` には Chapter 3.6 の `Diagnostic`/`AuditEnvelope` 実装を検証する最小サンプルを配置している。`pipeline_success.reml` は診断 0 件の成功パス、`pipeline_branch.reml` は複数経路を含むが最終的に成功するパスであり、どちらも CLI の `CliDiagnosticEnvelope` と監査イベントの流れを確認する目的で利用する。

## 実行とゴールデン更新

まず `compiler/rust/frontend` がビルドできる状態で、以下のスクリプトを利用してサンプルを実行する。

```bash
tooling/examples/run_examples.sh --suite core_diagnostics --with-audit
```

診断・監査ログのゴールデン (`*.expected.diagnostic.json` / `*.expected.audit.jsonl`) を更新する場合は `--update-golden` を付与する。`--update-golden` は自動的に `--with-audit` を有効化し、標準出力（診断）と標準エラー（監査）を解析して整形済み JSON と NDJSON を `examples/core_diagnostics/` 配下へ書き戻す。

```bash
tooling/examples/run_examples.sh --suite core_diagnostics --update-golden
```

生成物は以下の規約で保存する。

- `pipeline_success.expected.diagnostic.json`: `CliDiagnosticEnvelope` の JSON を `python -m json.tool` equivalent で整形したもの。
- `pipeline_success.expected.audit.jsonl`: `AuditEmitter` が出力した NDJSON (`pipeline_started` / `pipeline_completed`)。
- `pipeline_branch.*`: 同じ形式で分岐サンプルの結果を保存。

CI やドキュメントはこれらのファイルを参照し、`docs/spec/3-6-core-diagnostics-audit.md` §9 のサンプルとも連動している。
