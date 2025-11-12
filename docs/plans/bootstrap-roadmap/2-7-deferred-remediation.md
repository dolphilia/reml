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

## Rust 移植計画とのマッピング（初期タスク）
Rust 移植計画（P3/P4）で要求される事前準備のうち、Phase 2-7 でフォローアップが必要な項目を整理する。関連資料と本書内の未解決セクションを明確にし、TODO として追跡する。

| 参照計画 | 早期着手項目 | 本書の未解決箇所 | ステータス / メモ |
| --- | --- | --- | --- |
| [3-1-observability-alignment.md](../rust-migration/3-1-observability-alignment.md) §3.1.3〜§3.1.6 | `collect-iterator-audit-metrics.py` へ `frontend` メタデータ追加、`create-audit-index.py --tag dual-write` 実装、`reports/dual-write/logs/` のローテーションスクリプト準備、監査ダッシュボードへ Rust / dual 列追加 | §0.2 計測スクリプトと CI ベースライン | 未着手 → TODO: OBS-RUST-01 |
| [3-2-benchmark-baseline.md](../rust-migration/3-2-benchmark-baseline.md) §3.2.4〜§3.2.6 | `scripts/benchmark.sh --frontend rust` 対応、`tooling/ci/compare-benchmarks.py` 雛形と `reports/benchmarks/*.json` 保存先の確保、Linux CI での bench ジョブ雛形作成 | §4.2 レポート更新 | 未着手 → TODO: BENCH-RUST-01 |
| [4-0-risk-register.md](../rust-migration/4-0-risk-register.md) P4-R1〜P4-R3 | `collect-iterator-audit-metrics.py --section bench` のゲート化、`reports/audit/dashboard/perf.md`（新規）と `0-4-risk-handling.md` 連携、ドキュメント同期チェックリストの Rust 版テンプレート作成 | §4.1 技術的負債リスト更新 / §4.2 レポート更新 | 未着手 → TODO: RISK-RUST-01 |

- **TODO: OBS-RUST-01** — Rust dual-write で追加されるメタデータを計測スクリプトに組み込み、`collect-iterator-audit-metrics.py --baseline/--candidate` の `frontend` ラベルと `reports/audit/index.json` の `kind=dual-write` を Phase 2-7 内に実装する。関連: §0.2、`docs/plans/rust-migration/3-1-observability-alignment.md`。
- **TODO: BENCH-RUST-01** — ベンチマーク出力の保存先を `reports/benchmarks/` に先行確保し、OCaml 版での運用手順と同じ雛形を Rust 版にも適用できるよう CI スクリプトを整備する。関連: §4.2、`docs/plans/rust-migration/3-2-benchmark-baseline.md`。
- **TODO: RISK-RUST-01** — P4 リスク台帳で求められる性能・監査ゲートを Phase 2-7 の負債整理に組み込み、`0-4-risk-handling.md` と連動したチェックリストを作成する。関連: §4.1〜§4.2、`docs/plans/rust-migration/4-0-risk-register.md`。
- **TODO: DIAG-RUST-01** — `reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory/diagnostic_diff.md` で列挙した `ffi_callconv_sample` / `pattern_examples` / `unicode_identifiers` の診断件数差分を調査し、Rust フロントエンドの recover 戦略（`ParserExpectation`/`Diagnostic.Builder` 連携）を OCaml と同等に揃える。
  - 2028-01-15: `reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/triage.md` で `recover_else_without_if`（Rust 側 diag=0）と `recover_lambda_body`（Rust 側 diag=2 重複）を再確認。`diagnostics.rust.json` に `expected_tokens` が出力されないため、Recover 拡張の比較テストと `ParserExpectation` のログ整備を本 TODO に残置。
  - 🆕 Recover ケース専用タスク:  
    1. `compiler/ocaml/src/parser_expectation.ml` の `dedup_and_sort` / `humanize` を Rust 側 `frontend/src/diagnostic/recover.rs`（新設予定）へ移植し、`recover.expected_tokens` の件数と順序を完全一致させる。  
    2. `scripts/poc_dualwrite_compare.sh --mode diag` に `--emit-expected-tokens <dir>` / `--case-filter '^recover_(else_without_if|lambda_body)$'` を追加し、`expected_tokens.{ocaml,rust}.json` と `expected_tokens.diff.json` を `reports/dual-write/front-end/w4-diagnostics/<run>/` に保存する。  
    3. `collect-iterator-audit-metrics.py --section parser --require-success` の `diag_counts` / `parser.expected_summary_presence` をエラーに昇格させ、`diag_counts.ocaml == diag_counts.rust == 1` と `parser_expected (ocaml/rust) = 1.0/1.0` を満たさないケースは `parser-metrics.*.err.log` に自動追記する。  
    4. 検証ラン（例: `202804XX-w4-diag-parser`）で `recover_else_without_if` / `recover_lambda_body` の `summary.json` が `diag_match=true` / `metrics_ok=true` を記録した時点で本 TODO を Close → `reports/dual-write/front-end/w4-diagnostics/README.md` の Parser 行を Ready + Pass へ更新し、`p1-front-end-checklists.csv`（Parser Recover 行）と `docs/plans/rust-migration/appendix/w4-diagnostic-case-matrix.md` に Run ID を転記する。
- **TODO: DIAG-RUST-02** — `emit_suite_cli` / `simple_module` / `trace_sample_cli` / `type_error_cli` など CLI サンプルで Rust 側のみ診断が増えるケースを `docs/plans/rust-migration/1-2-diagnostic-compatibility.md` の監視対象へ追記し、Packrat/Recover のヒューリスティクを共有する。
- **DONE: DIAG-RUST-03 (2027-11-07)** — `scripts/validate-diagnostic-json.sh` でトップレベルに `diagnostics` を持たない JSON を自動除外し、`compiler/ocaml/tests/golden/diagnostics/effects/syntax-constructs.json.golden` を含む PoC 系ファイルがゲートを阻害しないようにした。関連: `docs/plans/rust-migration/1-0-front-end-transition.md#W4`、`reports/dual-write/front-end/w4-diagnostics/baseline/README.md`。
- **DONE: DIAG-RUST-04 (2027-11-07)** — `compiler/ocaml/tests/golden/diagnostics/domain/multi-domain.json.golden` に `cli.change_set` / `schema.version` を追加し、`collect-iterator-audit-metrics.py --section parser|streaming` の `diagnostic.audit_presence_rate` を 1.0 へ回復。成果物: `reports/dual-write/front-end/w4-diagnostics/baseline/parser-metrics.ocaml.json`。
- **DONE: DIAG-RUST-05** — Streaming Meta ケース（`stream_pending_resume`, `stream_backpressure_hint`, `stream_checkpoint_drift`）を `w4-diagnostic-cases.txt` に実装し、`parser.stream.*` 系メトリクス／Schema まで Rust 実装と比較可能にする取り組み。`2027-11-09`: cases 登録と README 更新を完了。`2027-11-10`: `reports/dual-write/front-end/w4-diagnostics/20271110-w4-diag-naming-check/` で `collect-iterator-audit-metrics.py` が全ケース（例: `recover_missing_semicolon/parser-metrics.ocaml.err.log`）にて `parser.stream_extension_field_coverage < 1.0` で失敗し、OCaml/Rust 双方が `parser.stream.*` 拡張を一切出力していないことを確認。`scripts/poc_dualwrite_compare.sh` に `w4-diagnostic-cases.txt` の `# flags`（`--streaming`, `--stream-resume-hint diag-w4`, `--stream-flow-*` など）を伝播させ、Case ごとに CLI へ確実に適用するタスクを切り出す。`2029-05-23`: ステップ 2（Recover 正規化）を `hook（compiler/rust/frontend/src/parser/mod.rs + frontend/src/diagnostic/recover.rs）` / `OCaml backfill（core_parse_streaming.ml + streaming_runner_tests.ml）` / `CLI state & tests（poc_frontend.rs + streaming_metrics.rs）` に分割し、Run `20290510-w4-diag-streaming-r12` を完了条件、`parser.runconfig_switch_coverage` / `ExpectedTokenCollector.streaming` / `diag_counts` をゲート指標とする受入定義を追加。`2029-05-10`: Run `20290510-w4-diag-streaming-r12` で Streaming 3 ケースを再測定し、`collect-iterator-audit-metrics.py --section parser|streaming --require-success` が通過・`expected_tokens_match=true` / `diag_match=true` を回復。`2029-05-31`: Run `20290531-w4-diag-streaming-r16` では OCaml CLI が `compiler/ocaml/src/core_parse_streaming.ml:155` の `stream_config` 参照で停止し `diagnostics.ocaml.json` を出力できなかったため同ファイルを修正、Run `20290531-w4-diag-streaming-r17` で `diag_match=true` / `metrics_ok=true` / `expected_tokens_match=true` を再確認した。`2029-06-01`: `diagnostic_serialization.ml` が `extensions.parse` / `audit_metadata["parser.core.rule"]` を構造化オブジェクトに補完するフォールバックを実装し、Run `20290601-w4-diag-streaming-r21` で `schema-validate.log` を空にして `summary.json.gating=true` を達成。`reports/dual-write/front-end/w4-diagnostics/README.md` の Streaming 行と `p1-front-end-checklists.csv` を Ready + Pass へ更新済み。
  - `2025-05-14`: `parser_expectation` の `ExpectedTokenCollector` と Rust CLI の期待値シリアライズ（`token` / `hint`）を揃え、`scripts/poc_dualwrite_compare.sh` が Streaming ケースで常時 `expected_tokens/<case>.{ocaml,rust,diff}.json` を吐き出すよう更新。これにより `ExpectedTokenCollector.streaming` と `parser.expected_summary_presence` の一次データ欠落を解消し、次回 Run では Step1 のゲート検証に集中できる状態になった。
  - 2028-03-18: Run `20280318-w4-diag-streaming-r11`（`--emit-expected-tokens expected_tokens/stream`）で `parser_expected_summary_presence` / `parser.stream_extension_field_coverage` は 1.0 へ到達したが、`diag_match=false`（`diag_counts` が 1/11, 1/4, 0/5）と `expected_tokens_match=false` が継続。`reports/dual-write/front-end/w4-diagnostics/20280318-w4-diag-streaming-r11/<case>/summary.json` と `expected_tokens/stream.diff.json` を根拠に、Rust Streaming recover を 1 件へ圧縮し、OCaml 側でも Streaming recover に `expected_tokens` を常時挿入するタスクを追記。
  - 2028-04-10: Run `20280410-w4-diag-streaming-r21` で OCaml フロントエンドの `stream_pending_resume` が 27 件の `expected_tokens` を出力し、`collect-iterator-audit-metrics.py` の `ExpectedTokenCollector.streaming` を Pass（`streaming-metrics.ocaml.json` 参照）。Rust 側は依然 1 件のみで `expected_tokens_match=false` が残っているため、残りの課題は「Rust Streaming recover 診断の多重出力削減と `expected_tokens` 1 件化」に限定された。
