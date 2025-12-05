# pipeline_branch Stage mismatch 復旧計画（2025-12-06）

## 1. 背景と現状整理
- `examples/core_diagnostics/pipeline_branch.reml` は Core Diagnostics 章で Stage mismatch (`effects.contract.stage_mismatch`) を再現するサンプルとして設計されているが、効果ハンドラ調整中に構文エラーや型エラーが混入し、`typeck.aborted.ast_unavailable` のため本来の診断まで到達できない状態となっていた。
- 最新のローカル Run（`run_id=09d5737a-9577-4433-b5e1-717cb536d615`）では構文・型エラーを解消済みだが、依然として Stage mismatch 診断 1 件のみを返し、ゴールデンと監査レポートは旧成功パスのまま放置されている。
- `examples/core_diagnostics/pipeline_branch.expected.{diagnostic.json,audit.jsonl}` は旧成功ログ（診断 0、`pipeline.outcome=success`）を保持しており、`tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` も途中で失敗するため CI 監査と仕様サンプルの乖離が続いている。
- `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` の §5.2 実施結果欄は未更新で、Stage mismatch 再現 Run のメタデータ・監査結果が記録されていない。
- `reports/spec-audit/ch3/` 配下に `capability_stage-mismatch-YYYYMMDD.json` が存在しないため、Chapter 3 監査ログに今回の Stage mismatch 復旧作業の痕跡が残っていない。

## 2. 問題の多角的視点による分析
1. **構文・型整合性の視点**: `choose` 関数や `trigger_console` 呼び出しを変更する過程で Reml の式構文に適合しない書き方（`match` の簡略記法、負数リテラルの扱い）が混入した。Parser で `parser.syntax.expected_tokens` が発生すると Typecheck が途中終了し、Stage mismatch 診断が生成されないため、まず AST を安定させる必要がある。
2. **効果・Stage 監査の視点**: `perform Console value` を実行するだけでは Stage mismatch が発火しない場合があり、`--effect-stage` オプションや Capability Registry の Stage 定義が実行環境に依存する。`EffectAuditContext`（`compiler/rust/frontend/src/diagnostic/effects.rs`）が要求する `required_stage`/`actual_stage` を収集できているか、`Trigger_console` 呼び出しが `do` ブロック内で Stage 追跡に干渉していないかを確認する必要がある。
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
2. `compiler/rust/frontend` から `cargo run --quiet --bin reml_frontend -- --output json --emit-audit-log ../../../examples/core_diagnostics/pipeline_branch.reml` を実行し、`effects.contract.stage_mismatch` 1 件のみを確認。追加診断が出る場合は Reml ソースを再調整。

### フェーズ B: ゴールデン・監査更新
3. `tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` を再実行し、`pipeline_success`/`pipeline_branch` 双方の `.expected.*` を更新。`pipeline_branch` の JSON と NDJSON が手動整形済み（`ensure_ascii=false`）であることを確認。
4. 上記 Run の出力を `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` に保存（`CliDiagnosticEnvelope` と `AuditEnvelope` の抜粋、再現コマンド、run_id を含む）。保存後に `scripts/validate-diagnostic-json.sh --effect-tag runtime reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` を実行し、メタデータ必須キーを検証する。

### フェーズ C: ドキュメント更新
5. `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` の §5.2（実施結果）へ今回の Run ID、更新ファイル、残課題（例: Stage 要件カタログとの比較必要性）を追記。必要なら脚注で `reports/spec-audit/ch3/...` を参照する。
6. 変更概要と検証結果を `docs/notes/runtime-capability-stage-log.md` など既存の Stage 監査ログに TODO として記載するか、別途 Issue 管理へ転記する。

## 5. リスクとフォローアップ
- Stage mismatch の再現には `Console` Capability の Stage 設定（beta 要求）が前提であり、ランタイム環境によっては `at_least:stable` が発行されず診断が再現しない恐れがある。`CapabilityRegistry` の初期化や CLI オプションを変更するスクリプトが存在する場合は Run 前に確認する。
- `tooling/examples/run_examples.sh` は `pipeline_success` の更新も再実行するため、他メンテナが並列で触れている場合はゴールデン競合が起こり得る。必要に応じて `examples/core_diagnostics/README.md` に注意書きを追加する。
- 監査レポートの JSON は今後の計画に再利用するため、`docs-migrations.log` へ記録を残すか、`reports/spec-audit/ch3/README.md` を更新して保管方法を共有する。
