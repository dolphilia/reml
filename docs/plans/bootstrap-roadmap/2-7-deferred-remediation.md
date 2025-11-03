# 2.7 診断パイプライン残課題・技術的負債整理計画

## 目的
- Phase 2-4 で持ち越した診断・監査パイプライン関連タスクと技術的負債（ID 22/23 など）を集中して解消する。
- CLI/LSP/CI の各チャネルで `Diagnostic` / `AuditEnvelope` の新仕様を安定運用できる状態を整え、Phase 2-8 の仕様検証に備える。

## スコープ
- **含む**: Windows/macOS CI での監査ゲート導入、LSP V2 互換テスト整備、CLI フォーマッタの再統合、技術的負債リストで Phase 2 中に解消可能な項目。
- **含まない**: 仕様書の全文レビュー（Phase 2-8 で実施）、新規機能の追加、Phase 3 以降へ移送済みの低優先度負債。
- **前提**:
  - Phase 2-4 の共通シリアライズ層導入と JSON スキーマ検証が完了していること。
  - Phase 2-5 の仕様差分補正で参照する基礎データ（差分リスト草案）が揃っていること。
  - Phase 2-6 の Windows 実装で `--emit-audit` を実行できる環境が CI 上に整備済みであること。

## 作業ディレクトリ
- `compiler/ocaml/src/cli/` : `diagnostic_formatter.ml`, `json_formatter.ml`, `options.ml`
- `compiler/ocaml/src/diagnostic_*` : Builder/API 互換レイヤ
- `tooling/lsp/` : `diagnostic_transport.ml`, `compat/`, `tests/client_compat`
- `tooling/ci/` : `collect-iterator-audit-metrics.py`, `sync-iterator-audit.sh`, 新規検証スクリプト
- `scripts/` : CI 向け検証スクリプト、レビュー補助ツール
- `reports/` : 監査ログサマリ、診断フォーマット差分
- `compiler/ocaml/docs/technical-debt.md` : ID 22/23, H1〜H4 の進捗更新

## フェーズ実行順序（引き継ぎ反映）

| 順序 | フォーカス | 主な事前条件 | 本書の参照 |
| --- | --- | --- | --- |
| 0 | フェーズ起動とハンドオーバー整備 | `docs/plans/bootstrap-roadmap/2-5-to-2-7-handover.md` §6、`docs/plans/bootstrap-roadmap/2-5-to-2-7-type-002-handover.md` | §0 フェーズ起動とハンドオーバー整備 |
| 1 | 監査ゲート強化（Windows/macOS CI） | フェーズ起動完了、共通スクリプト整備 | §1 監査ゲート整備 |
| 2 | Unicode 識別子プロファイルの既定化 | Kickoff 合意事項、監査ゲート稼働 | §7 Unicode 識別子プロファイル移行 |
| 3 | 効果構文・効果操作 PoC の有効化 | Unicode 移行のテレメトリ安定 | §8 効果構文 PoC 移行 |
| 4 | 効果行統合（TYPE-002） | 効果構文 PoC の KPI 1.0 維持、`type_row_mode=dual-write` 準備 | §TYPE-002 効果行統合ロードマップ |
| 5 | CLI/LSP/Streaming 出力整備と負債クローズ | 監査ゲート・効果系実装の成果物 | §2〜§6 |
| 6 | Phase 2-8 への引き継ぎ | KPI 1.0 維持、脚注撤去条件達成 | §5 Phase 2-8 への引き継ぎ準備 |

## 作業ブレークダウン