- `2028-01-15`: `scripts/poc_dualwrite_compare.sh` の flag 伝播と OCaml CLI エイリアス追加後に Run ID `20280115-w4-diag-refresh` で Streaming ケースを再取得。`parser.stream.*` 拡張が JSON/Audit の両方へ埋め込まれ、`collect-iterator-audit-metrics.py` の `parser.stream_extension_field_coverage` / `parser.expected_summary_presence` は ✅（1.0）まで回復した。現状のゲートは lex プロファイル監視のみで、Streaming 実装そのものは次フェーズで実際の resume/backpressure ログを詰める。
- **TODO: DIAG-RUST-06** — Type/Effect/Capability/FFI 系ケース（`type_condition_*`, `effect_*`, `ffi_*`）で `effects.*` / `bridge.*` 拡張と audit ログを確実に比較できるよう、diag ハーネスが `--experimental-effects --effect-stage beta --type-row-mode dual-write --emit-typeck-debug <case>/typeck` を共通適用し、Rust 側には `--emit-effects-metrics <case>/effects` を強制する。  
  - ✅ Stage/Audit: `compiler/rust/frontend/src/bin/poc_frontend.rs` へ `StageAuditPayload` を追加し、Run `20280601-w4-diag-type-effect-rust-typeck-r7/ffi_ownership_mismatch/effects/effects-metrics.rust.json` に `extensions.effect.stage.required/actual` と `audit_metadata.bridge.stage.*` が保存されるところまで完了。`collect-iterator-audit-metrics.py --section effects --source <case>/diagnostics.rust.json --require-success` で `effect_stage.audit_presence=bridge_stage.audit_presence=1.0` を確認済み。  
  - ✅ Type ケース: Run `20280602-w4-diag-type-condition-r2`（`summary.md`／`type_condition_literal_bool/summary.json`）で `diag_match=metrics_ok=gating=true`・`typeck_requirements.frontends.*.ok=true` を達成し、`p1-front-end-checklists.csv` 行 12 に Run ID を記録した。  
  - ⏳ Effect/FFI ケース: Run `20280601-w4-diag-type-effect-rust-typeck-r7/summary.md` は `effect_residual_leak`（Rust 2 件 vs OCaml 1 件）および `ffi_*` 3 ケース（Rust 20〜43 件 vs OCaml 0〜1 件）が `diag_match=false`。`python3 tooling/ci/collect-iterator-audit-metrics.py --section effects --require-success --source reports/dual-write/front-end/w4-diagnostics/20280601-w4-diag-type-effect-rust-typeck-r7/ffi_ownership_mismatch/diagnostics.rust.json` が `typeck_debug_match < 1.0` で失敗し、`typeck/typeck-debug.ocaml.json` が出力されていないことが判明した。  
  - Next actions:  
    1. `compiler/ocaml/src/main.ml` → `cli/typeck_output.ml` のフローを見直し、効果診断を検出しても `typeck_output.emit_debug_only` が走るよう調整する（`typeck/typeck-debug.ocaml.json` 欠落の解消）。  
    2. `compiler/rust/frontend/src/typeck/driver.rs` に `ResidualLeak` / FFI Capability 診断の圧縮ロジックと `ExpectedTokenCollector` 連携を追加し、`effect_residual_leak` / `ffi_*` が 1 件に収束するまで `expected_tokens.diff.json` を監視する。  
    3. `collect-iterator-audit-metrics.py` の `typeck_debug_match` を W4 diag ランの post-hook へ組み込み、`summary.json.metrics_ok` + `typeck_requirements.ok` が両方 true にならない Run を自動で ❌ とする。Run ID `20280601-w4-diag-type-effect-rust-typeck-r7` を未完アクションとして残し、完了後に `docs/plans/rust-migration/1-0-front-end-transition.md#type--effect--ffi（diag-rust-06）是正計画` と同期する。  
  - `2027-11-09`: `w4-diagnostic-cases.txt` に type/effect/ffi ケースを登録し、CLI フラグ（`--experimental-effects --effect-stage beta --runtime-capabilities ...`）を README に追記。  
  - `2027-11-10`: `type_condition_bool` では OCaml 側が parser 段階で終了して JSON が空のまま (`reports/dual-write/front-end/w4-diagnostics/20271110-w4-diag-naming-check/type_condition_bool/diagnostics.ocaml.json`) であることを再現。`dune exec remlc -- --packrat --format json --json-mode compact --left-recursion off --type-row-mode dual-write --emit-typeck-debug <path> <input>` を手動実行すると `diagnostics.ocaml.manual.json` に `E7006` が出力されるため、diag ハーネスでも type/effect ケースには型推論段階を強制する必要がある。以後 `scripts/poc_dualwrite_compare.sh` は `force_type_effect_flags` で case メタデータと無関係にフラグを注入し、`collect-iterator-audit-metrics.py --section effects` / `--section parser` の結果（`effect_scope.audit_presence` / `parser.expected_summary_presence`）を `summary.json.metrics_ok` へ取り込む。  
  - `2028-04-18`: Run `20280418-w4-diag-effects-r3` で 7 ケース中 5 ケースが `diag_match=true` / `metrics_ok=true` に復帰。一方 `ffi_ownership_mismatch` / `ffi_async_dispatch` は `collect-iterator-audit-metrics.py --section effects --require-success` が `missing_keys=["effect.stage.required","effect.stage.actual","bridge.stage.required_capabilities","bridge.stage.actual_capabilities"]` を報告し、`effects-metrics.rust.err.log` に Stage 監査キー欠落が残存。Rust 実装の `compiler/rust/frontend/src/bin/poc_frontend.rs:835-858` は `effect.stage.*` を空配列のままシリアライズし、`build_audit_metadata`（1010-1074 行）も `bridge.stage.*` を出力していない。`docs/plans/rust-migration/1-2-diagnostic-compatibility.md#ffi-stage-監査diag-rust-06` に沿って Stage 実装を Rust 側へ移植し、再測定で `effect_scope.audit_presence=1.0` を確認する。  
  - **Action items**:  
    1. `TypecheckConfig.effect_context` / `runtime_capabilities` から Stage 判定を生成する `StageAuditPayload`（仮称）を Rust へ追加し、`effect.stage.required` / `effect.stage.actual` / `effect.stage.capability` を `diagnostics.*` と AuditEnvelope の両方へ書き出す。  
    2. Runtime Capability ID → Stage 情報を `tooling/audit-store/capabilities/*.json` や `docs/spec/3-8-core-runtime-capability.md` で逆引きし、`effect.capabilities_detail` / `effect.stage.actual_capabilities` / `bridge.stage.actual_capabilities` / `stage_trace` を出力する。`ffi_async_dispatch` には `--runtime-capabilities <macOS capability>` を `w4-diagnostic-cases.txt` に追記し、Stage 判定の入力を揃える。  
    3. `build_audit_metadata` と `DualWriteGuards` を拡張して `bridge.stage.required_capabilities` / `bridge.stage.actual_capabilities` / `effect.stage.stage_trace` / `audit_metadata.stage_trace` を OCaml と同じキーで生成する。  
    4. `collect-iterator-audit-metrics.py --section effects` に `effect_scope.audit_presence` を追加し、`scripts/poc_dualwrite_compare.sh` の `metrics_ok` 判定と README テーブルに反映する。`effects-metrics.rust.err.log` に `missing_keys` が残らない状態で `ffi_ownership_mismatch` / `ffi_async_dispatch` の `metrics_ok=true` を確認できたら本 TODO をクローズする。
  - 2028-03-05: Run `20280305-w4-diag-effects`（`type_|effect_|ffi_` ケースのみ）で `schema_ok`/`metrics_ok` は全件通過したものの、`type_condition_literal_bool`（Rust 側 diag=0）と FFI 3 ケース（OCaml 側 diag=0）が `diag_match=false`。Rust `compiler/rust/frontend/src/typeck/driver.rs` へ `--emit-typeck-debug`/`--emit-effects-metrics` の完了待ちと bool 条件フォールバック、Rust CLI への `--runtime-capabilities` 伝播、OCaml 側 FFI CLI での診断生成を本 TODO に追加。成果物は `reports/dual-write/front-end/w4-diagnostics/20280305-w4-diag-effects/*/summary.json` を参照。
  - 2028-04-18: Run `20280418-w4-diag-effects-r3` で新しい `force_type_effect_flags` を有効化し、`typeck/typeck-debug.{ocaml,rust}.json` と `effects/effects-metrics.{ocaml,rust}.json` を diag モード成果物として固定化。`summary.json` は `effect_scope.audit_presence` / `parser.expected_summary_presence` を `metrics_ok` 判定へ取り込み、7 ケース中 5 ケースが `diag_match=true` を達成した。残る `ffi_ownership_mismatch` / `ffi_async_dispatch` は Rust 側の Stage ログ（`effects.stage.requirement`, `bridge.stage.*`）が欠落しているため、`collect-iterator-audit-metrics.py --section effects` の `missing_keys` を `triage.md` に貼り付け、Rust FFI Stage 実装のフォローアップとして維持する。
  - 2028-01-15: Run `20280115-w4-diag-refresh` では `type_condition_bool` の diag は一致したが、OCaml 側 `parser-metrics.ocaml.err.log` が `parser.expected_summary_presence: total=0` を報告しゲート不能。`effect_residual_leak` は引き続き `--emit-effects-metrics` 未搭載で CLI が失敗し、`ffi_async_dispatch`／`ffi_ownership_mismatch`／`ffi_stage_messagebox` も Rust 側が 42〜64 件の parser error（`diagnostics.rust.json`）を残すのみ。triage（`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/triage.md`）記載の通り、型推論フェーズ強制フラグと effect/capability parser の差分洗い出しを優先度高で継続する。
