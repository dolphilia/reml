# 診断フォーマット差分レビュー手順

## 目的
- `diagnostic_serialization` の変更や CLI/LSP 出力の更新時に、旧フォーマットからの差分を可視化しレビューを容易にする。
- CI の `diagnostic-json` ジョブで検知したエラーの調査手順を明文化する。

## 1. ローカル検証手順
1. `npm ci --prefix tooling/lsp/tests/client_compat`
2. `npm run ci --prefix tooling/lsp/tests/client_compat`
3. `bash scripts/validate-diagnostic-json.sh tmp/diagnostics-output/`
4. 差分が発生した場合、`tooling/lsp/tests/client_compat/fixtures/` および `compiler/ocaml/tests/golden/diagnostics/` を比較し、意図した変更か確認する。
5. RunConfig 切替シナリオを記録する場合は `tmp/diagnostics-output/runconfig/` を作成し、`remlc --require-eof examples/cli/add.reml --format json --emit-ast --packrat --left-recursion=auto` など CLI フラグを組み合わせて出力を保存する。`scripts/validate-diagnostic-json.sh tmp/diagnostics-output/runconfig` を実行し、`extensions.config.*` に CLI の設定値が反映されていることを確認する。

## 2. 差分レポートのまとめ方
- 変更前後の JSON を `jq --sort-keys` で整形し、`diff -u` で比較する。
- `effects.*` や `bridge.*` など拡張キーの追加は、対応する仕様書 (`docs/spec/3-6-core-diagnostics-audit.md` 等) を参照して説明を添える。とくに `effect.required_capabilities` / `effect.actual_capabilities` / `effect.stage.required_capabilities` / `effect.stage.actual_capabilities` の配列化は Phase 2-5 EFFECT-003 の成果物として扱い、レビュー時に配列内容と `capabilities_detail` の同期を確認する。
- 効果行統合 (`TYPE-002`): `effect.type_row.{declared,residual,canonical}` が CLI/LSP 診断および監査ログ (`audit.metadata`) の両方に含まれることを確認し、`collect-iterator-audit-metrics.py --section effects --require-success` の結果と一致するかレビューする。互換モードで `metadata-only` を使用した場合は差分メモに理由を追記する。
- SerializedDiagnostic 基盤へ移行した CLI では、空配列しか持たない `effects.*` / `effect.*` 拡張が JSON から省略されるため、差分確認時は「値が消えた理由が空集合の正規化によるものか」「本当に情報欠落なのか」を切り分ける。空集合による省略は仕様どおりであり、`typeclass.iterator.stage_mismatch` のように実データがあるケースでは従来どおり配列が残る。
- 効果構文 PoC (`effect_syntax.*`) の出力を更新する場合は、`metrics.syntax.effect_construct_acceptance` / `metrics.effects.syntax_poison_rate` の値を確認する。Phase 2-5 では 0.0 / 1.0 を基準値として記録し、値が変化した場合は理由をレビューで説明する。サンプルは `compiler/ocaml/tests/golden/diagnostics/effects/effect-syntax-poc.json.golden`（新設予定）と `tooling/lsp/tests/client_compat/fixtures/effect-syntax-poc.json` に保存し、`tooling/ci/collect-iterator-audit-metrics.py --section effects` の出力と照合する。
- `extensions.typeclass.dictionary.*` / `typeclass.dictionary.*` に変更が生じた場合は、辞書監査ゴールデン `compiler/ocaml/tests/golden/typeclass_dictionary_resolved.json.golden` を更新し、`typeclass.dictionary_pass_rate` のトラッキングに差分内容を反映する。
- `extensions["recover"]` を含む診断の差分を扱う場合は、`compiler/ocaml/tests/golden/diagnostics/parser/recover-missing-semicolon.json.golden`・`.../recover-unclosed-block.json.golden`・`tooling/lsp/tests/client_compat/fixtures/diagnostic-recover.json` を合わせて確認し、`sync_tokens`/`hits`/`strategy`/`has_fixits`/`notes` が揃っているかチェックする。`parser_recover_tests.ml` と `scripts/validate-diagnostic-json.sh` の `recover` バリデータで再現できるかも確認する。
- ストリーミング監査 (`parser.stream.pending` / `parser.stream.error`) を更新した場合は、`compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` と `reports/audit/dashboard/streaming.md` を同時に見直し、`scripts/validate-diagnostic-json.sh --suite streaming` の必須項目（`resume_hint` / `last_reason` / `expected_tokens` / `last_checkpoint` / `stream_meta.*`）が満たされているかを確認する。`tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-streaming-meta.json` も合わせて更新し、LSP 側の `data.stream_meta.*` が CLI と一致することをレビューする。`tooling/ci/collect-iterator-audit-metrics.py --section streaming --require-success` の `parser.stream.demandhint_coverage` / `parser.stream.backpressure_sync` も再実行し、逸脱時は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#stream-poc-demandhint` を更新する。
- 値制限診断（`type_inference.value_restriction_violation` / `type_inference.value_restriction_legacy_usage`）が追加された場合は、テンプレートゴールデン `compiler/ocaml/tests/golden/type_inference_value_restriction.{strict,legacy}.json.golden` をベースに Strict/Legacy 両モードの出力を更新し、`evidence[]` に `tag` / `capability` / `stage.required` / `stage.actual` が揃っているか確認する。
- 期待値の変更がある場合は、CLI テキスト出力も取得し、利用者視点で破壊的でないかを確認する。
- Info/Hint など Severity 拡張を確認する場合は、`compiler/ocaml/tests/golden/diagnostics/severity/info-hint.json.golden` と `tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-info-hint.json` を併用し、`scripts/validate-diagnostic-json.sh` と `npm run ci --prefix tooling/lsp/tests/client_compat` の双方で Info/Hint が欠落していないか検証する。
- Phase 2-4 以降は `tooling/review/audit-diff.py --base <path> --target <path>` を併用し、Markdown/HTML レポート (`reports/audit/review/<commit>/diff.{md,html}`) を生成する。CI では `tooling/ci/publish-audit-diff.py` が同レポートを PR コメントへ要約投稿するため、レビュー担当者はコメントリンクを起点に確認する。
- `tooling/review/audit-diff.py --query-file tooling/review/presets/stage-regressions.dsl` のようにクエリファイルを指定して重要メタデータのみ抽出し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の指標（`diagnostic_regressions`, `audit_diff.regressions` など）に基づいて影響範囲を判断する。

