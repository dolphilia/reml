# Reml AI 統合ガイド（Draft）

> `reml ai-*` コマンドや IDE 連携での AI 支援機能を安全に活用するための指針。

## 1. スコープ
- コード提案、最適化、テスト生成、ドキュメント生成。
- 対応予定コマンド：`reml ai-suggest`, `reml ai-optimize`, `reml ai-test-gen`, `reml ai-doc-gen`。

## 2. セキュリティとプライバシー
- データ送信範囲、匿名化方針（3-6, 4-6 参照）。
- オプトイン/アウト機構、監査ログ要件。

## 3. モデル互換レイヤ
- LSP/CLI からのリクエスト形式。
- ベンダー差異吸収とフォールバック戦略。

## 4. ガードレール
- 効果タグや Capability 制約と整合性のある候補のみを提示。
- `@dsl_export` と矛盾する提案のフィルタリング。

## 5. ワークフロー統合
- CLI パイプラインでの利用例（build/test/fmt との連携）。
- IDE プラグイン（4.3）での UI 仕様。
- AI 推論へ診断コンテキストを渡す際は `reml diagnose --format json` で `SerializedDiagnostic` JSON を取得し、CI 連携やログ連携で人間可読性を優先する場合は `--format text --no-snippet` を利用する。
- ストリーミング解析を含むログでは、`stream_meta.*`（`bytes_consumed`, `await_count`, `resume_count`, `backpressure_events` など）を必須フィールドとして収集し、CLI/LSP 双方で同じ RunConfig (`extensions["stream"].stats=true`) を共有する。これにより `collect-iterator-audit-metrics.py --section streaming` の閾値と AI モデルの信頼度判定が一致する。

### 5.1 LSP 診断ローカライズキー対応表
- CLI/LSP は `LocalizationKey`（`compiler/frontend/src/output/localization.rs`）を共通で利用し、`data.localization = { message_key, locale, locale_args }` を JSON に埋め込む。LSP 側でのマッピング処理は `tooling/lsp/src/handlers/diagnostics.rs` の Draft 実装を参照する。
- AI 連携機能で診断メッセージを再整形する際は、`message` の生文字列ではなく `data.localization` のキーと引数を参照し、クライアントのロケール設定に応じてテンプレートを解決する。

| `message_key` | 種別 | 既定出力先 | ロケール引数例 | 備考 |
| --- | --- | --- | --- | --- |
| `parse.expected` | Parser Recover | CLI Human/JSON, `textDocument/publishDiagnostics` | ``["fn", "identifier"]`` | `ExpectedTokenCollector` が生成。`locale_args` は期待候補を優先順に列挙する。 |
| `effects.contract.stage_mismatch` | Type/Eff | LSP `data.localization`, CLI JSON | ``["stage.beta", "stage.experimental"]`` | 効果診断 (`StageAuditPayload`) から渡され、AI ガイドは `effect.stage.*` と合わせて提示する。 |
| `cli.locale.default` | CLI ツール | CLI JSON, `window/logMessage` | ``["en-US"]`` | CLI がフォールバックロケールで実行された場合に一度だけ挿入される警告。 |

- 上表は `../../spec/3-6-core-diagnostics-audit.md` §10–11 のキー一覧から派生しており、`reports/diagnostic-format-regression.md#cli-output-note` に保存されたサンプルログでも確認できる。追加キーを定義した場合は本ガイドと README の導線を更新し、`tooling/lsp/tests/client_compat/fixtures/*.json` に `data.localization` を含む期待値を用意する。

### 5.2 Config/Data レポートの活用

- Manifest/Schema の AI 処理は `remlc config lint` / `remlc config diff` から得た JSON をそのまま入力に用いる。`examples/core_config/cli/lint.expected.json` は一例であり、`stats.validated` と `diagnostics[]` をそのままフィードするだけで Stage/KPI 差分を説明できる。
- 実行例:
  ```bash
  cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- \
    config lint \
    --manifest ../../../examples/core_config/cli/reml.toml \
    --schema ../../../examples/core_config/cli/schema.json \
    --format json | jq '{command, manifest, diagnostics}'
  ```
- 差分レビュー時は `diff.expected.json` を参照し、`change_set.items[*]`（`collections.diff.*`）と `schema_diff.changes[*]` のセットを AI 提供データに含める。`ChangeSet` 側のメタデータ（`origin`, `policy`, `stage`）を必須フィールドとし、`tooling/examples/run_examples.sh --suite core_config --update-golden` でゴールデン更新した後に整合を確認する。