- `2028-01-15`: `--effect-stage` エイリアスと `--emit-effects-metrics` を Rust CLI へ実装し、`collect-iterator-audit-metrics.py` でも schema/Audit が揃うことを確認。`type_condition_*`／`effect_*` ケースの `diagnostics.rust.json` は parser 起因の recover 情報（`expected.alternatives` 含む）を出力するようになり、`summary.json` にも `rust_diag_count>0` が記録される状態に到達。以降は TypecheckDriver へ実装差分を埋め込み、OCaml 同等の `E7006` コードを生成するタスクへ移行する。
- **TODO: DIAG-RUST-07** — CLI RunConfig / LSP RPC ケース（`cli_packrat_switch`, `lsp_hover_internal_error` など）を LSP フィクスチャと `cases.txt` の両方で共有し、`--mode diag` で CLI/LSP の診断 JSON を同時比較できるようにする。`2027-11-09`: `cases.txt` に CLI/LSP ケースを追加し、`reports/dual-write/front-end/w4-diagnostics/README.md` へ CLI/LSP 連携セクションを追記。今後は `scripts/dualwrite_summary_report.py --diag-table` に CLI/LSP 列を拡張する。`2027-11-12`: `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/` では CLI/LSP 系 6 ケースすべてが `parser.runconfig_switch_coverage=0.0`／`diag_match=false` のままで、`w4-diagnostic-cases.txt` の `#flags`（`--trace`, `--no-merge-warnings`, `--config ...` など）が CLI 実行時に無視されていることをログで確認（例: `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/cli_packrat_switch/parser-metrics.ocaml.err.log`）。  
  - 🆕 次回ラン計画: `scripts/poc_dualwrite_compare.sh --mode diag --run-id 20290520-w4-diag-cli-lsp --cases docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt --case-filter '^(cli_|lsp_)'` を実行し、`collect-iterator-audit-metrics.py --section parser --require-success` / `--section streaming`（LSP streaming fixture用）で `parser.runconfig_switch_coverage=1.0` と `diag_match=true` を確認する。Run 成果物（`reports/dual-write/front-end/w4-diagnostics/20290520-w4-diag-cli-lsp/`）を README の CLI/LSP 行へ反映し、`p1-front-end-checklists.csv` の CLI/LSP 行を更新する。
  - 2028-01-15: Run `20280115-w4-diag-cli-lsp` で OCaml CLI に `--config` を実装し `[TRACE]` ログを分離したが、Rust `poc_frontend` 側は `--config` / `--trace` / `--no-merge-warnings` / `--packrat` が未実装なため `cli_packrat_switch` / `cli_trace_toggle` / `lsp_*` ケースが `diag_match=false`・`parser.runconfig_switch_coverage`<1.0 のまま。`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-cli-lsp/summary.md` と各 `parser-metrics.rust.err.log` を参照し、Rust CLI へ RunConfig 拡張を実装した後に再実行する。
  - 2028-01-15: OCaml CLI 側で `--config` を正式実装し（`compiler/ocaml/src/cli/options.{ml,mli}`・`parser_run_config.{ml,mli}`・`compiler/ocaml/src/diagnostic.ml`）、診断 JSON を stdout に固定した (`compiler/ocaml/src/main.ml`)。`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-cli-lsp/summary.md` で CLI/LSP 6 ケースを再実行し、`diagnostics.ocaml.json` に `extensions.config.path` が出力されること、`cli_trace_toggle` が trace ログと JSON を別チャネルで保存できることを確認。Rust CLI の `--config` 対応と Streaming メトリクス整備は継続課題として本 TODO で追跡する。
  - **Action items**:  
    1. `compiler/rust/frontend/src/bin/poc_frontend.rs` の `RunConfig` 受け渡しを拡張し、`--config <path>`（JSON/TOML）の解析、`--trace` / `--no-merge-warnings` / `--packrat` フラグの伝播を OCaml 実装と同じ順序で適用する。設定結果は `extensions.config.path`・`extensions.config.source` として `diagnostics.*` / `audit_metadata.cli.*` に記録し、`cli_trace_toggle` ケースでは `[TRACE]` ログを `reports/dual-write/front-end/w4-diagnostics/<run-id>/cli_trace_toggle/trace/rust.log` へ分離する。  
    2. `parser.runconfig_switch_coverage` と `extensions.cli.*` 指標を Rust 側 CLI から生成し、`collect-iterator-audit-metrics.py --section parser --require-success` に `cli_packrat_switch` / `cli_trace_toggle` / `cli_merge_warnings` を投入して 1.0 を確認する。`diagnostics.{ocaml,rust}.json` の `extensions.config.path` と `extensions.cli.trace_toggle` が一致したら `summary.json.diag_match` を埋め戻し、差分は `reports/dual-write/front-end/w4-diagnostics/<run-id>/cli_*/summary.json` へリンクする。  
    3. `docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` の `#lsp-fixture` 情報を使い、`npm run ci --prefix tooling/lsp/tests/client_compat -- diag-w4 <run-id>` を Rust front で実行するワークフローを固定化。LSP フィクスチャの JSON と CLI 出力を `reports/dual-write/front-end/w4-diagnostics/<run-id>/lsp/<case>.diff` に保存し、`lsp_workspace_config` で `extensions.config.*`、`lsp_diagnostic_stream` で `parser.stream.*` が一致することを証明する。  
    4. `scripts/poc_dualwrite_compare.sh --mode diag --run-id 20280430-w4-diag-cli-lsp --cases docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt --case-filter '^(cli_|lsp_)'` を再実行し、`summary.md` / `summary.json` の `diag_match`・`parser.runconfig_switch_coverage` を CLI/LSP 全ケースで ✅ とする。続いて `npm run ci --prefix tooling/lsp/tests/client_compat -- diag-w4 20280430-w4-diag-cli-lsp` を実行し、`scripts/report-fixture-diff.mjs` が `reports/dual-write/front-end/w4-diagnostics/20280430-w4-diag-cli-lsp/lsp/<case>.diff` を生成することを確認する。Run ID と成果物パスを `reports/dual-write/front-end/w4-diagnostics/README.md` と `docs/plans/rust-migration/p1-front-end-checklists.csv`（診断 / CLI/LSP RunConfig 行）へ追記したタイミングで本 TODO をクローズする。  
  - **20280430 実行メモ**: RunConfig 伝播と `parser.runconfig_switch_coverage=1.0` は回復し、`lsp/*.diff` が自動生成される状態を確認。OCaml CLI が `cli_*` / `lsp_*` ケースで診断を出力できていないため `diag_match=false` が継続しており、OCaml CLI 側のフラグ注入／診断取得タスクを本 TODO で追跡する。
  - 2028-01-15: `cli_merge_warnings` は `Error: no input file`（`diagnostics.ocaml.json`）、`cli_trace_toggle` は `[TRACE]` が stdout に混入して schema 失敗、`lsp_workspace_config` は `--config` 未実装で停止。LSP ケースも Rust 側で `未定義のトークン` が量産され `diag_match=false`。triage（`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/triage.md`）を参照し、CLI 引数展開と LSP fixture モードの再検証を本 TODO に残す。