## 3. CI 連携時の確認ポイント
- GitHub Actions `diagnostic-json` ジョブが失敗した場合、`tooling/lsp/tests/client_compat` のテストログと `scripts/validate-diagnostic-json.sh` の結果を参照する。
- `scripts/validate-diagnostic-json.sh` は Parser 診断の `expected.alternatives` 欠落を即時に報告するため、`tests/golden/_actual/*.actual.json` に出力されたスナップショットで期待集合が出力されているか確認する。
- スキーマ違反が発生した場合は `tooling/json-schema/diagnostic-v2.schema.json` を更新し、併せてフィクスチャを追加する。
- Windows/macOS 固有のフィクスチャ（`diagnostic-v2-ffi-macos-sample.json` など）が最新の監査ログと整合しているか確認する。
- 値制限違反診断を扱う際は `scripts/validate-diagnostic-json.sh` の `value_restriction` チェックで必須キー欠落が無いか確認し、`tooling/ci/collect-iterator-audit-metrics.py --require-success` を実行して `type_inference.value_restriction_violation` が 0 件であることを検証する。
- 効果構文 PoC を更新した場合は `tooling/ci/collect-iterator-audit-metrics.py --section effects --summary` を実行し、`syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` が期待値（PoC 期間は 0.0 / 1.0）で出力されるか確認する。`--require-success` を適用する場合は Phase 2-7 以降に導入されるゲート条件を参照する。
- `audit-review` 系ジョブ（`audit-diff`, `audit-dashboard`）が失敗した場合は、生成された `reports/audit/review/<commit>/diff.json` とダッシュボードアーティファクト (`reports/audit/dashboard/index.html`) を確認し、`collect-iterator-audit-metrics.py --section review` の出力に警告がないかチェックする。
- `tooling/review/audit-query --query '<dsl>' --from <path>` または `--query-file tooling/review/presets/<preset>.dsl` をローカルで実行し、CI の DSL クエリ結果と一致しているか検証する。差異がある場合は `tooling/review/audit_shared.py` の正規化ロジックまたは DSL プリセット (`tooling/review/presets/*.dsl`) を更新する。