### 0. フェーズ起動とハンドオーバー整備（34週目前半）
*参照*: [2-5-to-2-7-handover.md](./2-5-to-2-7-handover.md#6-phase-2-7-初期アクションチェックリスト)、[2-5-to-2-7-type-002-handover.md](./2-5-to-2-7-type-002-handover.md)、[compiler/ocaml/docs/technical-debt.md](../../compiler/ocaml/docs/technical-debt.md)

0.1. **キックオフレビューと役割確認**
- LEXER-001 / SYNTAX-001 / SYNTAX-003 / EFFECT-002 / TYPE-002 の担当リード合同レビューを開催し、境界 API とスプリント順序を確定する。決定事項は `docs/plans/bootstrap-roadmap/2-5-review-log.md` に `PHASE2-7-KICKOFF` タグで追記する。
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` から各ハンドオーバー資料へ遷移できることを確認し、リンク切れがあれば本書と関連資料を同時更新する。
- **完了状況 (2025-11-04)**: Kickoff 合意事項を `docs/plans/bootstrap-roadmap/2-5-review-log.md#phase2-7-キックオフレビュー2025-11-04` に記録し、本節の参照リンクを更新してハンドオーバー資料へ直接遷移できることを確認した。

0.2. **計測スクリプトと CI ベースライン**
- `tooling/ci/collect-iterator-audit-metrics.py` と `scripts/validate-diagnostic-json.sh` の Phase 2-7 ブランチを作成し、`--require-success` での実行結果を共有ドライブへ保存する。Windows/macOS 用のプリセットが未整備の場合はこの段階で追加する。
- KPI の初期値（`lexer.identifier_profile_unicode`, `syntax.effect_construct_acceptance`, `diagnostics.effect_row_stage_consistency` など）を測定し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に起動時ベースラインとして記録する。
- **完了状況 (2025-11-04)**: Phase 2-7 キックオフ時点のベースライン（`lexer.identifier_profile_unicode = 0.0`, `syntax.effect_construct_acceptance = 0.0`, `diagnostics.effect_row_stage_consistency = null`）を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追記し、スクリプトの Phase 2-7 プロファイル確認結果を `docs/plans/bootstrap-roadmap/2-5-review-log.md#phase2-7-キックオフレビュー2025-11-04` に記録した。

0.3. **脚注・リスク・RunConfig ガードの整合**
- `docs/spec/1-1-syntax.md` ほか脚注 `[^lexer-ascii-phase25]`, `[^effects-syntax-poc-phase25]`, `[^type-row-metadata-phase25]` の撤去条件を再確認し、移行時に必要なチェックリストを本書該当セクションへ反映する。
- `0-4-risk-handling.md` の関連リスク（Unicode XID、効果構文 Stage、TYPE-002 ROW 統合）を Phase 2-7 担当者へ再アサインし、週次レビューのエスカレーション経路を共有する。`compiler/ocaml/docs/technical-debt.md` に記載された ID 22/23 の対応状況を初期ステータスとして確認する。
- **完了状況 (2025-11-04)**: 脚注撤去条件を再確認し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に Phase 2-7 Parser・Effects・Type チームを担当として追記した。技術的負債 ID22/23 の現状は `compiler/ocaml/docs/technical-debt.md` の記載どおりで未変更であることを確認済み。

**成果物**: キックオフ議事録、最新ベースラインメトリクス、脚注およびリスク整合メモ

### 1. 監査ゲート整備（34-35週目）
**担当領域**: Windows/macOS CI

1.1. **Windows Stage 自動検証 (ID 22)**
- `tooling/ci/sync-iterator-audit.sh` を MSYS2 Bash で動作させ、`--platform windows-msvc` 実行パスを整備。
- `tooling/ci/collect-iterator-audit-metrics.py` に Windows プラットフォーム専用プリセット (`--platform windows-msvc`) を追加し、`ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` を算出。
- `bootstrap-windows.yml` に `audit-matrix` ジョブを追加し、pass_rate < 1.0 の場合は PR を失敗させる。
- `reports/ffi-bridge-summary.md` と `docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md` の TODO 欄を更新。
- DIAG-002 で追加した `diagnostic.audit_presence_rate` をダッシュボードへ組み込み、`python3 tooling/ci/collect-iterator-audit-metrics.py --require-success` の結果を Windows 行にも掲載する（ソース: `compiler/ocaml/tests/golden/diagnostics/**/*.json.golden` / `compiler/ocaml/tests/golden/audit/**/*.json[l].golden`）。

1.2. **macOS FFI サンプル自動検証 (ID 23)**
- `ffi_dispatch_async.reml` / `ffi_malloc_arm64.reml` をビルド可能なよう修正し、`scripts/ci-local.sh --target macos-arm64 --emit-audit` に組み込む。
- `collect-iterator-audit-metrics.py` で `bridge.platform = macos-arm64` の pass_rate 集計を追加し、`ffi_bridge.audit_pass_rate` に反映。
- `bootstrap-macos.yml` に監査ゲートを追加し、成果物 (audit JSON, summary) をアーティファクト化。

- **完了状況 (2025-11-06)**: `tooling/ci/collect-iterator-audit-metrics.py` に `--platform` フィルタを実装し、Windows (`windows-msvc`) / macOS (`macos-arm64`) / Linux それぞれで `ffi_bridge.audit_pass_rate` と `iterator.stage.audit_pass_rate` を個別にゲートできるようにした。`bootstrap-windows.yml`・`bootstrap-macos.yml` へ同オプションを適用したことで、Windows CI は `tooling/ci/iterator-audit-metrics.json` が `1.0` 未満の場合に失敗し、macOS CI も `iterator-audit` ジョブで `macos-arm64` の pass_rate を強制する。監査サマリ (`reports/iterator-stage-summary-*.md`) と `reports/ffi-bridge-summary.md` を更新し、ID 22/23 の技術的負債は解消済みとして記録した。

**成果物**: Windows/macOS CI 監査ゲート、更新済みレポート、技術的負債リスト反映

### 2. CLI 出力統合とテキストフォーマット刷新（35週目前半）
**担当領域**: CLI フォーマッタ

2.1. **`--format` / `--json-mode` 集約**
- `compiler/ocaml/src/cli/options.ml` で `--format` と `--json-mode` の派生オプションを整理し、`SerializedDiagnostic` を利用するフォーマッタ選択ロジックを再構築。
- `docs/spec/0-0-overview.md` と `docs/guides/ai-integration.md` に新オプションを追記。

2.2. **テキストフォーマット刷新**
- `compiler/ocaml/src/cli/diagnostic_formatter.ml` を `SerializedDiagnostic` ベースへ移行し、`unicode_segment.ml`（新規）を導入して Grapheme 単位のハイライトを実装。
- `--format text --no-snippet` を追加し、CI 向けログを簡略化。
- テキストゴールデン (`compiler/ocaml/tests/golden/diagnostics/*.golden`) を更新し、差分は `reports/diagnostic-format-regression.md` に記録。

- **完了状況 (2025-11-08)**: `Diagnostic_formatter` / `Json_formatter` / `main.ml` を `Diagnostic_serialization` 正規化経由に切り替え、`--format`／`--json-mode` の分岐が単一の `SerializedDiagnostic` を共有するよう統合した。テキスト／JSON ゴールデン（`compiler/ocaml/tests/golden/**`）を最新出力で更新し、`dune runtest` による回帰確認を完了。空配列の省略ルールは `reports/diagnostic-format-regression.md` に追記済み。

**成果物**: CLI オプション整理、テキストフォーマッタ更新、ドキュメント追記

### 3. LSP V2 互換性確立（35週目後半）
**担当領域**: LSP・フロントエンド

3.1. **フィクスチャ拡充とテスト**
- `tooling/lsp/tests/client_compat/fixtures/` に効果診断・Windows/macOS 監査ケースを追加し、AJV スキーマ検証を更新。
- `npm run ci` にフィクスチャ差分のレポート出力を追加し、PR で参照可能にする。

3.2. **`lsp-contract` CI ジョブ**
- GitHub Actions に `lsp-contract` ジョブを追加し、V1/V2 双方の JSON を `tooling/json-schema/diagnostic-v2.schema.json` で検証。
- `tooling/lsp/README.md` と `docs/guides/plugin-authoring.md` に V2 連携手順を追記。

3.3. **互換レイヤ仕上げ**
- `tooling/lsp/compat/diagnostic_v1.ml` を安定化させ、`[@deprecated]` 属性を付与。
- `tooling/lsp/jsonrpc_server.ml` で `structured_hints` の `command`/`data` 変換エラーを `extensions.lsp.compat_error` に記録。

3.4. **Recover FixIt 継続整備**
- `Parser_expectation.Packrat` に `recover` スナップショットを保持するハンドルを追加し、Packrat 経路でも `parser.recover_fixit_coverage = 1.0` を維持する。検証手順と残課題は `docs/notes/core-parse-streaming-todo.md` に追記済み。
- `Diagnostic.Builder.add_note` が生成する `recover` notes をローカライズ可能なテンプレートへ移行し、CLI/LSP のテキスト刷新と連動して多言語化を完了させる。`docs/spec/2-5-error.md`・`docs/spec/3-6-core-diagnostics-audit.md` の脚注と整合させる。
- ストリーミング Pending → resume 循環で FixIt が重複発火しないことを監査ログ (`StreamOutcome.Pending.extensions.recover`) と `collect-iterator-audit-metrics.py` の新指標で確認する。必要に応じて CI に検証ステップを追加する。

- **進捗記録 (2025-11-05)**:
  - `tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-effects-sample.json` と `diagnostic-v2-ffi-sample.json`（Windows Stage ミスマッチ）および `diagnostic-v2-ffi-macos-sample.json` を確認し、効果・Windows/macOS 向けのフィクスチャカバレッジが Phase 2-7 要件を満たすことをレビュー済み。Packrat 復旧系フィクスチャは今後追加が必要。
  - `tooling/lsp/tests/client_compat/client-v2.ts` が `tooling/json-schema/diagnostic-v2.schema.json` を AJV で検証していることを確認。フィクスチャ差分レポートを自動生成する `scripts/report-fixture-diff.mjs`（仮称）を Week35 中に追加し、`npm run ci` から `reports/diagnostic-format-regression.md` へ貼り付けられるようにするタスクを登録した。
  - `.github/workflows` 配下に LSP 専用 CI が存在しないため、`lsp-contract.yml` を追加して V1/V2 JSON の AJV 検証とフィクスチャ差分収集を自動化する作業を次スプリントへ繰り越した。
  - `tooling/lsp/compat/diagnostic_v1.ml` は最小限のダウングレード実装のみで `[@deprecated]` 属性や欠損フィールド補完が未実装。変換失敗時に `extensions["lsp.compat_error"]` を付与する処理を `tooling/lsp/jsonrpc_server.ml` へ追加する必要がある。
  - `compiler/ocaml/src/parser_expectation.ml`・`parser_expectation.mli` と `compiler/ocaml/src/diagnostic.ml` を確認したが、`recover` スナップショットやローカライズテンプレートの実装は未着手。`collect-iterator-audit-metrics.py` へ `parser.recover_fixit_coverage` 指標を追加し、Packrat 経路を含む測定ループを整備するフォローアップを設定した。

**成果物**: 拡充済み LSP テスト群、CI ジョブ、更新ドキュメント

### 4. 技術的負債の棚卸しとクローズ（36週目前半）
**担当領域**: 負債管理

4.1. **技術的負債リスト更新**
- `compiler/ocaml/docs/technical-debt.md` で ID 22 / 23 を完了扱いに更新し、H1〜H4 の進捗をレビュー。
- Phase 2 以内に解消できなかった項目を Phase 3 へ移送し、`0-4-risk-handling.md` に直結するリスクとして記録。

4.2. **レポート更新**
- `reports/diagnostic-format-regression.md` と `reports/ffi-bridge-summary.md` に完了状況を追記し、差分がないことを確認。
- 監査ログの成果物パスを `reports/audit/index.json` に登録し、`tooling/ci/create-audit-index.py` のテストを更新。

**成果物**: 最新化された技術的負債リスト、報告書更新、移送リスト

- **完了状況 (2025-11-07)**: `compiler/ocaml/docs/technical-debt.md` で ID22/23 を完了扱いに更新し、H1〜H4 のレビュー結果を追記した。`reports/diagnostic-format-regression.md` / `reports/ffi-bridge-summary.md` へ Step4 の差分確認ログを追加し、`reports/audit/phase2-7/*.audit.jsonl` と `reports/audit/index.json` を生成。`tooling/ci/tests/test_create_audit_index.py` を新設し、index 生成ロジックの単体テストを整備済み。

### 6. ストリーミング PoC フォローアップ（Phase 2-7 序盤）
*参照*: `docs/guides/core-parse-streaming.md`, `docs/guides/runtime-bridges.md`, `docs/spec/2-7-core-parse-streaming.md`, `docs/plans/bootstrap-roadmap/2-5-to-2-7-handover.md` §3.4-§3.5  
**担当領域**: Core.Parse.Streaming / Runtime Bridge / CLI

6.1. **Packrat キャッシュ共有と KPI 監視**
- `Parser_driver.Streaming` → `Parser_driver.run` の委譲境界を整理し、`Core_parse.State.memo` と `ContinuationMeta.commit_watermark` を同一ヒープに保持する。`compiler/ocaml/src/parser_driver.ml` / `parser_expectation.ml` を dual-write し、`compiler/ocaml/tests/streaming_runner_tests.ml` に Pending/Resume のスナップショットテストを追加する。
- `parser.stream.outcome_consistency` を `collect-iterator-audit-metrics.py --section streaming` に新設し、`reports/audit/dashboard/streaming.md` で Linux/Windows/macOS の pass_rate を比較できるようにする。1.0 未満の場合は当該チャンクの `ContinuationMeta.resume_lineage` を差分として記録する。
- `docs/spec/2-7-core-parse-streaming.md` の `Continuation` / `StreamMeta` 節へ `memo_bytes`・`resume_lineage` の脚注を追加し、Packrat 共有要件を仕様へ反映する。

6.2. **FlowController とバックプレッシャ自動化**
- `RunConfig.extensions["stream"].flow` を構造体化し、`FlowController.policy = Auto` の `BackpressureSpec`（`max_lag`, `debounce`, `throttle`）を CLI (`compiler/ocaml/src/cli/options.ml`) / LSP (`tooling/lsp/run_config_loader.ml`) から設定できるようにする。
- `--stream-flow auto` 指定時に `DemandHint.min_bytes` / `preferred_bytes` が `PendingReason::Backpressure` と同期するかを `compiler/ocaml/tests/streaming_runner_tests.ml` と `tooling/lsp/tests/client_compat/streaming_*.json` で検証する。
- `docs/guides/core-parse-streaming.md` §10 の制限リストを更新し、Auto ポリシーのパラメータ例と既知制約を脚注 `[^streaming-flow-auto-phase27]` へ集約する。

6.3. **Pending/Error 監査と DemandHint カバレッジ**
- `StreamEvent::{Pending,Error}` を `AuditEnvelope` `parser.stream.pending` / `parser.stream.error` へ転送し、`resume_hint`, `last_reason`, `continuation.meta.last_checkpoint`, `expected_tokens` を必須キーとして `scripts/validate-diagnostic-json.sh --suite streaming` で検証する。
- `parser.stream.demandhint_coverage` 指標を 1.0 で維持するため、`collect-iterator-audit-metrics.py --require-success --section streaming` で DemandHint 欠損をガードし、逸脱時は `0-4-risk-handling.md` の `STREAM-POC-DEMANDHINT` リスクを再オープンする。
- LSP/CLI 共通で `StreamEvent::Error` から `Diagnostic.extensions["recover"]` と `expected_tokens` を生成する経路を `parser_expectation.ml` と `diagnostic_serialization.ml` で共有し、`reports/diagnostic-format-regression.md` にストリーミング専用の回帰ログを追加する。

6.4. **CLI / JSON メトリクス連携**
- `Cli.Stats` と JSON 出力 (`compiler/ocaml/src/cli/json_formatter.ml`) に `stream_meta.bytes_consumed`, `stream_meta.resume_count`, `stream_meta.await_count`, `stream_meta.backpressure_events` を追加し、`compiler/ocaml/tests/golden/diagnostics/streaming/*.json.golden` を整備する。
- LSP publishDiagnostics にも `stream_meta` を添付し、`tooling/lsp/tests/client_compat/streaming_meta*.snapshot` で比較する。`docs/spec/2-1-parser-type.md` §D の RunConfig 共有節に `extensions["stream"].stats=true` の運用例を追記。
- CLI `--stats` 出力と `reports/audit/index.json` の指標名を同期し、ログ収集基盤が `stream_meta.*` を自動集計できるよう `docs/guides/ai-integration.md` のログ例を更新する。

6.5. **Runtime Bridge 連携と Stage 監査**
- `docs/guides/runtime-bridges.md` §10 を更新し、`DemandHint` / Backpressure hooks を Runtime Bridge へ渡すチェックリストと `effects.contract.stage_mismatch` 連携手順を追加する。
- `RuntimeBridgeRegistry` に `stream_signal` ハンドラを追加し、`PendingReason::Backpressure` を `bridge.stage.backpressure` 診断で監査する。`reports/ffi-bridge-summary.md` にストリーミング信号の導入結果を追記する。
- `collect-iterator-audit-metrics.py --platform windows-msvc --section streaming` を週次で実行し、Windows でも Backpressure signal が取得できるよう `docs/plans/bootstrap-roadmap/2-6-windows-support.md` の監査要件と同期させる。

6.6. **レポート化とフォローアップ共有**
- `reports/audit/dashboard/streaming.md` を新設し、Packrat 共有・Backpressure・DemandHint カバレッジ・Runtime Bridge signal の KPI と計測手順を一覧化する。
- `compiler/ocaml/docs/technical-debt.md` に `STREAM-POC-PACKRAT` / `STREAM-POC-BACKPRESSURE` を追加し、クローズ条件を本節の KPI に揃える。達成後は `docs/notes/core-parse-streaming-todo.md` へ移送可否を記録する。
- 週次レビューで 6.1〜6.5 の数値を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に転記し、Phase 2-8 キックオフ資料でも同じ表を参照できるようにする。

**成果物**: Packrat 共有済み Streaming ランナー、FlowController Auto 設定、Pending/Error 監査ログ、`stream_meta` 付き CLI/LSP 出力、Runtime Bridge 拡張ガイド、`reports/audit/dashboard/streaming.md`

- **完了状況 (2025-11-04)**: 6.1〜6.6 の作業単位と KPI を明確化し、参照資料・成果物・監査手順を本節に集約した。今後の実装進捗は各小項目へ検証ログを追記し、`collect-iterator-audit-metrics.py` と `docs/guides/runtime-bridges.md` の更新タイミングを同期させる。

### 7. Unicode 識別子プロファイル移行（SYNTAX-001 / LEXER-001）
*参照*: `docs/plans/bootstrap-roadmap/2-5-to-2-7-handover.md` §3.1-§3.2、`docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-001-proposal.md`、`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-001-proposal.md`
**担当領域**: Lexer / Docs / Tooling

7.1. **XID テーブル整備**
- `scripts/` 配下に UnicodeData 由来の `XID_Start` / `XID_Continue` テーブル生成スクリプトを追加し、CI キャッシュとライセンス整備を実施する。生成物は `compiler/ocaml/src/lexer_tables/`（新設予定）で管理し、`dune` の `@check-unicode-tables` で再生成チェックを行う。
- `compiler/ocaml/src/lexer.mll` と `Core_parse.Lex` に新テーブルを組み込み、`--lex-profile=unicode` を既定へ移行する段階的ロードマップを作成する。ASCII プロファイルは互換モードとして残し、切り替え手順を `docs/spec/2-3-lexer.md` に記載する。

7.2. **テストとメトリクス**
- CI で `REML_ENABLE_UNICODE_TESTS=1` を常時有効化し、`compiler/ocaml/tests/unicode_ident_tests.ml` と `unicode_identifiers.reml` フィクスチャを全プラットフォームで実行する。`collect-iterator-audit-metrics.py --require-success` の `parser.runconfig.lex.profile` 集計で `unicode` が 100% となることを確認する。
- `lexer.identifier_profile_unicode` 指標が 1.0 へ遷移した日付とログを `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追記し、値が下回った場合は `0-4-risk-handling.md` のリスクを更新する。

7.3. **ドキュメントとクライアント整備**
- `docs/spec/1-1-syntax.md`・`docs/spec/1-5-formal-grammar-bnf.md` の暫定脚注を撤去し、Unicode 識別子仕様への更新内容を `docs/spec/0-2-glossary.md` と `docs/spec/README.md` に波及させる。
- CLI/LSP のエラーメッセージから ASCII 制限文言を除去し、Unicode 識別子が正しく表示されることを `compiler/ocaml/tests/golden/diagnostics` と `tooling/lsp/tests/client_compat` で検証する。`docs/guides/plugin-authoring.md` と `docs/notes/dsl-plugin-roadmap.md` のチェックリストを更新する。
- `docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-001-proposal.md` Step5/6 の進捗を反映し、完了後は Phase 2-8 へ脚注撤去タスクを引き継ぐ。

**成果物**: Unicode プロファイル既定の lexer/parser、更新済みテスト・CI 指標、仕様およびガイドの脚注整理

### 8. 効果構文 PoC 移行（SYNTAX-003 / EFFECT-002）
*参照*: `docs/plans/bootstrap-roadmap/2-5-to-2-7-handover.md` §3.3-§3.4、`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-003-proposal.md`、`docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-002-proposal.md`
**担当領域**: 効果システム / CLI / CI

8.1. **PoC 実装の統合**
- `parser.mly` に `perform` / `do` / `handle` を受理する規則を導入し、`Type_inference_effect` へ `TEffectPerform` / `TEffectHandle`（仮称）を追加する。PoC 設計（Phase 2-5 S1/S2）を反映し、`Σ_before` / `Σ_after` の差分が残余効果診断へ渡ることを確認する。
- `compiler/ocaml/tests/effect_syntax_tests.ml` を新設し、成功ケース・未捕捉ケース・Stage ミスマッチケースをゴールデン化する。`collect-iterator-audit-metrics.py --section effects` で `syntax.effect_construct_acceptance = 1.0`、`effects.syntax_poison_rate = 0.0` を期待値としてゲート化する。
- `tooling/ci/collect-iterator-audit-metrics.py` に effect 指標の集計関数を実装し、`--require-success` 時には両指標が 1.0 でない場合に失敗するようガードを追加する。逸脱時は `0-4-risk-handling.md` へ登録。

8.2. **フラグ運用とドキュメント**
- `-Zalgebraic-effects`（仮称）を CLI/LSP/ビルドスクリプトで共通制御する。CLI オプションは `compiler/ocaml/src/cli/options.ml`、LSP は `tooling/lsp/tests/client_compat/fixtures/` で検証し、ビルドスクリプトは `scripts/validate-diagnostic-json.sh` や CI 定義に Experimental フラグを反映する。
- 仕様書 (`docs/spec/1-1-syntax.md`・`1-5-formal-grammar-bnf.md`・`3-8-core-runtime-capability.md`) と索引 (`docs/spec/README.md`) に付与した脚注 `[^effects-syntax-poc-phase25]` の撤去条件を整理し、Stage = Stable へ到達した後に Phase 2-8 へ通知する運用を確立する。
- `docs/notes/dsl-plugin-roadmap.md` に効果ハンドラと Capability Stage の整合チェックを追加し、`effects.contract.stage_mismatch` / `bridge.stage.*` 診断が PoC 実装で再現できることを検証する。

8.3. **ハンドオーバーとレビュー**
- `docs/notes/effect-system-tracking.md` の「Phase 2-5 S4 引き継ぎパッケージ」に沿って、PoC 到達条件と残課題を確認。チェックリスト H-O1〜H-O5 が完了した時点で `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` に更新メモを残す。
- 週次レビューで効果構文の Stage 遷移を報告し、`syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` の推移を `0-3-audit-and-metrics.md` へ記録する。脚注撤去可否は Phase 2-7 終盤のレビューで判断する。

**成果物**: 効果構文 PoC 実装、CI メトリクス 100% 化、フラグ運用指針、脚注撤去条件の整理

### TYPE-002 効果行統合ロードマップ {#type-002-effect-row-integration}
*参照*: `docs/plans/bootstrap-roadmap/2-5-to-2-7-type-002-handover.md`、`docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-002-proposal.md`
**担当領域**: Type + Effects + QA  
**着手条件**: Phase 2-5 TYPE-002 Step1〜Step4 が完了しており、`compiler/ocaml/docs/effect-system-design-note.md` §3、`docs/spec/1-2-types-Inference.md` / `1-3-effects-safety.md` / `3-6-core-diagnostics-audit.md` の脚注 `[^type-row-metadata-phase25]`、および `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の新規 KPI が整合していること。

**スプリント構成（想定: Week35〜Week37）**

1. **Sprint A — 型表現と dual-write 基盤**  
   - `types.ml` に `effect_row` レコード（`declared` / `residual` / `canonical` / `row_var`）を導入し、`TArrow of ty * effect_row * ty` を追加。  
   - `typed_ast.ml` と `Type_inference` で `effect_row` を構築しつつ、既存の `typed_fn_decl.tfn_effect_profile` を並行保持する dual-write モードを実装。  
   - `RunConfig.extensions["effects"].type_row_mode` に `dual-write` を追加し、CLI/LSP/CI オプションで `metadata-only` ↔ `dual-write` を切り替えられるようにする。  
   - 監査ログへ `effect.type_row.{declared,residual,canonical}` を出力し、`collect-iterator-audit-metrics.py --section effects` のベースラインを記録。

2. **Sprint B — 推論・テスト・KPI 実装**  
   - `generalize` / `instantiate` / `Type_unification` / `constraint_solver.ml` で `effect_row` を扱うユーティリティを実装し、RowVar は予約値 (`Open`) として保持。  
   - `Effect_analysis.merge_usage_into_profile` と `Type_inference_effect` を更新し、残余効果が `effect_row.residual` へ反映されるようにする。  
   - テストスイート: `compiler/ocaml/tests/test_type_inference.ml` に `type_effect_row_equivalence_*` ケース、`compiler/ocaml/tests/streaming_runner_tests.ml` に `streaming_effect_row_stage_consistency` を追加。  
   - KPI: `collect-iterator-audit-metrics.py --require-success --section effects` で `diagnostics.effect_row_stage_consistency = 1.0`, `type_effect_row_equivalence = 1.0`, `effect_row_guard_regressions = 0` をゲート条件に設定。逸脱時は自動ロールバック（`type_row_mode=metadata-only`）を実行し、`0-4-risk-handling.md` に登録。

3. **Sprint C — Core IR 伝播とプラットフォーム検証**  
   - `core_ir/desugar_fn.ml`, `core_ir/iterator_audit.ml`, `runtime/effect_registry.ml` を更新し、IR/Runtime の効果情報が `effect_row` を参照できる状態にする。  
   - Windows/macOS CI ワークフローを更新し、`collect-iterator-audit-metrics.py --section effects --platform <target>` で `effect_row_guard_regressions` が 0 件であることを確認。  
   - CLI/LSP ゴールデンを更新し、dual-write 期間中の差分レビューを `reports/diagnostic-format-regression.md` §2 に追記。  
   - 仕様脚注 `[^type-row-metadata-phase25]` を撤去するためのチェックリスト（KPI 1.0 維持・監査ログ整合・Docs/Type レビュー承認）を満たした時点で Phase 2-8 へ報告。

**検証・完了条件**
- `dune runtest compiler/ocaml/tests/test_type_inference.ml --force` で `type_effect_row_equivalence_*` シリーズが全て成功し、CI 集計で 1.0 を報告する。  
- `collect-iterator-audit-metrics.py --require-success --section effects` が Linux/macOS/Windows すべてで成功し、`effect_row_guard_regressions = 0` のまま `ty-integrated` へ切り替えが完了する。  
- dual-write → `ty-integrated` への移行後、`effects.type_row.integration_blocked` 診断が発生しないことを CLI/LSP/監査のゴールデンで確認し、必要に応じて `--type-row-mode=metadata-only` で旧挙動へ戻せる。  
- `docs/spec/1-2-types-Inference.md` / `1-3-effects-safety.md` / `3-6-core-diagnostics-audit.md` の効果行脚注を削除し、`docs/notes/effect-system-tracking.md` と本書に完了メモ（解除日・KPI 値・レビュー承認者）を記録する。

**ハンドオーバー**
- Step5（Phase 2-5 TYPE-002）で作成するハンドオーバーノートを参照し、dual-write 期間の監査ログとテストログを保管。  
- RowVar（行多相）については Phase 3 へ移管し、`constraint_solver` 拡張案・API 予約値の扱い・性能評価計画を `effect-system-tracking.md#phase-3-以降の検討` に沿って追跡する。

### 5. Phase 2-8 への引き継ぎ準備（36週目後半）
**担当領域**: ドキュメント整備

5.1. **差分記録**
- Phase 2-4, 2-7 で実施した変更点・残項目を `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の前提セクションへ追記。
- 監査ログ/診断の安定化完了を `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`（新規）から参照できるよう脚注を整備。

5.2. **メトリクス更新**
- `0-3-audit-and-metrics.md` に CI pass_rate の推移と LSP テスト完了状況を記録。
- `tooling/ci/collect-iterator-audit-metrics.py` の集計結果を `reports/audit/dashboard/` に反映し、Phase 2-8 のベースラインとする。
- DIAG-003 Step5 で追加された `diagnostics.domain_coverage` / `diagnostics.plugin_bundle_ratio` / `diagnostics.effect_stage_consistency` をダッシュボードへ掲載し、`Plugin` / `Lsp` / `Capability` ドメインの Stage 連携が視覚化されるようグラフとしきい値を設計する（`docs/spec/3-6-core-diagnostics-audit.md` 脚注参照）。

**成果物**: 更新済み前提資料、メトリクス記録、Phase 2-8 用脚注



## 成果物と検証
- Windows/macOS CI で `ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` が 1.0 を維持し、監査欠落時にジョブが失敗すること。
- CLI `--format` / `--json-mode` の整合が取れており、テキスト・JSON 双方のゴールデンが更新済みであること。
- LSP V2 の互換テストが `npm run ci` および GitHub Actions `lsp-contract` で成功し、フィクスチャ差分がレポートとして残ること。
- 効果構文の PoC 実装を有効化した状態で `collect-iterator-audit-metrics.py --require-success` が `syntax.effect_construct_acceptance = 1.0`、`effects.syntax_poison_rate = 0.0` を満たし、CLI/LSP/監査ログに `effects.contract.*` 診断が出力されること。
- 技術的負債リストと関連レポートに最新状況が反映され、Phase 3 へ移送する項目が明確になっていること。

## リスクとフォローアップ
- CI 監査ゲート導入によるジョブ時間増大: 実行時間を監視し、10% 超過時はサンプル数の調整や並列化を検討。
- CLI フォーマット変更による開発者体験への影響: `reports/diagnostic-format-regression.md` で差分レビューを必須化し、顧客影響を評価。
- LSP V2 導入に伴うクライアント側調整: `tooling/lsp/compat/diagnostic_v1.ml` を一定期間維持し、互換性レイヤ廃止時のスケジュールを Phase 3 で検討。
- PARSER-003 Step5 連携: Packrat キャッシュ実装後に `effect.stage.*`／`effect.capabilities[*]` が欠落しないことを CI で確認するため、`tooling/ci/collect-iterator-audit-metrics.py --require-success` に Packrat 専用チェックを追加する（Stage 監査テストケースを新設）。  
- Recover 拡張: §3.4 で定義した Packrat カバレッジ・notes ローカライズ・ストリーミング重複検証を遅延させず実施する。`RunConfig.extensions["recover"].notes` を CLI/LSP 表示へ反映し、`Diagnostic.extensions["recover"]` の多言語テンプレートを `docs/spec/2-5-error.md` 脚注と同期させる。
- PARSER-003 Step6 連携: `Core_parse` モジュールのテレメトリ統合と Menhir 完全置換の是非を評価し、`parser.core_comb_rule_coverage` / `parser.packrat_cache_hit_ratio` を利用した監査ダッシュボード拡張を決定する。仕様更新時は `docs/spec/2-2-core-combinator.md` 脚注と `docs/guides/plugin-authoring.md` / `core-parse-streaming.md` の共有手順を再検証する。
- 効果構文の Stage 遷移: `syntax.effect_construct_acceptance` が 1.0 未満、または CLI/LSP で `-Zalgebraic-effects` の挙動が不一致になった場合は Phase 2-7 のクリティカルリスクとして即時エスカレーションする。Stage 遷移が遅延する場合、Phase 2-8 の仕様凍結に影響するため優先度を再評価する。

## 参考資料
- [2-4-diagnostics-audit-pipeline.md](2-4-diagnostics-audit-pipeline.md)
- [2-3-to-2-4-handover.md](2-3-to-2-4-handover.md)
- [2-5-spec-drift-remediation.md](2-5-spec-drift-remediation.md)
- [2-6-windows-support.md](2-6-windows-support.md)
- [compiler/ocaml/docs/technical-debt.md](../../../compiler/ocaml/docs/technical-debt.md)
- [reports/diagnostic-format-regression.md](../../../reports/diagnostic-format-regression.md)
- [reports/ffi-bridge-summary.md](../../../reports/ffi-bridge-summary.md)

[^streaming-flow-auto-phase27]: Phase 2-7 ストリーミング PoC の FlowController Auto ポリシー向け暫定脚注。`docs/guides/core-parse-streaming.md` §10 と `docs/spec/2-7-core-parse-streaming.md` のバックプレッシャ要件を突き合わせ、`max_lag`/`debounce`/`throttle` の既定域と CI での検証手順 (`collect-iterator-audit-metrics.py --section streaming`) を共有する。