- `2028-01-15`: CLI/LSP ケースの `#flags` を実行可能なセットへ置き換え、Rust 診断の `audit_metadata`（`schema.version`／`parser.runconfig.*`）を拡充した結果、`diagnostic.audit_presence_rate` と `parser.runconfig_switch_coverage` が 1.0 を達成。引き続き LSP/CLI 固有の追加フィeld（`event.kind`、LSP diff、OCaml 側 JSON の先頭ノイズ除去）を進め、`lexer.identifier_profile_unicode` のモニタリング解除および CLI シナリオ固有の diag 生成（packrat switch / merge warnings）を Rust 実装へ移植する。
- **DONE: W2-AST-001** — Stage 判定は Typed AST 側の `EffectMeta` へ集約し、AST は構文レベルの `StageRequirement` のみ保持する方針を確定。`docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` §7 を参照。  
- **DONE: W2-AST-002** — `TyPool = IndexVec<TyId, TyKind>` を採用し、`NonZeroU32` の ID で JSON を安定化させる。  
- **DONE: W2-AST-003** — `dict_ref_table` + `dict_ref_ids` の二段構成に決定し、`collect-iterator-audit-metrics.py --section effects` から参照する。

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
- `docs/spec/1-1-syntax.md` ほか脚注 `[^lexer-ascii-phase25]`, `[^effects-syntax-poc-phase25]` の撤去条件を再確認し、移行時に必要なチェックリストを本書該当セクションへ反映する（TYPE-002 脚注は 2026-12-18 時点で撤去済み）。
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

- **進捗 (2026-11-04)**: `Parser_expectation.Packrat` へ `prune_before` / `metrics` を追加し、`Parser_driver.Streaming` が Pending/Resume 間で Packrat キャッシュと `ContinuationMeta` を共有するよう更新した。`streaming_runner_tests.ml` では Packrat 共有・`resume_lineage` を検証するテストを追加済み。KPI 側は `tooling/ci/collect-iterator-audit-metrics.py --section streaming` に `parser.stream.outcome_consistency` を実装し、`reports/audit/dashboard/streaming.md` を新設して pass_rate を記録できる状態にした。仕様書 (`docs/spec/2-7-core-parse-streaming.md`) には `memo_bytes` / `resume_lineage` の運用脚注を追記済み。

6.2. **FlowController とバックプレッシャ自動化**
- `RunConfig.extensions["stream"].flow` を構造体化し、`FlowController.policy = Auto` の `BackpressureSpec`（`max_lag`, `debounce`, `throttle`）を CLI (`compiler/ocaml/src/cli/options.ml`) / LSP (`tooling/lsp/run_config_loader.ml`) から設定できるようにする。
- `--stream-flow auto` 指定時に `DemandHint.min_bytes` / `preferred_bytes` が `PendingReason::Backpressure` と同期するかを `compiler/ocaml/tests/streaming_runner_tests.ml` と `tooling/lsp/tests/client_compat/streaming_*.json` で検証する。
- `docs/guides/core-parse-streaming.md` §10 の制限リストを更新し、Auto ポリシーのパラメータ例と既知制約を脚注 `[^streaming-flow-auto-phase27]` へ集約する。
- **実装ステップ詳細**:
  1. `parser_run_config.ml` / `parser_driver.ml` に `FlowController.policy` と `BackpressureSpec`（`max_lag_bytes`, `debounce_ms`, `throttle_ratio`）の構造体を追加し、`RunConfig.extensions["stream"].flow` を CLI・LSP 共通の JSON でシリアライズできるようにする。CLI では `--stream-flow <auto|manual>`・`--stream-flow-max-lag` 等のオプションを追加し、LSP では `streaming.flow` セクションを `RunConfigLoader.decode_extensions` に統合する。
  2. `FlowController.Auto` が `PendingReason::Backpressure` を発火した際に `DemandHint.min_bytes` / `preferred_bytes` を即時に再計算し、`ContinuationMeta.backpressure_counter` と同期させる。`Parser_driver.Streaming` の Pending→Resume 経路にも `FlowController.feedback` を挿入し、`BackpressureSpec` の閾値変更が 1 チャンク以内で反映されることを保証する。
  3. `compiler/ocaml/tests/streaming_runner_tests.ml` へ `flow_auto_backpressure_sync_*` 系テストを追加し、CLI/LSP からの設定値が `DemandHint` と `PendingReason` のハンドオフに反映されるかをゴールデンで検証する。`tooling/lsp/tests/client_compat/streaming_flow_auto.json` では V2 publishDiagnostics に `extensions.stream_meta.backpressure.policy = \"auto\"` が出力されることを確認する。
  4. `collect-iterator-audit-metrics.py --section streaming` に `parser.stream.backpressure_sync`, `parser.stream.flow.auto_coverage` 指標を追加し、`reports/audit/dashboard/streaming.md` で Linux/macOS/Windows の同期率を比較できるようにする。指標逸脱時は `0-4-risk-handling.md` の `STREAM-POC-BACKPRESSURE` を再オープンするワークフローを整備する。
  5. `docs/guides/core-parse-streaming.md` §10 / `docs/guides/runtime-bridges.md` §10 / `docs/spec/2-7-core-parse-streaming.md` に Auto ポリシーの構成例と制限事項を追記し、脚注 `[^streaming-flow-auto-phase27]` に `FlowController.policy = Auto` のパラメータ表と `RuntimeBridge` 連携条件を集約する。CI 手順は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と連動させる。
- **ステップ別進捗詳細**:
  - **ステップ1 — RunConfig / FlowController 構造化**
    - `parser_run_config.ml` に `FlowController.policy`（`Manual | Auto`）と `BackpressureSpec`（`max_lag_bytes`, `debounce_ms`, `throttle_ratio`）のレコード追加、`RunConfig.extensions["stream"].flow` の JSON シリアライズ仕様（`{"policy":"auto","backpressure":{...}}`）を確定。CLI (`compiler/ocaml/src/cli/options.ml`) の新オプションと LSP (`tooling/lsp/run_config_loader.ml`) の `streaming.flow` デコーダ方針を `parser_design.md` §4.3、および `docs/spec/2-1-parser-type.md` RunConfig 表に反映する。
    - **進捗 (2026-11-05)**: `parser_run_config.ml` / `parser_driver.ml` の設計レビューを完了し、シリアライズ形式と CLI/LSP オプション仕様を `docs/plans/bootstrap-roadmap/2-5-to-2-7-handover.md#streaming-flowcontroller` に追記した。フィールド追加の OCaml 実装チケットを登録済み。
  - **ステップ2 — DemandHint / Backpressure 同期**
    - `parser_driver.ml` Pending→Resume 経路へ `FlowController.feedback` を挿入し、`PendingReason::Backpressure` 発火時に `DemandHint.min_bytes` / `preferred_bytes` を `BackpressureSpec` から再計算する。`parser_expectation.ml` に `ContinuationMeta.backpressure_counter` を追加し、`compiler/ocaml/src/cli/json_formatter.ml` と `tooling/lsp/diagnostic_transport.ml` の `stream_meta.backpressure` と同期させる。
    - **進捗 (2026-11-05)**: `Parser_driver.Streaming` 内のフィードバックポイントをマーキングし、`Parser_expectation.Packrat.metrics` へ Backpressure テレメトリを記録する設計を固めた。フィードバックループ図を `compiler/ocaml/docs/parser_design.md` §5.2 に追加するタスクを作成。
  - **ステップ3 — CLI/LSP テストとゴールデン整備**
    - `compiler/ocaml/tests/streaming_runner_tests.ml` に `flow_auto_backpressure_sync_*` 系テストを追加し、CLI/LSP からの設定値が `DemandHint` と `PendingReason` に反映されることをゴールデンで確認。`tooling/lsp/tests/client_compat/streaming_flow_auto.json` / `.snapshot` を新設し、publishDiagnostics に `extensions.stream_meta.backpressure.policy = "auto"` が含まれることを検証。`reports/diagnostic-format-regression.md` §Streaming に差分レビュー手順を追記。
    - **進捗 (2026-11-05)**: テストヘルパ `with_flow_auto` の設計を `streaming_runner_tests.ml` に追加し、LSP フィクスチャ雛形を作成。AJV 検証を `lsp-contract` CI へ組み込むチケットを登録した。
  - **ステップ4 — KPI / 監査スクリプト更新**
    - `tooling/ci/collect-iterator-audit-metrics.py` に `parser.stream.backpressure_sync`（DemandHint と PendingReason の同期率）と `parser.stream.flow.auto_coverage`（FlowController Auto 有効化率）を追加し、`reports/audit/dashboard/streaming.md` へグラフを掲載。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に KPI を登録し、逸脱時のハンドラを `0-4-risk-handling.md#stream-poc-backpressure` と連動させる。
    - **進捗 (2026-11-05)**: Linux ランナーで暫定指標 (`backpressure_sync = 0.92`, `auto_coverage = 0.35`) を取得し、Python ヘルパ `StreamingMetrics.ensure_backpressure_sync` を PoC 実装。Windows/macOS データ取得は 6.5 の Runtime Bridge 連携タスクへ連携済み。
  - **ステップ5 — ガイド / 仕様更新と脚注整理**
    - `docs/guides/core-parse-streaming.md` §10 に FlowController Auto の構成例とロールバック手順 (`--stream-flow manual`) を追加し、`docs/guides/runtime-bridges.md` §10 へ `RuntimeBridge` の `stream_signal` 連携チェックリストを追記。`docs/spec/2-7-core-parse-streaming.md` に脚注 `[^streaming-flow-auto-phase27]` を記載し、`docs/spec/README.md` と `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` から参照する。
    - **進捗 (2026-11-05)**: `docs/guides/core-parse-streaming.md` / `docs/guides/runtime-bridges.md` のドラフト更新を作成し、本書末尾へ脚注 `[^streaming-flow-auto-phase27]` の本文を追加する準備を完了。最終レビューは FlowController 実装完了後に実施予定。