## 4. レビュー用チェックリスト
- [ ] JSON スキーマ (`tooling/json-schema/diagnostic-v2.schema.json`) とサンプル出力が一致している。
- [ ] 全診断で `audit` / `timestamp` フィールドが欠落していない（`scripts/validate-diagnostic-json.sh` および `collect-iterator-audit-metrics.py --require-success` の結果を確認）。
- [ ] `scripts/validate-diagnostic-json.sh` の既定対象（`compiler/ocaml/tests/golden/diagnostics`, `compiler/ocaml/tests/golden/audit`）でエラーがない。
- [ ] `compiler/ocaml/tests/golden/typeclass_dictionary_resolved.json.golden` の辞書フィールド（`kind`, `identifier`, `repr`）が `extensions.*`／`audit_metadata` の双方で欠落していない。
- [ ] `npm run ci --prefix tooling/lsp/tests/client_compat` が成功する。
- [ ] `docs/plans/bootstrap-roadmap/2-4-status.md` に進捗や既知リスクが反映されている。
- [ ] `tooling/review/audit-diff.py` で生成した `diff.md` / `diff.html` の差分サマリが付属し、`diagnostic.regressions`・`metadata.changed` の値がレビュー対象に共有されている。
- [ ] `tooling/review/audit_dashboard.py --render` で最新の指標グラフを生成し、`reports/audit/dashboard/index.{html,md}` を確認した。
- [ ] `tooling/review/audit-query` のプリセットクエリ（例: `tooling/review/presets/stage-regressions.dsl`）を実行し、監査ログの重点領域（Stage/FFI/型クラス）に未確認のレグレッションがない。

---

この文書は Phase 2-4 の診断・監査パイプライン作業のレビュー補助ツールとして運用する。追加の手順や改善案があれば追記すること。

## 5. Phase 2-7 Step4 更新ログ（2025-11-07）
- `reports/audit/index.json` に Windows/macOS 監査ログ（`reports/audit/phase2-7/*.audit.jsonl`）を登録し、`tooling/ci/verify-audit-metadata.py --index reports/audit/index.json --strict` のローカル実行で `audit`/`timestamp` 欠落が再発しないことを確認した。`tooling/ci/create-audit-index.py` のテスト (`tooling/ci/tests/test_create_audit_index.py`) では index 生成時の `size_bytes`・`pass_rate` を検証している。
- CLI/LSP/Streaming ゴールデンは `tooling/ci/collect-iterator-audit-metrics.py --require-success` と `scripts/validate-diagnostic-json.sh` の組み合わせで再実行し、`effects.*` / `bridge.*` 拡張に差分が無いことを確認した。結果は `compiler/ocaml/tests/golden/diagnostics/` と `tooling/lsp/tests/client_compat/fixtures/` の再生成ログ、および本ドキュメントのチェックリストに記録した。
- 監査レポートの参照元を `reports/ffi-bridge-summary.md` / `reports/iterator-stage-summary*.md` から横断できるよう脚注を整理し、H3（ゴールデン拡充）の進捗レビュー結果を `compiler/ocaml/docs/technical-debt.md` に反映した。
