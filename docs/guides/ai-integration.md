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
- CLI/LSP は `LocalizationKey`（`compiler/rust/frontend/src/output/localization.rs`）を共通で利用し、`data.localization = { message_key, locale, locale_args }` を JSON に埋め込む。LSP 側でのマッピング処理は `tooling/lsp/src/handlers/diagnostics.rs` の Draft 実装を参照する。
- AI 連携機能で診断メッセージを再整形する際は、`message` の生文字列ではなく `data.localization` のキーと引数を参照し、クライアントのロケール設定に応じてテンプレートを解決する。

| `message_key` | 種別 | 既定出力先 | ロケール引数例 | 備考 |
| --- | --- | --- | --- | --- |
| `parse.expected` | Parser Recover | CLI Human/JSON, `textDocument/publishDiagnostics` | ``["fn", "identifier"]`` | `ExpectedTokenCollector` が生成。`locale_args` は期待候補を優先順に列挙する。 |
| `effects.contract.stage_mismatch` | Type/Eff | LSP `data.localization`, CLI JSON | ``["stage.beta", "stage.experimental"]`` | 効果診断 (`StageAuditPayload`) から渡され、AI ガイドは `effect.stage.*` と合わせて提示する。 |
| `cli.locale.default` | CLI ツール | CLI JSON, `window/logMessage` | ``["en-US"]`` | CLI がフォールバックロケールで実行された場合に一度だけ挿入される警告。 |

- 上表は `docs/spec/3-6-core-diagnostics-audit.md` §10–11 のキー一覧から派生しており、`reports/diagnostic-format-regression.md#cli-output-note` に保存されたサンプルログでも確認できる。追加キーを定義した場合は本ガイドと README の導線を更新し、`tooling/lsp/tests/client_compat/fixtures/*.json` に `data.localization` を含む期待値を用意する。

## 6. Unicode 正規化ポリシー
- AI への入力は **常に** `Unicode.normalize(str, NormalizationForm::NFC)` を通過させ、識別子候補は `Unicode.prepare_identifier` を併用する。`examples/core-text/text_unicode.reml` では Bytes→Str→String 正規化、`TextBuilder`、`log_grapheme_stats` をまとめており、`expected/text_unicode.tokens.golden` を差分比較に利用できる。  
- 文字幅や Grapheme 統計を AI 提案に付与する際は `examples/core-text/expected/text_unicode.grapheme_stats.golden` 相当の JSON を埋め込み、`text.grapheme_stats.cache_hits` が 0 の場合は AI に渡す前段でキャッシュを温める。  
- ストリーミング decode の AI 前処理は `cargo run --manifest-path compiler/rust/runtime/Cargo.toml --bin text_stream_decode -- --input <file>` を利用し、BOM/Invalid ポリシーをログ化した JSON（`text_unicode.stream_decode.golden`）を LLM へ共有する。`docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` §5 との整合を保ち、正規化ポリシー逸脱時は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の `R-041 Unicode Data Drift` を参照する。

## 7. 今後のタスク
- セーフティ評価・ログ収集テンプレートの整備。
- モデル更新時の検証手順。

> Draft。AI 機能実装フェーズで詳細を確定させる。