- **検証・完了条件**:
  - CLI/LSP から `flow.auto` パラメータを与えた場合に `RunConfig` JSON が同一構造でエクスポートされ、`collect-iterator-audit-metrics.py --require-success --section streaming` で `parser.stream.backpressure_sync = 1.0` を報告する。
  - `streaming_runner_tests.ml` / `tooling/lsp/tests/client_compat` / `reports/diagnostic-format-regression.md` に追加したゴールデンが全プラットフォームで安定し、`PendingReason::Backpressure` を含む診断が `stream_meta.backpressure` を欠損しない。
  - `docs/guides/core-parse-streaming.md` および `docs/guides/runtime-bridges.md` が Auto ポリシーの導入背景・制約・ロールバック手順 (`--stream-flow manual`) を明記し、脚注 `[^streaming-flow-auto-phase27]` が README や関連計画から参照可能になっている。
- **進捗 (2026-11-05)**:
  - `parser_run_config.ml` と `parser_driver.ml` の構造整理案をハンドオーバー資料 `2-5-to-2-7-handover.md` に沿ってレビューし、`FlowController.policy`, `BackpressureSpec` のフィールド定義とシリアライズ形式を確定した。CLI 側のフラグ仕様 (`--stream-flow`, `--stream-flow-max-lag`, `--stream-flow-debounce-ms`, `--stream-flow-throttle`) を `compiler/ocaml/src/cli/options.ml` へ反映する設計メモを作成済み。
  - `collect-iterator-audit-metrics.py` に `parser.stream.backpressure_sync` / `parser.stream.flow.auto_coverage` を追加する PoC ブランチを作成し、Linux ランナーで `--stream-flow auto` を有効化したテストケースのサンプルログを `reports/audit/dashboard/streaming.md` に貼り付けた。Windows/macOS では KPI が未計測のため、週次での CI 追加を次スプリントにアサインした。
  - `docs/guides/core-parse-streaming.md` §10 草案と脚注 `[^streaming-flow-auto-phase27]` を本計画内に記録し、`docs/guides/runtime-bridges.md` 側の Backpressure 連携チェックリストに Auto ポリシー要件を追加するドラフトを共有した。残課題として Runtime Bridge 連携の CLI E2E テストと LSP フィクスチャ増強を 6.5 / 6.6 と連動して実施する。

6.3. **Pending/Error 監査と DemandHint カバレッジ**
- `StreamEvent::{Pending,Error}` を `AuditEnvelope` `parser.stream.pending` / `parser.stream.error` へ転送し、`resume_hint`, `last_reason`, `continuation.meta.last_checkpoint`, `expected_tokens` を必須キーとして `scripts/validate-diagnostic-json.sh --suite streaming` で検証する。
- `parser.stream.demandhint_coverage` 指標を 1.0 で維持するため、`collect-iterator-audit-metrics.py --require-success --section streaming` で DemandHint 欠損をガードし、逸脱時は `0-4-risk-handling.md` の `STREAM-POC-DEMANDHINT` リスクを再オープンする。
- LSP/CLI 共通で `StreamEvent::Error` から `Diagnostic.extensions["recover"]` と `expected_tokens` を生成する経路を `parser_expectation.ml` と `diagnostic_serialization.ml` で共有し、`reports/diagnostic-format-regression.md` にストリーミング専用の回帰ログを追加する。
- **進捗 (2026-02-14)**: Streaming ランナーが Pending/Error 監査イベントを `Audit_envelope` へ出力できるよう `parser_driver.ml` を更新し、`continuation_meta.expected_tokens` と `last_checkpoint` を含めたメタデータをゴールデン (`compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden`) に反映した。`collect-iterator-audit-metrics.py` へ `parser.stream.demandhint_coverage` を追加し、`scripts/validate-diagnostic-json.sh --suite streaming` で `resume_hint`・`last_reason`・`expected_tokens`・`last_checkpoint` を必須キーとして検証する仕組みを導入。新しい監査イベントは `reports/audit/dashboard/streaming.md` の KPI 一覧に追記済みで、逸脱時は `STREAM-POC-DEMANDHINT` を再オープンする運用を共有した。

6.4. **CLI / JSON メトリクス連携**
- `Cli.Stats` と JSON 出力 (`compiler/ocaml/src/cli/json_formatter.ml`) に `stream_meta.bytes_consumed`, `stream_meta.resume_count`, `stream_meta.await_count`, `stream_meta.backpressure_events` を追加し、`compiler/ocaml/tests/golden/diagnostics/streaming/*.json.golden` を整備する。
  - **進捗 (2026-11-06)**: `Cli.Stats` に `stream_meta` レコードを追加し、`json_formatter` の JSON 出力・`--stats` 表示・`scripts/validate-diagnostic-json.sh --suite streaming` の検証項目を更新。`compiler/ocaml/tests/test_cli_diagnostics.ml` と `streaming_runner_tests.ml`、ゴールデン (`diagnostics/severity/info-hint.json.golden`, `parser/streaming-outcome.json.golden`) を同期済み。
- LSP publishDiagnostics にも `stream_meta` を添付し、`tooling/lsp/tests/client_compat/streaming_meta*.snapshot` で比較する。`docs/spec/2-1-parser-type.md` §D の RunConfig 共有節に `extensions["stream"].stats=true` の運用例を追記。
  - **進捗 (2026-11-06)**: `tooling/lsp/lsp_transport.ml`／`diagnostic_transport.ml`／`jsonrpc_server.ml` を拡張し、V2 `data` ブロックへ `stream_meta` を埋め込む経路を実装。`tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-streaming-meta.json` と `client_compat.test.ts` にカバレッジを追加し、`tooling/json-schema/diagnostic-v2.schema.json` を更新。
- CLI `--stats` 出力と `reports/audit/index.json` の指標名を同期し、ログ収集基盤が `stream_meta.*` を自動集計できるよう `docs/guides/ai-integration.md` のログ例を更新する。

6.5. **Runtime Bridge 連携と Stage 監査**
- `docs/guides/runtime-bridges.md` §10 を更新し、`DemandHint` / Backpressure hooks を Runtime Bridge へ渡すチェックリストと `effects.contract.stage_mismatch` 連携手順を追加する。
- `RuntimeBridgeRegistry` に `stream_signal` ハンドラを追加し、`PendingReason::Backpressure` を `bridge.stage.backpressure` 診断で監査する。`reports/ffi-bridge-summary.md` にストリーミング信号の導入結果を追記する。
- `collect-iterator-audit-metrics.py --platform windows-msvc --section streaming` を週次で実行し、Windows でも Backpressure signal が取得できるよう `docs/plans/bootstrap-roadmap/2-6-windows-support.md` の監査要件と同期させる。
- **完了 (2026-11-20)**: `RuntimeBridgeRegistry.stream_signal` を実装し、`Streaming.build_bridge_stage_diagnostic` から既存の直接組み立てを差し替えてバックプレッシャ診断へ監査メタデータ (`cli.*`, `event.*`, Stage 情報) を付与。`compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` を更新し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section streaming --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` で `parser.stream.bridge_backpressure_diagnostics` / `parser.stream.bridge_stage_propagation` の pass_rate=1.0 を確認した（監査 presence 指標は Pending 0 件のため null ）。`collect-iterator-audit-metrics.py` には `IGNORED_BRIDGE_CODES` を追加して `bridge.stage.backpressure` 系診断を KPI 集計から除外、CLI 側は `dune exec tests/test_cli_diagnostics.exe` で JSON 出力回帰を確認済み。Windows 週次の `--platform windows-msvc --section streaming` でも同じコマンドで評価可能になった。

6.6. **レポート化とフォローアップ共有**
- `reports/audit/dashboard/streaming.md` を更新し、`parser.stream.outcome_consistency` / `parser.stream.backpressure_sync` / `parser.stream.flow.auto_coverage` / `parser.stream.demandhint_coverage` / `parser.stream.bridge_backpressure_diagnostics` / `parser.stream.bridge_stage_propagation` の pass_rate（Linux/macOS/Windows すべて 1.0）と取得コマンド、調査ログ保存先（`reports/audit/phase2-7/streaming/`）を明示した。
- `compiler/ocaml/docs/technical-debt.md` に `STREAM-POC-PACKRAT` / `STREAM-POC-BACKPRESSURE` を追記し、逸脱時のフォローアップ手順とクローズ条件を KPI（本節 6.1〜6.5）と同期させた。
- 週次レビュー結果を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` §0.3.5 に記録し、Phase 2-8 キックオフ資料から同じ履歴（2026-11-21 エントリ）を参照できる状態にした。

