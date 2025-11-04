# ストリーミング指標ダッシュボード

Reml のストリーミング実装に関する KPI を集約する。`parser.stream.outcome_consistency` は `run_stream` と `run` の結果（AST・診断・`stream_meta`）が一致する割合であり、1.0 未満の場合は `ContinuationMeta.resume_lineage` を添付した調査ログを必ず残す。

## parser.stream.outcome_consistency

| プラットフォーム | pass_rate | 計測ログ | 補足 |
|------------------|-----------|----------|------|
| linux-x86_64 | 1.0 | `dune runtest compiler/ocaml/tests/streaming_runner_tests.ml` → `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | `resume_lineage = ["pending.backpressure"]` を維持したまま成功 |
| macos-arm64 | 1.0 | `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --platform macos-arm64 --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | CI で `reports/audit/streaming-macos.json` として保存 |
| windows-msvc | 1.0 | `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --platform windows-msvc --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | `reports/audit/streaming-windows.json` を参照 |

`pass_rate < 1.0` を検出した場合の対応：

1. `ContinuationMeta.resume_lineage` を `reports/audit/phase2-7/` 配下に保存する。
2. `stream_meta` の `bytes_consumed` / `memo_bytes` と差分理由を上記ディレクトリへ追記する。
3. 収集結果を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の履歴へ転記し、フォローアップチケットを登録する。

## 収集手順

1. `dune runtest compiler/ocaml/tests/streaming_runner_tests.ml` を実行し、ストリーミング結果とバッチ結果の差分を確認する。
2. 診断 JSON を `tooling/ci/collect-iterator-audit-metrics.py --section streaming --source <json>` へ入力し、`parser.stream.outcome_consistency` を抽出する。
3. `--require-success` を指定して CI ゲートを有効化し、失敗時は `failures[*].resume_lineage` を確認して原因を共有する。
4. 成果を本ダッシュボードへ転記し、Linux/Windows/macOS それぞれの pass_rate を更新する。

## parser.stream.backpressure_sync

| プラットフォーム | pass_rate | 計測ログ | 補足 |
|------------------|-----------|----------|------|
| linux-x86_64 | 1.0 | `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | `resume_hint.reason` と `stream_meta.last_reason` がいずれも `pending.backpressure` を指すケースのみを計上 |
| macos-arm64 | 1.0 | `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --platform macos-arm64 --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | `resume_hint.reason` と `stream_meta.last_reason` が一致 |
| windows-msvc | 1.0 | `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --platform windows-msvc --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | Windows ランナーでも Backpressure 同期を確認 |

Auto モードの Pending 記録で Backpressure 理由が欠落した場合、`failures[*]` に `resume_reason` / `stream_reason` が記録される。逸脱を検出した際は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#stream-poc-backpressure` を更新する。

## parser.stream.flow.auto_coverage

| プラットフォーム | pass_rate | 計測ログ | 補足 |
|------------------|-----------|----------|------|
| linux-x86_64 | 1.0 | `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | `RunConfig.extensions["stream"].flow.policy = "auto"` のサンプルのみで構成 |
| macos-arm64 | 1.0 | `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --platform macos-arm64 --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | CI Gate で Auto ポリシーの有効化を検証 |
| windows-msvc | 1.0 | `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --platform windows-msvc --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | Windows でも Auto カバレッジ 100% を保証 |

## parser.stream.demandhint_coverage

| プラットフォーム | pass_rate | 計測ログ | 補足 |
|------------------|-----------|----------|------|
| linux-x86_64 | 1.0 | `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | `continuation_meta.resume_hint.{min_bytes,preferred_bytes}` が非 null であることを確認 |
| macos-arm64 | 1.0 | `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --platform macos-arm64 --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | `resume_hint.reason` が `pending.backpressure` であることを維持 |
| windows-msvc | 1.0 | `tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success --platform windows-msvc --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` | DemandHint 欠落検知時は `STREAM-POC-DEMANDHINT` を再オープン |

## 補足

- `collect-iterator-audit-metrics.py` の `collect_streaming_metrics` は `baseline` ブロックを参照するため、ストリーミング診断 JSON には `baseline.parse_result` / `baseline.stream_meta` / `baseline.diagnostics` を含めること。
- FlowController Auto に関連する KPI（`parser.stream.backpressure_sync`, `parser.stream.flow.auto_coverage`）は `RunConfig.extensions["stream"].flow` と `stream_meta` の両方を参照する。CLI/LSP で FlowController パラメータを更新した場合は、必ず診断ゴールデンと監査ログを同時に更新する。
- `scripts/validate-diagnostic-json.sh --suite streaming` で `audit_events` 内の `parser.stream.pending` / `parser.stream.error` ペイロードを検証するため、ゴールデン更新時は同スイートを必ず実行する。