### 5.3 実験的 Stage 診断の扱い
- Rust Frontend は `Diagnostic.extensions["effects"].stage` に `experimental` が含まれる診断を自動的に `warning` へ降格させる。`--ack-experimental-diagnostics` を明示すると Severity を `error` として扱い、CI や AI サービスにブロッキング信号を伝播できる。
- AI 連携ワークフローで「実験機能も強制レビュー対象」とする場合は、CLI/LSP 両チャネルから本フラグを渡し、`effects.stage.*`・`effect.capability` をリクエストペイロードへ同梱する。逆に PoC 段階の提案ではフラグを付けず、ヒント扱いの `warning` を優先して提示する。
- 例:
  ```bash
  reml_frontend --format json \
    --ack-experimental-diagnostics \
    --emit-audit-log \
    examples/core_effects/experimental_handler.reml \
    > tmp/diagnostics.experimental.json
  ```
  生成された JSON/Audit には `severity = "error"` のまま `effects.stage.trace` が保持されるため、AI クライアントは Stage 差分を解析しつつリスクスコアへ反映できる。

### 5.4 Core Diagnostics JSON の参照例
- `examples/core_diagnostics/pipeline_branch.expected.diagnostic.json` は `CliDiagnosticEnvelope` を整形したゴールデンであり、AI 連携で取り扱う最小構成の JSON を示している。`tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` で再生成し、`summary.stats.run_config.effects.type_row_mode` や `stream_meta.packrat_enabled` のような補助指標も含めて取得する。
- `effects.stage.*` / `capability.*` / `bridge.stage.*` は `diagnostics[].extensions` と `diagnostics[].audit_metadata` の双方に存在する。AI 推論時は `effects.stage.actual` と `effect.stage.required` の差分、および `audit_metadata["pipeline.node"]` を併せて解析し、どの DSL ノードで Stage 違反が発生したかを正確に説明する。
- 監査ログは NDJSON（例: `examples/core_diagnostics/pipeline_success.expected.audit.jsonl`）で保存され、`pipeline_started` と `pipeline_completed` のメタデータが 1 行ずつ付与される。`cli.run_id` と `audit_id` をキーに `diagnostics` 側と突き合わせることで、AI 連携が複数エンドポイント（CLI/LSP/監査ダッシュボード）の整合を簡単に確認できる。
- `diagnostics[].structured_hints[*]` と `diagnostics[].fixits[*]` は UI でそのまま Quick Fix を提示するための構造化データであり、`kind`（`quick_fix`/`information`）・`actions[*].kind`（`insert`/`replace`/`delete`）を AI が参照することで「自動修正を提案する／ヒントのみ表示する」判断を行える。`schema_version = "3.0.0-alpha"` の JSON では `structured_hints` が常に配列で提供される点に注意する。
- `diagnostics[].audit_metadata` は `AuditEnvelope.metadata` を JSON レベルで複製したフィールドであり、監査 NDJSON を別途読み込まなくても `pipeline.*` や `config.migration.*` の値を検索できる。AI クライアントは `audit_metadata` のみをキャッシュし、詳細調査が必要になった場合に NDJSON 側を参照する二段構成を推奨する。

## 6. Unicode 正規化ポリシー
- AI への入力は **常に** `Unicode.normalize(str, NormalizationForm::NFC)` を通過させ、識別子候補は `Unicode.prepare_identifier` を併用する。`examples/core-text/text_unicode.reml` では Bytes→Str→String 正規化、`TextBuilder`、`log_grapheme_stats` をまとめており、`expected/text_unicode.tokens.golden` を差分比較に利用できる。  
- 文字幅や Grapheme 統計を AI 提案に付与する際は `examples/core-text/expected/text_unicode.grapheme_stats.golden` 相当の JSON を埋め込み、`text.grapheme_stats.cache_hits` が 0 の場合は AI に渡す前段でキャッシュを温める。  
- ストリーミング decode の AI 前処理は `cargo run --manifest-path compiler/runtime/Cargo.toml --bin text_stream_decode -- --input <file>` を利用し、BOM/Invalid ポリシーをログ化した JSON（`text_unicode.stream_decode.golden`）を LLM へ共有する。`../../plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` §5 との整合を保ち、正規化ポリシー逸脱時は `../../plans/bootstrap-roadmap/0-4-risk-handling.md` の `R-041 Unicode Data Drift` を参照する。

## 7. 今後のタスク
- セーフティ評価・ログ収集テンプレートの整備。
- モデル更新時の検証手順。

> Draft。AI 機能実装フェーズで詳細を確定させる。