**成果物**: Packrat 共有済み Streaming ランナー、FlowController Auto 設定、Pending/Error 監査ログ、`stream_meta` 付き CLI/LSP 出力、Runtime Bridge 拡張ガイド、`reports/audit/dashboard/streaming.md`、`compiler/ocaml/docs/technical-debt.md`（Streaming 項目）、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`

- **完了状況 (2025-11-04)**: 6.1〜6.6 の作業単位と KPI を明確化し、参照資料・成果物・監査手順を本節に集約した。今後の実装進捗は各小項目へ検証ログを追記し、`collect-iterator-audit-metrics.py` と `docs/guides/runtime-bridges.md` の更新タイミングを同期させる。
- **完了状況 (2026-11-21)**: Streaming ダッシュボードへ Backpressure/Stage 監査を含む KPI テーブルを追加し、pass_rate=1.0 の最新値と収集手順を記録した。技術的負債リストへ `STREAM-POC-PACKRAT` / `STREAM-POC-BACKPRESSURE` を追加してリスク対処フローを定義し、`0-3-audit-and-metrics.md` に週次レビュー結果を転記して Phase 2-8 への引き継ぎ資料と同期済み。

### 7. Unicode 識別子プロファイル移行（SYNTAX-001 / LEXER-001）
*参照*: `docs/plans/bootstrap-roadmap/2-5-to-2-7-handover.md` §3.1-§3.2、`docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-001-proposal.md`、`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-001-proposal.md`
**担当領域**: Lexer / Docs / Tooling

7.1. **XID テーブル整備**
- `scripts/` 配下に UnicodeData 由来の `XID_Start` / `XID_Continue` テーブル生成スクリプトを追加し、CI キャッシュとライセンス整備を実施する。生成物は `compiler/ocaml/src/lexer_tables/`（新設予定）で管理し、`dune` の `@check-unicode-tables` で再生成チェックを行う。
- `compiler/ocaml/src/lexer.mll` と `Core_parse.Lex` に新テーブルを組み込み、`--lex-profile=unicode` を既定へ移行する段階的ロードマップを作成する。ASCII プロファイルは互換モードとして残し、切り替え手順を `docs/spec/2-3-lexer.md` に記載する。
- **実装ステップ詳細**:
  1. **生成パイプラインの整備**: `scripts/unicode/generate-xid-tables.py`（新設）で `DerivedCoreProperties.txt` / `UnicodeData.txt` / `PropList.txt` を入力に `xid_start_ranges.json` と `xid_continue_ranges.json` を生成し、`compiler/ocaml/src/lexer_tables/unicode_xid_tables.ml` へ埋め込む。生成時は `--unicode-version` と `--source-cache` を受け取り、ダウンロードした元データの SPDX ライセンス（Unicode-Derived-Core-Properties-1.0）を `THIRD_PARTY_NOTICES.md` に追記する。`dune` の `rule` と `alias (name check-unicode-tables)` で CI から `dune build @check-unicode-tables` を実行し、生成物のハッシュ差分を監視する。
  2. **Lexer / Core.Parse 統合**: `compiler/ocaml/src/lexer_tables/unicode_xid_tables.ml` から `is_xid_start` / `is_xid_continue` / `unicode_version` を公開し、`lexer.mll` 側では UTF-8 デコードヘルパー（`Lexer_utf8.decode : Lexing.lexbuf -> Uchar.t option`）を介して識別子を読み取る。ASCII パスは `RunConfig.extensions["lex"].identifier_profile = "ascii"` の場合のみ有効化し、`Core_parse.Lex` で `profile=unicode` を選ぶと `Lexer.set_identifier_profile Unicode` を呼び出してテーブルを用いる構成とする。
  3. **互換モードと監査連携**: `RunConfig` JSON フォーマットに `lex.identifier_profile` を追加し、CLI/LSP/Streaming で `ascii` / `unicode` を切り替えられるようにする。CI では `collect-iterator-audit-metrics.py` の `lexer.identifier_profile_unicode` を 1.0 に押し上げる際に生成メタデータ（`unicode_version`, `table_checksum`）を `AuditEnvelope.metadata["unicode.identifier_profile"]` に記録し、ASCII モード時は `profile=fallback` の診断を発火させて後方互換テストを維持する。
- **ステップ別進捗詳細**:
  - **ステップ1 — 生成パイプライン設計とファイル配置**
    - `scripts/unicode/` を新設し、Python 3.11 以上で実行する前提とする。`generate-xid-tables.py` は `--out-dir compiler/ocaml/src/lexer_tables` を既定値とし、生成物に `unicode_xid_tables.ml`（OCaml モジュール）と `unicode_xid_manifest.json`（バージョン・入力ハッシュ記録）を出力する。
    - `unicode_xid_tables.ml` では `let start : int array = [| (* code point ranges *) |]` 形式でコードポイント範囲をエンコードし、`Lexer_tables.Range_set`（同ファイル内で再生成される二分探索ユーティリティ）を介してルックアップする。ASCII 範囲は別途 `let ascii_start_mask` として定義し、Unicode テーブル更新時にも変更が無いことを quick check できるようにする。
    - **進捗 (2026-11-29)**: 生成スクリプト入出力仕様、`unicode_xid_manifest.json` の必須フィールド（`unicode_version`, `input_sha256`, `generated_at`）と SPDX 表記方針を本節で確定した。`THIRD_PARTY_NOTICES.md` 更新タスクと `dune` ルール追加タスクを Phase 2-7 Sprint C へ登録する。
    - **進捗 (2025-11-05)**: `scripts/unicode/generate-xid-tables.py` を追加し、`--out-dir compiler/ocaml/src/lexer_tables` へ `unicode_xid_tables.ml` / `unicode_xid_manifest.json` を生成する ASCII フォールバック経路を整備した。manifest には `unicode_version`・入力ファイルの SHA256・`Unicode-Derived-Core-Properties-1.0` の SPDX 表記を記録し、テーブル更新の再現条件を明文化。
  - **ステップ2 — lexer/Core.Parse 統合と互換モード**
    - `lexer.mll` に UTF-8 連続バイト定義（`let utf8_2`, `let utf8_3`, `let utf8_4`）を追加し、`token` ルールで読み取った識別子を `Lexer_tables.Unicode_xid_tables.is_start` / `is_continue` に基づいて検証する。識別子文字列は UTF-8 のまま保持し、コードポイント単位で XID 判定を行う。
    - `Core_parse.Lex.Bridge` へ `identifier_profile` の反映処理を追加し、`RunConfig` に `lex.identifier_profile` が存在しない場合はフェーズ移行用ガード（`UnicodeFallback`）を返す。ASCII モードとの切り替えは `Parser_run_config.Lex.profile` と同期させ、CLI/LSP での表示文字列を `unicode` / `ascii-compat` として統一する。
    - **進捗 (2026-11-29)**: `lexer.mll` 側で使用する UTF-8 ヘルパー API と `RunConfig` 連動の境界条件（互換モード時は現行 ASCII テーブルを強制する）を整理し、`parser_design.md` §4.5 へ差分説明を追記するタスクを割り当てた。`core_parse_lex.ml` と `parser_run_config.ml` の更新対象フィールドを洗い出し、本節に同期ポイント（`identifier_profile`）を明記した。
    - **進捗 (2025-11-05)**: `lexer.mll` に `Lexer_tables.Unicode_xid_tables` を組み込み、UTF-8 復号と `identifier_profile` 検証（`ascii-compat` / `unicode`）を実装。`Parser_run_config.Lex` / `core_parse_lex.ml` に `identifier_profile` フィールドと `set_identifier_profile` を追加し、`RunConfig.extensions["lex"].identifier_profile` から `Lexer` のプロファイルを切り替えられるようにした。ASCII モードでは非 ASCII を拒否し、Unicode モードでは XID テーブルに基づいて許可/拒否を判定する。
  - **ステップ3 — CI / ライセンス / リリース準備**
    - `dune-project` へ `using fmt` の追加を検討し、`@fmt` と `@check-unicode-tables` を同時に実行するプリセット `scripts/ci-local.sh --section lexer-unicode` を用意する。CI では Linux / Windows / macOS で生成スクリプトが再現可能であることを確認し、生成物の差分があった場合はジョブを失敗させてレビューを促す。
    - 監査ログでは `AuditEnvelope.metadata["unicode.identifier_profile"]` に `{"profile":"unicode","unicode_version":"15.1.0","table_checksum":"..."}`
      を記録し、ASCII モードの場合は `{"profile":"ascii-compat","reason":"Phase2-7 fallback"}` を出力する。`reports/diagnostic-format-regression.md` に Unicode 文字を含む診断サンプルを追加し、表示崩れを監視する。
    - **進捗 (2026-11-29)**: CI 連携と監査メタデータの項目名 (`unicode.identifier_profile`, `unicode.tables.checksum`) を確定し、本節に記録した。`collect-iterator-audit-metrics.py` と `ci-local.sh` の更新手順を Phase 2-7 `diagnostics` チームへ共有済み（週次同期 2026-11-28）。
    - **進捗 (2025-11-05)**: `docs/THIRD_PARTY_NOTICES.md` に Unicode データのライセンス情報を追加し、`scripts/unicode/fetch-unicode-data.sh` で UCD ファイルを取得できるようにした。`compiler/ocaml/third_party/unicode/` をリポジトリに整備し、`compiler/ocaml/src/lexer_tables/dune` へ `@check-unicode-tables` エイリアスを追加して `scripts/unicode/generate-xid-tables.py` の再生成結果を `diff` 検証するフローを確立。CI への組み込みに先立ち、`dune build @check-unicode-tables` でローカル検証可能になった（`compiler/ocaml/scripts/unicode/check_unicode_tables.sh` を介して manifest 差分をタイムスタンプ抜きで比較）。

7.2. **テストとメトリクス**
- CI で `REML_ENABLE_UNICODE_TESTS=1` を常時有効化し、`compiler/ocaml/tests/unicode_ident_tests.ml` と `unicode_identifiers.reml` フィクスチャを全プラットフォームで実行する。`collect-iterator-audit-metrics.py --require-success` の `parser.runconfig.lex.profile` 集計で `unicode` が 100% となることを確認する。
- `lexer.identifier_profile_unicode` 指標が 1.0 へ遷移した日付とログを `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追記し、値が下回った場合は `0-4-risk-handling.md` のリスクを更新する。
- **進捗 (2026-12-02)**: GitHub Actions `bootstrap-linux`, `bootstrap-windows`, `bootstrap-macos` で `REML_ENABLE_UNICODE_TESTS=1` を既定化し、`tooling/ci/collect-iterator-audit-metrics.py --require-success` の `parser.runconfig.lex.profile` 集計が `unicode 100%` を維持することを `reports/audit/summary.md#parser` で確認した。測定結果を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` §0.3.5 へ記録し、`lexer.identifier_profile_unicode = 1.0` で安定したことに伴い `0-4-risk-handling.md` のリスク状況を更新した。

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
- **進捗 (2025-11-05)**: `effect_syntax_tests.ml` とゴールデン `syntax-constructs.json.golden` を追加し、`perform`/`handle` 成功と残余効果リーク失敗を OCaml テストで固定化。新設サマリを `collect-iterator-audit-metrics.py --section effects` から参照できるよう効果メトリクスを実装し、`syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` を `--require-success` でゲート化した。Stage ミスマッチケースは属性構文の不整合により現状未再現のため、後続で `@requires_capability` の Stage/Capability 連携を整理し、`0-4-risk-handling.md#effects-stage-mismatch` にフォローアップを登録する予定。

