# pipeline_branch Stage mismatch 復旧計画（2025-12-06）

## 1. 背景と現状整理
- `examples/core_diagnostics/pipeline_branch.reml` は Core Diagnostics 章で Stage mismatch (`effects.contract.stage_mismatch`) を再現するサンプルとして設計されているが、効果ハンドラ調整中に構文エラーや型エラーが混入し、`typeck.aborted.ast_unavailable` のため本来の診断まで到達できない状態となっていた。
- 最新のローカル Run（`run_id=09d5737a-9577-4433-b5e1-717cb536d615`）では構文・型エラーを解消済みだが、依然として Stage mismatch 診断 1 件のみを返し、ゴールデンと監査レポートは旧成功パスのまま放置されている。
- `examples/core_diagnostics/pipeline_branch.expected.{diagnostic.json,audit.jsonl}` は旧成功ログ（診断 0、`pipeline.outcome=success`）を保持しており、`tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` も途中で失敗するため CI 監査と仕様サンプルの乖離が続いている。
- `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` の §5.2 実施結果欄は未更新で、Stage mismatch 再現 Run のメタデータ・監査結果が記録されていない。
- `reports/spec-audit/ch3/` 配下に `capability_stage-mismatch-YYYYMMDD.json` が存在しないため、Chapter 3 監査ログに今回の Stage mismatch 復旧作業の痕跡が残っていない。

## 2. 問題の多角的視点による分析
1. **構文・型整合性の視点**: `choose` 関数や `trigger_console` 呼び出しを変更する過程で Reml の式構文に適合しない書き方（`match` の簡略記法、負数リテラルの扱い）が混入した。Parser で `parser.syntax.expected_tokens` が発生すると Typecheck が途中終了し、Stage mismatch 診断が生成されないため、まず AST を安定させる必要がある。
2. **効果・Stage 監査の視点**: `perform Console value` を実行するだけでは Stage mismatch が発火しない場合があり、`--effect-stage` オプションや Capability Registry の Stage 定義が実行環境に依存する。`EffectAuditContext`（`compiler/frontend/src/diagnostic/effects.rs`）が要求する `required_stage`/`actual_stage` を収集できているか、`Trigger_console` 呼び出しが `do` ブロック内で Stage 追跡に干渉していないかを確認する必要がある。
3. **ゴールデン・監査ログの視点**: サンプル再現用ゴールデンが成功パスのまま固定されているため、Stage mismatch を復旧しても CLI テストが常に差分を出してしまう。`tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` が途中で落ちる要因（`pipeline_branch` 失敗）を切り出し、個別に JSON/NDJSON を反映させる必要がある。
4. **ドキュメント連携の視点**: Chapter 3-8 の計画書や監査レポート類が今回の変更を参照できないため、再発時にトレースしづらい。`docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` と `reports/spec-audit/ch3/` の両方に Run 情報と結果を記録し、`docs/spec/3-6-core-diagnostics-audit.md` のサンプル参照先を維持する必要がある。

## 3. ゴール
1. `pipeline_branch.reml` が構文・型エラーなしで Stage mismatch 1 件のみを出力する状態を安定化させる。
2. `examples/core_diagnostics/pipeline_branch.expected.{diagnostic.json,audit.jsonl}` を最新出力に合わせて更新し、`tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` が完走するようにする。
3. `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json`（日付は実行日）を作成し、`scripts/validate-diagnostic-json.sh --effect-tag runtime` で必須メタデータ検証を通過させる。
4. `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` §5.2 に本 Run の結果（Run ID、生成ファイル、フォローアップ有無）を追記し、ステークホルダーが差分を参照できる状態にする。

## 4. タスク計画
### フェーズ A: 実装と検証
1. `pipeline_branch.reml` の `trigger_console` および `pipeline_branch` 本体を確認し、`do` ブロック・効果呼び出しが Stage 監査に必要な構造（`perform Console value` を副作用として保持しつつ戻り値は Int）になっていることを確定させる。必要ならコメントで Stage mismatch を誘発する意図を明記。
2. `compiler/frontend` から `cargo run --quiet --bin reml_frontend -- --output json --emit-audit-log ../../../examples/core_diagnostics/pipeline_branch.reml` を実行し、`effects.contract.stage_mismatch` 1 件のみを確認。追加診断が出る場合は Reml ソースを再調整。