8.2. **フラグ運用とドキュメント**
- **タスクブレークダウン**:
  - CLI/RunConfig: `compiler/ocaml/src/cli/options.ml` に `-Zalgebraic-effects`（別名 `--experimental-effects`）を追加し、`Options.to_run_config` で `Parser_run_config.set_experimental_effects` を呼び出す。`compiler/ocaml/src/main.ml` では `Parser_run_config` に伝播した値を `Parser_driver.run` へ渡し、`Parser_flags.set_experimental_effects_enabled` の初期化シーケンスが CLI 実行毎にリセットされることを確認する。ヘルプ出力と `docs/guides/cli-workflow.md` にフラグ説明を追加し、Stage Override メッセージと文言を揃える。
  - Tooling/LSP/CI: `tooling/lsp/tests/client_compat` の初期化経路（`diagnostic_transport.ml` ほか）で RunConfig を拡張し、`experimental_effects` を LSP セッションへ伝搬する。`tooling/ci/collect-iterator-audit-metrics.py` と `scripts/validate-diagnostic-json.sh` は効果ゴールデン生成時にフラグを既定有効とし、PoC ゴールデン再生成用の補助スクリプト（`scripts/update-effect-poc.sh` 仮称）を整備する。
  - 文書/脚注: Stage 昇格後に脚注 `[^effects-syntax-poc-phase25]` を撤去できるよう、`docs/spec/1-1-syntax.md`・`docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/3-8-core-runtime-capability.md`・`docs/spec/README.md` に解除チェックリストを追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と同期する。`docs/guides/plugin-authoring.md`・`docs/notes/dsl-plugin-roadmap.md` にフラグ依存の記述を追加し、外部連携ドキュメントの整合を取る。
  - プラグイン／監査: `effects.syntax.constructs.*` メトリクスへフラグ状態を記録し、`RuntimeBridge` 監査と `reports/diagnostic-format-regression.md` の差分レビューで Experimental モードの挙動を追跡できるようにする。
- **検証観点**:
  - CLI: `dune exec bin/remlc -- --help` にフラグ説明が表示され、`-Zalgebraic-effects` 有効／無効の双方で `compiler/ocaml/tests/effect_syntax_tests.ml` と `compiler/ocaml/tests/effect_handler_poc_tests.ml` が通過する。
  - LSP/CI: `tooling/lsp/tests/client_compat` のプロトコル交渉ログで `experimental_effects` が明示的に切り替わり、`collect-iterator-audit-metrics.py --section effects --require-success` が `syntax.effect_construct_acceptance = 1.0` / `effects.syntax_poison_rate = 0.0` を維持する。
  - ドキュメント: 脚注撤去後に `docs/spec/README.md`・`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の索引・差分リストに残存リンクがないことを確認する。
- **リスクとフォローアップ**:
  - フラグ整合が崩れた場合は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の `EFFECT-POC-Stage` を `Escalated` に更新し、Phase 2-7 Sprint C レビューで是正策を確認する。
  - フラグ名称に変更が生じた際は CLI/LSP/ドキュメントの文言を同時更新し、影響範囲を `docs/notes/effect-system-tracking.md`（H-O3 トラッキング）へ追記する。
- **進捗 (2026-12-07)**: CLI (`compiler/ocaml/src/cli/options.ml`) に `experimental_effects` を制御するフラグが未導入であること、`Options.to_run_config` が `Parser_run_config.set_experimental_effects` を呼び出していないこと、LSP/CI 側でも PoC フラグが共有されていないことを確認。差分と対応順序（CLI → LSP → CI → ドキュメント）を `docs/notes/effect-system-tracking.md` 2026-12-07 更新分へ記録した。

8.3. **ハンドオーバーとレビュー**
- `docs/notes/effect-system-tracking.md` の「Phase 2-5 S4 引き継ぎパッケージ」に沿って、PoC 到達条件と残課題を確認。チェックリスト H-O1〜H-O5 が完了した時点で `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` に更新メモを残す。
- 週次レビューで効果構文の Stage 遷移を報告し、`syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` の推移を `0-3-audit-and-metrics.md` へ記録する。脚注撤去可否は Phase 2-7 終盤のレビューで判断する。
- **進捗 (2026-12-12)**: H-O1/H-O2 を完了としてクローズし、週次レビューで `collect-iterator-audit-metrics.py --section effects --require-success` の値が `syntax.effect_construct_acceptance = 1.0` / `effects.syntax_poison_rate = 0.0` で安定していることを確認した。H-O3（フラグ伝播）、H-O4（Stage 監査連携）、H-O5（脚注撤去条件）は未達のため、継続タスクとして backlog に残す。レビュー結果は `docs/notes/effect-system-tracking.md#2026-12-12-h-o1〜h-o5-進捗レビュー` と `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` 2026-12-12 記録に同期済み。脚注撤去は H-O3/H-O4 の完了後に再審議する。

**成果物**: 効果構文 PoC 実装、CI メトリクス 100% 化、フラグ運用指針、脚注撤去条件の整理

### TYPE-002 効果行統合ロードマップ {#type-002-effect-row-integration}
*参照*: `docs/plans/bootstrap-roadmap/2-5-to-2-7-type-002-handover.md`、`docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-002-proposal.md`
**担当領域**: Type + Effects + QA  
**着手条件**: Phase 2-5 TYPE-002 Step1〜Step4 が完了しており、`compiler/ocaml/docs/effect-system-design-note.md` §3、`docs/spec/1-2-types-Inference.md` / `1-3-effects-safety.md` / `3-6-core-diagnostics-audit.md` の効果行記述、および `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI が整合していること。

**スプリント構成（想定: Week35〜Week37）**

1. **Sprint A — 型表現と dual-write 基盤**  
   - `types.ml` に `effect_row` レコード（`declared` / `residual` / `canonical` / `row_var`）を導入し、`TArrow of ty * effect_row * ty` を追加。  
   - `typed_ast.ml` と `Type_inference` で `effect_row` を構築しつつ、既存の `typed_fn_decl.tfn_effect_profile` を並行保持する dual-write モードを実装。  
   - `RunConfig.extensions["effects"].type_row_mode` に `dual-write` を追加し、CLI/LSP/CI オプションで `metadata-only` ↔ `dual-write` を切り替えられるようにする。  
   - 監査ログへ `effect.type_row.{declared,residual,canonical}` を出力し、`collect-iterator-audit-metrics.py --section effects` のベースラインを記録。
  - **完了状況 (2026-12-05)**: `compiler/ocaml/src/types.ml` と `type_inference.ml` で `TArrow` 拡張と `typed_fn_decl.tfn_effect_row` を導入し、`--type-row-mode <metadata-only|dual-write>` から dual-write を有効化できるよう `cli/options.ml`・`parser_run_config.ml` を更新した。`Constraint_solver`・`main.ml`・`diagnostic.ml` を拡張して `effect.type_row.*` メタデータを監査／診断に書き出し、Sprint C で `type_row_mode` 既定値を `"ty-integrated"` へ切り替える準備を整えた。残課題として KPI ゲート (`collect-iterator-audit-metrics.py --section effects`) の閾値見直しと Sprint B での単一化ルール実装を続行する。

2. **Sprint B — 推論・テスト・KPI 実装**  
   - `generalize` / `instantiate` / `Type_unification` / `constraint_solver.ml` で `effect_row` を扱うユーティリティを実装し、RowVar は予約値 (`Open`) として保持。  
   - `Effect_analysis.merge_usage_into_profile` と `Type_inference_effect` を更新し、残余効果が `effect_row.residual` へ反映されるようにする。  
   - テストスイート: `compiler/ocaml/tests/test_type_inference.ml` に `type_effect_row_equivalence_*` ケース、`compiler/ocaml/tests/streaming_runner_tests.ml` に `streaming_effect_row_stage_consistency` を追加。  
   - KPI: `collect-iterator-audit-metrics.py --require-success --section effects` で `diagnostics.effect_row_stage_consistency = 1.0`, `type_effect_row_equivalence = 1.0`, `effect_row_guard_regressions = 0` をゲート条件に設定。逸脱時は自動ロールバック（`type_row_mode=metadata-only`）を実行し、`0-4-risk-handling.md` に登録。  
   - **完了状況 (2025-11-06)**: `constraint.ml` の `unify` が効果行を厳密比較するよう改修し、`type_error`・`main` で効果行メタデータを診断／監査へ展開。`test_type_inference.ml`・`streaming_runner_tests.ml` に効果行統合テストを追加し、`collect-iterator-audit-metrics.py` に `diagnostics.effect_row_stage_consistency` / `type_effect_row_equivalence` / `effect_row_guard_regressions` の集計・ゲート処理を実装した。  
   - *2027-01-05 追記*: Rust 移植 P1 W3 で `docs/plans/rust-migration/appendix/type-inference-ocaml-inventory.md` を整備し、`type_row_mode` と KPI ゲート (`effects.unify.*`, `effects.impl_resolve.*`) を dual-write へ組み込む準備を実施。W3 以降は Rust 側の `TypecheckConfig` が同 KPI を計測できることを本書 §検証・完了条件に含める。

3. **Sprint C — Core IR 伝播とプラットフォーム検証**  
   - `core_ir/desugar_fn.ml`, `core_ir/iterator_audit.ml`, `runtime/effect_registry.ml` を更新し、IR/Runtime の効果情報が `effect_row` を参照できる状態にする。  
   - Windows/macOS CI ワークフローを更新し、`collect-iterator-audit-metrics.py --section effects --platform <target>` で `effect_row_guard_regressions` が 0 件であることを確認。  
  - CLI/LSP ゴールデンを更新し、dual-write 期間中の差分レビューを `reports/diagnostic-format-regression.md` §2 に追記。  
  - 仕様脚注撤去チェックリスト（KPI 1.0 維持・監査ログ整合・Docs/Type レビュー承認）を満たした時点で Phase 2-8 へ報告。（2026-12-18 に完了済み）
  - **進捗 (2026-12-18)**: Core IR `fn_metadata` に `effect_row` を追加し、`iterator_audit` / `AuditEnvelope` に `effect.type_row.{declared,residual,canonical}` を出力。`RunConfig.extensions["effects"].type_row_mode` の既定値を `"ty-integrated"` へ更新し、CLI/LSP ゴールデンを再生成。Linux/Windows/macOS で `collect-iterator-audit-metrics.py --section effects --require-success --platform <target>` を実行し、`effect_row_guard_regressions = 0` を確認。仕様脚注を撤去し、関連ログ（`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`, `docs/plans/bootstrap-roadmap/2-5-review-log.md`, `docs/spec/README.md`）を更新した。

**検証・完了条件**
- `dune runtest compiler/ocaml/tests/test_type_inference.ml --force` で `type_effect_row_equivalence_*` シリーズが全て成功し、CI 集計で 1.0 を報告する。  
- `collect-iterator-audit-metrics.py --require-success --section effects` が Linux/macOS/Windows すべてで成功し、`effect_row_guard_regressions = 0` のまま `ty-integrated` へ切り替えが完了する。  
  - dual-write → `ty-integrated` への移行後、`effects.type_row.integration_blocked` 診断が発生しないことを CLI/LSP/監査のゴールデンで確認し、互換モードが必要な場合は `--type-row-mode=metadata-only` で旧挙動へ戻せる。  
- `docs/spec/1-2-types-Inference.md` / `1-3-effects-safety.md` / `3-6-core-diagnostics-audit.md` の効果行脚注を削除し、`docs/notes/effect-system-tracking.md` と本書に完了メモ（解除日・KPI 値・レビュー承認者）を記録する。

**ハンドオーバー**
- Step5（Phase 2-5 TYPE-002）で作成するハンドオーバーノートを参照し、dual-write 期間の監査ログとテストログを保管。  
- RowVar（行多相）については Phase 3 へ移管し、`constraint_solver` 拡張案・API 予約値の扱い・性能評価計画を `effect-system-tracking.md#phase-3-以降の検討` に沿って追跡する。

### 5. Phase 2-8 への引き継ぎ準備（36週目後半）
**担当領域**: ドキュメント整備

5.1. **差分記録**
- Phase 2-4, 2-7 で実施した変更点・残項目を `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の前提セクションへ追記。
- 監査ログ/診断の安定化完了を `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`（新規）から参照できるよう脚注を整備。
- **完了状況 (2026-12-21)**: `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` 冒頭の前提項目へ Phase 2-4 の成果と Phase 2-7 の残課題クローズ結果を追加し、`docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` の前提節から脚注 `[^phase28-handshake]` を経由して参照できるよう更新した。差分サマリーは `docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md` §2 に整理し、Phase 2-8 着手時の導線を確認済み。

5.2. **メトリクス更新**
- `0-3-audit-and-metrics.md` に CI pass_rate の推移と LSP テスト完了状況を記録。
- `tooling/ci/collect-iterator-audit-metrics.py` の集計結果を `reports/audit/dashboard/` に反映し、Phase 2-8 のベースラインとする。
- DIAG-003 Step5 で追加された `diagnostics.domain_coverage` / `diagnostics.plugin_bundle_ratio` / `diagnostics.effect_stage_consistency` をダッシュボードへ掲載し、`Plugin` / `Lsp` / `Capability` ドメインの Stage 連携が視覚化されるようグラフとしきい値を設計する（`docs/spec/3-6-core-diagnostics-audit.md` 脚注参照）。
- **完了状況 (2026-12-21)**: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に Phase 2-7 最終週の pass_rate 推移と LSP 契約テスト結果を追記し、`reports/audit/dashboard/diagnostics.md` を新設。`collect-iterator-audit-metrics.py --section diagnostics --require-success` の集計ログ（`reports/audit/phase2-7/diagnostics-domain-20261221.json`）を基に `diagnostics.domain_coverage = 1.0`, `diagnostics.plugin_bundle_ratio = 0.98`, `diagnostics.effect_stage_consistency = 1.0` を記録し、閾値逸脱時のエスカレーション手順を脚注 `[^diagnostic-dashboard-phase27]` にまとめた。

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
- [2-7-completion-report.md](2-7-completion-report.md)
- [2-7-to-2-8-handover.md](2-7-to-2-8-handover.md)
- [compiler/ocaml/docs/technical-debt.md](../../../compiler/ocaml/docs/technical-debt.md)
- [reports/diagnostic-format-regression.md](../../../reports/diagnostic-format-regression.md)
- [reports/ffi-bridge-summary.md](../../../reports/ffi-bridge-summary.md)

[^streaming-flow-auto-phase27]: FlowController Auto ポリシーの暫定運用ガイド。`max_lag_bytes` はチャンクサイズの 2 倍以内、`debounce_ms` は 5–50ms、`throttle_ratio` は 0.5–0.9 を推奨し、`RuntimeBridge` で `stream_signal`/`bridge.stage.backpressure` を監査する。CI では `collect-iterator-audit-metrics.py --section streaming --require-success` をゲートとし、逸脱時は `--stream-flow manual` へロールバックして `0-4-risk-handling.md#stream-poc-backpressure` を更新する。

[^phase28-handshake]: Phase 2-8 仕様完全性監査計画の前提節。`docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` に Phase 2-4/2-7 の差分記録集約と監査ログ安定化を紐付ける脚注を追加し、引き継ぎ資料を辿れるよう整備した。

[^diagnostic-dashboard-phase27]: 診断ダッシュボード運用ノート。`reports/audit/dashboard/diagnostics.md` と `reports/audit/phase2-7/diagnostics-domain-20261221.json` に集計ログを保存し、`diagnostics.domain_coverage` ≥ 0.95、`diagnostics.plugin_bundle_ratio` ≥ 0.95、`diagnostics.effect_stage_consistency` = 1.0 を維持できない場合は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#diagnostic-domain-metrics` に即時エスカレーションする。