### フェーズ B: ゴールデン・監査更新
3. `tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` を再実行し、`pipeline_success`/`pipeline_branch` 双方の `.expected.*` を更新。`pipeline_branch` の JSON と NDJSON が手動整形済み（`ensure_ascii=false`）であることを確認。
4. 上記 Run の出力を `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` に保存（`CliDiagnosticEnvelope` と `AuditEnvelope` の抜粋、再現コマンド、run_id を含む）。保存後に `scripts/validate-diagnostic-json.sh --effect-tag runtime reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` を実行し、メタデータ必須キーを検証する。

### フェーズ C: ドキュメント更新
5. `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` の §5.2（実施結果）へ今回の Run ID、更新ファイル、残課題（例: Stage 要件カタログとの比較必要性）を追記。必要なら脚注で `reports/spec-audit/ch3/...` を参照する。
6. 変更概要と検証結果を `docs/notes/runtime/runtime-capability-stage-log.md` など既存の Stage 監査ログに TODO として記載するか、別途 Issue 管理へ転記する。

## 5. リスクとフォローアップ
- Stage mismatch の再現には `Console` Capability の Stage 設定（beta 要求）が前提であり、ランタイム環境によっては `at_least:stable` が発行されず診断が再現しない恐れがある。`CapabilityRegistry` の初期化や CLI オプションを変更するスクリプトが存在する場合は Run 前に確認する。
- `tooling/examples/run_examples.sh` は `pipeline_success` の更新も再実行するため、他メンテナが並列で触れている場合はゴールデン競合が起こり得る。必要に応じて `examples/core_diagnostics/README.md` に注意書きを追加する。
- 監査レポートの JSON は今後の計画に再利用するため、`docs/notes/docs-migrations.log` へ記録を残すか、`reports/spec-audit/ch3/README.md` を更新して保管方法を共有する。

## 6. 実施ログ

### Run ID: 31bed62e-f04e-4810-acc2-ce5138088068
- `tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` を実行したところ `pipeline_branch` が意図通り `exit_code=1` で停止するためスクリプトが途中で止まることを再確認。`pipeline_success` 側は同スクリプトで更新しつつ、`cargo run --quiet --bin reml_frontend -- --output json --emit-audit-log examples/core_diagnostics/pipeline_branch.reml` を単体で走らせ、標準出力/標準エラーを `tmp/` 経由でキャプチャして `examples/core_diagnostics/pipeline_branch.expected.{diagnostic.json,audit.jsonl}` を手動で整形した。
- 上記 Run の `CliDiagnosticEnvelope` / `AuditEnvelope` をまとめた `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` を作成し、`pipeline_branch` の再現手順・コマンド・stage trace をワンパッケージで参照できるようにした。
- `scripts/validate-diagnostic-json.sh reports/spec-audit/ch3/capability_stage-mismatch-20251206.json --effect-tag runtime` を実行し、`capability.id=console` / `effect.stage.required=at_least:beta` / `effect.stage.actual=at_least:stable` / `effects.contract.stage_trace` が CLI/Audit の両方に残っていることを確認した。検証ログと Run ID は `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md#5.2-実施結果` と `docs/notes/runtime/runtime-capability-stage-log.md#2025-12-06-core-diagnostics-stage-mismatch` にリンク済み。

### Run ID: 3961ffb6-ed62-499c-ad04-e1bff4bd3274
- `tooling/examples/run_examples.sh --suite core_diagnostics --with-audit` を実行し、`core_diagnostics/pipeline_branch.reml` が `effects.contract.stage_mismatch` を発火して `exit_code=1` でもスイート全体が継続することを確認。スクリプトは `core_diagnostics/pipeline_branch.reml` のみ `allowed failure` として扱い、その他の例でエラーが発生した場合は即時失敗する。
- `examples/core_diagnostics/README.md` に Stage mismatch 用サンプルであることと `allowed failure` 動作を追記し、フォローアップに挙げていた `set -e` との整合課題を本 Run でクローズした。

### Run ID: 80b0d934-6b51-4718-9fc4-dcff8c57b849
- `tooling/examples/run_examples.sh --suite core_diagnostics --with-audit --update-golden` を実行し、`pipeline_success`/`pipeline_branch` の期待値を単一コマンドで更新した。`pipeline_branch` の `exit_code=1` は `allowed failure during --update-golden` としてログに記録され、スイート全体の更新を阻害しないことを確認。
- 上記コマンドで生成した `examples/core_diagnostics/pipeline_branch.expected.{diagnostic.json,audit.jsonl}` を用いて `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` を再生成し、`capability.id=console` / `effect.stage.*` / `bridge.stage.trace` の最新メタデータと Run ID を記録した。
- `scripts/validate-diagnostic-json.sh reports/spec-audit/ch3/capability_stage-mismatch-20251206.json --effect-tag runtime` を再実行し、`capability.*` / `effect.stage.*` / `effects.contract.stage_trace` / `pipeline.*` が CLI/Audit 双方に存在することを検証した。検証結果は `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md#5.2-実施結果` と本書にリンク済み。

### Run ID: ec456a62-42bc-4cf6-9fed-5858fdc9fc83
- `tooling/examples/run_examples.sh --suite core_diagnostics --with-audit` を実行し、`pipeline_success`（run_id=`06c6a78e-be71-4323-a6fd-23e74515bf34`）と `pipeline_branch`（run_id=`ec456a62-42bc-4cf6-9fed-5858fdc9fc83`）の両方が最新 Runtime で再現できることを確認した。`pipeline_branch` は許容された失敗として `effects.contract.stage_mismatch` 1 件のみを出力し、`pipeline.outcome=success` と `pipeline.exit_code=failure` の組み合わせが audit NDJSON に揃っている。
- 本 Run の CLI/Audit 出力は既存ゴールデンと差分が無かったためファイル更新は不要だが、`docs/notes/runtime/runtime-capability-stage-log.md#2025-12-06-core-diagnostics-stage-mismatch` に run_id を追記し、5.5 節の Runbook（Capability マトリクス）変更後も Stage ミスマッチ再現手順が維持されていることを明示した。

## 7. テスト・CI 反映
1. `docs/plans/bootstrap-roadmap/assets/pipeline-branch-ci-checklist.md` を新設し、CLI 単体実行・`core_diagnostics` スイート・JSON バリデーション・監査メトリクス・CI 連携の 5 ステップを表形式でまとめた。`tooling/examples/run_examples.sh --suite core_diagnostics --with-audit --update-golden` が `allowed failure` を抱えたまま完走すること、`scripts/validate-diagnostic-json.sh --effect-tag runtime` のログと `collect-iterator-audit-metrics.py --section core_diagnostics --scenario pipeline_branch` の結果を `reports/spec-audit/ch3/pipeline_branch-metrics-YYYYMMDD.json` として保存することを必須条件にしている。
2. CI 向けには `.github/workflows/core-diagnostics.yml`（Phase 3 で追加予定）に `pipeline_branch` ステップを分離し、失敗時に `scripts/ci/post_failure_runtime_capability.sh` を呼んで `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#runtime-capability` と `docs/notes/runtime/runtime-capability-stage-log.md` へ Run ID を書き戻す手順を定義した。CI の成果物には `reports/spec-audit/ch3/capability_stage-mismatch-YYYYMMDD.json`・`...-metrics-YYYYMMDD.json` をまとめ、3.8 計画 §7 で新設した `runtime.capability_ci_pass_rate` KPI の補助資料として扱う。
3. 本節の更新に合わせて `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md#7-テスト・ci-統合` と相互参照を追加し、Runtime Capability 計画（7.2/7.3）から `core_diagnostics` スイートの allowed failure と `pipeline_branch` 専用メトリクスの扱いを共有した。Stage mismatch を再現する唯一のサンプルである点を `docs/spec/3-6-core-diagnostics-audit.md`／`docs/guides/runtime/runtime-bridges.md`／`docs/guides/ecosystem/ai-integration.md` へ伝播させる。
