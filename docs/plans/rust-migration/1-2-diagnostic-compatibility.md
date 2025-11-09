# 1.2 診断互換性計画

Rust フロントエンド移植において、OCaml 実装と同一の診断 (`Diagnostic.t`) を生成するための基準と検証手順を定義する。構文・型推論・効果解析で発生する診断が JSON/LSP/監査メトリクスに反映される点までをカバーし、`reports/diagnostic-format-regression.md` のフローと整合させる。

## 1.2.1 目的
- OCaml 版で生成される診断 JSON / CLI 出力 / LSP データを Rust 版でも再現し、`effects.*`, `parser.stream.*`, `type_row.*` など拡張フィールドを含めて完全互換を確保する。
- Dual-write 実行時に発生する差分を特定・分類・記録する手順を定義し、仕様差分か実装差分かを判定できる状態を作る。
- CI（P3）で導入予定の自動比較ジョブを想定し、Rust 版診断生成の API とメトリクス収集を標準化する。

## 1.2.2 スコープ
- **対象**: `Diagnostic.Builder`, `parser_driver.ml` の recover 拡張、型推論エラー (`Type_error`), 効果監査 (`Type_inference_effect`, `collect-iterator-audit-metrics.py`)。
- **除外**: CLI レイヤーの最終的なテキスト整形（`diagnostic_formatter.ml`）の Rust 実装詳細。テキスト整形は Phase P2 で再検討し、P1 では JSON 互換性と LSP/XLang への出力のみ確認する。
- **前提**: P0 で確定したゴールデン (`compiler/ocaml/tests/golden/diagnostics/`) が最新であり、`scripts/validate-diagnostic-json.sh` が成功する状態。

## 1.2.3 ベースラインと比較対象

| 出力経路 | OCaml 版ベースライン | 検証用ツール | 備考 |
| --- | --- | --- | --- |
| CLI JSON | `compiler/ocaml/tests/golden/diagnostics/*.json.golden` | `scripts/validate-diagnostic-json.sh` | JSON Schema v2.0.0-draft に準拠 |
| CLI テキスト | `compiler/ocaml/tests/golden/diagnostics/*.txt.golden` | `diagnostic_formatter.mli` を参照（P1 では参考） | P2 で Rust CLI 実装と同期 |
| LSP JSON-RPC | `tooling/lsp/tests/client_compat/fixtures/*.json` | `npm run ci --prefix tooling/lsp/tests/client_compat` | Position 情報の差分は許容なし |
| 監査メトリクス | `reports/diagnostic-format-regression.md` 手順で生成 | `tooling/ci/collect-iterator-audit-metrics.py` | `--section parser`/`effects` 等 |

## 1.2.4 差分分類と対応

| 分類 | 例 | 対応 |
| --- | --- | --- |
| 仕様差分（許容外） | `severity` が `Warning` から `Error` へ | 即時修正。Rust 実装のバグとして扱い、差分ログに記録 |
| 実装差分（許容内） | フィールド順序、空配列省略 | `reports/diagnostic-format-regression.md` で規定された正規化を適用 |
| 新拡張フィールド追加 | `extensions.effect_syntax.*` の増加 | `docs/spec/3-6-core-diagnostics-audit.md` 等の仕様更新を伴う。P1 では原則追加しない |
| Precision 差分 | 数値のフォーマット違い | `serde_json::Number` の文字列表現を OCaml と揃える（`format!("{:.6}", ...)` 等） |

## 1.2.5 Dual-write 検証フロー
1. `remlc --frontend ocaml --format json --emit-ast path.reml > reports/dual-write/front-end/ocaml/<case>.json`
2. `remlc --frontend rust --format json --emit-ast path.reml > reports/dual-write/front-end/rust/<case>.json`
3. `scripts/validate-diagnostic-json.sh reports/dual-write/front-end/{ocaml,rust}/<case>.json` を実行して Schema 検証
4. `jq --sort-keys` で整形し `diff -u`。差分がある場合は `reports/dual-write/front-end/diff/diagnostic_<case>.diff` に保存
5. `collect-iterator-audit-metrics.py --section parser --baseline reports/dual-write/front-end/ocaml/<case>.json --candidate reports/dual-write/front-end/rust/<case>.json` を実行し、メトリクス差分を取得
6. 差分内容を `reports/diagnostic-format-regression.md` のフォーマットで記録し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に TODO を追加（必要なら）

## 1.2.6 重点監視フィールド

| キー | 説明 | 参照仕様 | 検証観点 |
| --- | --- | --- | --- |
| `expected_tokens` (recover 拡張) | 期待トークン列 | `docs/spec/2-5-error.md` | OCaml と同順序・同件数、`message/context` の有無一致 |
| `effects.stage.*` | 効果段階監査 | `docs/spec/3-6-core-diagnostics-audit.md` | `type_row` 診断と連動、空配列は省略 |
| `parser.stream.*` | ストリーミング監査 | `docs/guides/core-parse-streaming.md` | Packrat 収束率、`backpressure_sync` |
| `type_row.*` / `typeclass.dictionary.*` | 型行 / 型クラス辞書監査 | `docs/spec/1-3-effects-safety.md`, `EFFECT-002-proposal.md` | dual-write で JSON 配列順序を固定 |
| `extensions["recover"]` | 再回復ヒント | `reports/diagnostic-format-regression.md` | CLI/LSP 両方で一致すること |

## 1.2.7 Rust 実装での設計指針
- `Diagnostic` モデルは `serde` で JSON 直列化可能な構造体として設計し、既存スキーマと同じフィールド名を採用。`Option`/空配列の扱いは OCaml 実装に合わせて「空配列 → 省略」「空文字列 → 省略」。
- `Diagnostic.Builder` の API を Rust でも提供し、`set_expected`, `set_extension` 等のメソッド名を踏襲。`recover` 拡張は専用ビルダ関数を定義する。
- `parse_error` / `type_error` などイベント単位でログ出力を行い、dual-write 比較時に原因追跡できるよう `trace_id` を付与する。
- 効果監査 (`Type_inference_effect`) のメタデータは `HashMap<String, Value>` で保持し、`collect-iterator-audit-metrics.py` が期待するキーセットを維持。Rust 実装では `serde_json::Value` で透過的に扱う。

## 1.2.8 テスト拡張計画
- **ゴールデン増補**: 効果構文 PoC (`effect_syntax.*`) や Streaming Runner (`parser/streaming-outcome.json.golden`) を Rust 版向けに再実行し、差分がなければ共通ゴールデンとして維持。
- **CLI/LSP 一貫性テスト**: `tooling/lsp/tests/client_compat` を Rust 実装で再利用できるよう、`remlc` CLI に Rust フロントエンド選択フラグを追加。LSP から得た診断 JSON を CLI 出力と diff。
- **手動検証ノート**: 仕様変更や例外的な差分は `reports/diagnostic-format-regression.md` の指示に従って調査ノートを残し、`docs/notes/` に TODO 付きで記録する。

## 1.2.9 既知リスクと対策
- **JSON 直列化の順序差**: Rust の `serde_json` はマップ順序を保証しないため、`IndexMap` を採用してフィールド順序を OCaml と揃える。`sort_keys` を行ってから比較することも必須。
- **数値フォーマットの差分**: `f64` 等をそのまま直列化すると指数表記が変化する可能性がある。OCaml 版が文字列を保持している箇所（リテラル等）は Rust でも文字列として保存。
- **Packrat 統計の収集差**: Rust 実装で `packrat_stats` を実装しないと `parser.stream.packrat_hit` 等が 0 になる。`Core_parse_streaming.packrat_cache` 同等のメトリクスを実装する。
- **外部依存ライブラリ**: JSON Schema 検証のために `jsonschema` crate を導入する場合、スキーマファイルのメンテナンスを `docs/spec/2-5-error.md` と同期させる。

## 1.2.10 ドキュメント連携
- 本計画で確定した比較ルールは `1-0-front-end-transition.md` に記載したマイルストーンと連動させ、レビュー時に参照する。
- 差分の緩和条件や例外は `appendix/glossary-alignment.md`・`docs/spec/3-6-core-diagnostics-audit.md` に反映し、用語・キー名称の整合を保つ。
- CI への組み込み手順は P3 ドキュメント (`3-0-ci-and-dual-write-strategy.md`) に移植する。P1 ではローカルおよび臨時 CI ジョブで実施。

## 1.2.11 型推論起因診断の比較手順（W3 拡張）
- `docs/plans/rust-migration/appendix/w3-typeck-dualwrite-plan.md` で定義した `effects-metrics.{ocaml,rust}.json` と `typeck-debug.{ocaml,rust}.json` を診断互換性の必須成果物に追加する。`collect-iterator-audit-metrics.py --section effects --require-success` を実行し、`effects.impl_resolve.delta` `effects.stage_mismatch.delta` が ±0.5pt 以内であることを確認する。
- `scripts/poc_dualwrite_compare.sh --mode typeck --run-id <label> --cases <file>` を実行すると、`reports/dual-write/front-end/w3-type-inference/<case>/` に `diagnostics.{ocaml,rust}.json` / `effects-metrics.*` / `typeck-debug.*` が保存される。`typeck-debug` には `effect_scope`, `residual_effects`, `recoverable`, `ocaml_exception` など型推論固有のフィールドが含まれるため、`jq --sort-keys` で整形した後 `diff -u` を取得する。
- Rust 側で `Result<T, TypeError>` を導入した箇所は、OCaml 側の例外名・診断コード・Recover ヒントを `diagnostic::codes::TYPE_*` に写像し、`typeck-debug` に `{"ocaml_exception": "...", "rust_error": "...", "diagnostic_code": "TYPE_xxx"}` の形で両実装のメタデータを併記する。これにより、`scripts/validate-diagnostic-json.sh` が指摘した差分を `typeck-debug` から逆引きできる。
- CLI 追加フラグ: `remlc --frontend rust --emit typed-ast --emit constraints --emit typeck-debug <dir>` / `remlc --frontend ocaml --emit-constraints-json <path> --emit-typeck-debug <path>`。両方の出力を `p1-front-end-checklists.csv` の新規行（型推論診断）の受入基準として記録し、`docs/spec/3-6-core-diagnostics-audit.md` へのフィードバック対象にする。
- *2027-01-17 進捗*: `reports/dual-write/front-end/w3-type-inference/2027-01-15-w3-typeck/diagnostic-validate.log` で `scripts/validate-diagnostic-json.sh` 通過、`effects-metrics.{ocaml,rust}.json` の `effects.unify.*` / `effects.impl_resolve.*` 誤差 0 を確認した。`ffi_dispatch_async` のみ OCaml 側診断が `Type_error` で終了するため `typeck_match=false` だが、診断 JSON の差分は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#W3-TYPECK-ffi-dispatch-async` で追跡し、その他 4 ケースは `typeck-debug` を含め完全一致した。

## 1.2.12 W4 診断互換試験向けベースライン更新
- *2027-11-07 進捗*: W4 Step1（ゲート設定）として OCaml 側資産を再検証し、`reports/dual-write/front-end/w4-diagnostics/baseline/` に成果物を集約した。  
  - `npm ci && npm run ci --prefix tooling/lsp/tests/client_compat` を実行し、LSP V2 フィクスチャ 9 件の pass を確認。  
  - `scripts/validate-diagnostic-json.sh $(cat tmp/w4-parser-diag-paths.txt)` で Schema v2.0.0-draft を 10 ケース通過させ、リスト外だった `compiler/ocaml/tests/golden/diagnostics/effects/syntax-constructs.json.golden` は validator 側のフィルタ（2027-11-07 `DIAG-RUST-03` 完了）で除外。  
  - `collect-iterator-audit-metrics.py --section parser|effects|streaming` の結果を `parser-metrics.ocaml.json` / `effects-metrics.ocaml.json` / `streaming-metrics.ocaml.json` に保存し、`domain/multi-domain.json.golden` の audit メタデータを補完して `diagnostic.audit_presence_rate=1.0`（`DIAG-RUST-04` 完了）。  
- Rust 側 dual-write を始める前に上記 TODO を解消し、OCaml 基準の完全通過を達成することが W4 Step2 以降の着手条件となる。
- `appendix/w4-diagnostic-case-matrix.md` でカテゴリ別ケースを公開し、`w4-diagnostic-cases.txt` から `scripts/poc_dualwrite_compare.sh --mode diag --cases ...` を実行できるようにした。`scripts/dualwrite_summary_report.py --diag-table reports/dual-write/front-end/w4-diagnostics/README.md --update-diag-readme ...` を併用し、`reports/dual-write/front-end/w4-diagnostics/README.md` の `<!-- DIAG_TABLE_* -->` ブロックにサマリ表を自動埋め込みする。
- *2027-11-09 進捗*: `reports/dual-write/front-end/w4-diagnostics/20271107-w4-new` で diag モードの初回ランを実施し、recover 5 件 + type/effect 1 件を OCaml/Rust で収集。結果は `tmp/w4new-table.md` および README の diag テーブルに反映済み。課題として以下を確認した。
  - Rust recover ケースで `diagnostics` が 0（`recover_else_without_if`）/2（`recover_lambda_body`）になり、`gating=false`。`DOC: DIAG-RUST-05` で Rust parser recover 実装の差分を追跡。  
  - すべてのケースで `collect-iterator-audit-metrics.py --section streaming` が `parser.stream_extension_field_coverage < 1.0` を返し、メトリクスゲートがブロック。Streaming ケースを追加し、非ストリーミング入力では `--section streaming` をスキップする条件を `DIAG-RUST-05` で検討する。  
  - `type_condition_bool` は inline 化したものの、OCaml 側 JSON が未生成で schema 検証が実行されず（Rust 側のみ 1 件）。`ocaml.parse-debug.json` では `diagnostics=[]` かつ type inference ログ未出力であることから、現行 `diag` ハーネスが parser までしか実行していないことが判明。`scripts/poc_dualwrite_compare.sh --mode typeck` を併用して型診断を取得し、CLI 側には `--type-row-mode dual-write --experimental-effects --emit-typeck-debug <tmp>` を付与する TODO を `DIAG-RUST-06` に追加した。Rust 版はまだパラメータ型注釈を解釈できず `:` トークンで recover しているため、typed parameter 対応を `compiler/rust/frontend/parser` 側に実装するまでは `type_condition_literal_bool`（bool 条件に整数リテラルを置くサブケース）を使って recover/metrics の健全性を確認する。
- *2027-11-12 進捗*: `appendix/w4-diagnostic-case-matrix.md` に Source/CLI 列を追加し、`test_cli_diagnostics.ml`／`streaming_runner_tests.ml`／`test_cli_callconv_snapshot.ml`／`test_ffi_contract.ml`／`DIAG-002-proposal.md` など参照元を明文化した。これに合わせて `w4-diagnostic-cases.txt` へ `#tests` / `#flags` / `#lsp-fixture` メタデータを付与し、diag モードと LSP フィクスチャ双方で同じ入力セットを再利用できるようにした。  
  - CLI フラグ（`--streaming` / `--stream-resume-hint` / `--experimental-effects` / `--effect-stage beta` / `--type-row-mode dual-write` など）をケース単位で記録したため、`poc_dualwrite_compare.sh --mode diag` の再実行と `diagnostic.audit_*` チェックが自動化できる。  
  - `p1-front-end-checklists.csv` に「W4 diag ケースマトリクス整備」の受入項目を追加し、Step2 で要求された「各カテゴリ 3 件以上 + parser recover 5 件」の達成状況を記録。今後は README/Runbook 更新をこのチェックリスト経由でトレースする。

## 1.2.13 W4 診断 dual-write 実行結果（20271112）
- `scripts/poc_dualwrite_compare.sh --mode diag --run-id 20271112-w4-diag-m1 --cases docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` を再実行し、`reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/summary.md` に 21 ケース分の `gating/schema_ok/metrics_ok` を集約した。`scripts/dualwrite_summary_report.py --diag-table` で README へ反映済み。  
- 実行結果の主な観測:  
  1. `diag_match` は `recover_missing_semicolon` / `recover_missing_tuple_comma` / `recover_unclosed_block` / `type_condition_literal_bool` の 4 ケースのみ。Rust 側 `diagnostics.rust.json` が依然としてデバッグ用途の構造（`severity` や `schema_version`、`extensions.*`、`domain`、`location` 欄が欠落）であり、`collect-iterator-audit-metrics.py` が `parser.audit_pass_fraction=0.0` を返す。代表例: `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/recover_missing_semicolon/diagnostics.rust.json` および同ディレクトリの `parser-metrics.rust.err.log`。→ `DIAG-RUST-05/07` で Rust `Diagnostic` モデルと CLI/LSP 監査キーの整備を継続。  
  2. Recover ケースの一部（`recover_else_without_if`, `recover_lambda_body`）では Rust フロントエンドが recover 診断を生成できず `diagnostics=[]` または過剰件数となる。一方 OCaml 側は `extensions.recover.*` を保持しているため、Rust 側で `parser_expectation` の同期と `Diagnostic.Builder` を実装する必要がある。→ `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-diag-rust-05`。  
  3. Streaming / Capability / CLI / LSP ケースでは `w4-diagnostic-cases.txt` に記載した `#flags` が diag ハーネスへ伝播しておらず、`--streaming` `--stream-resume-hint diag-w4` `--experimental-effects` `--runtime-capabilities ...` `--trace` などが未設定のまま CLI を実行している。このため OCaml 側でも意図した監査フィールドが生成されず、`reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/stream_pending_resume/schema-validate.log` では `diagnostics[0].expected` 欠落により Schema ゲートが停止し、`parser.stream_extension_field_coverage` と `parser.runconfig_switch_coverage` が常に 0.0 で `metrics_ok=false` となる。→ `DIAG-RUST-05/07` でケースメタデータ（`#flags`/`#tests`/`#lsp-fixture`）を `poc_dualwrite_compare.sh` の CLI 引数へ反映するタスクを最優先で実施する。  
  4. Type / Effect / Capability Stage ケースでは OCaml 側 CLI が parser フェーズで終了しており、`reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/type_condition_bool/diagnostics.ocaml.json` のように空配列のまま。Rust 側は recover 1 件のみを出力するため比較が成立しない。`--type-row-mode dual-write --emit-typeck-debug <dir>` を diag モードにも注入し、`reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/type_condition_bool/typeck/typeck-debug.ocaml.manual.json` と同じフラグを自動化しない限り `DIAG-RUST-06` は完了しない。  
- 以上の結果を `p1-front-end-checklists.csv`（診断行）と `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`（DIAG-RUST-05/06/07）へ反映し、W4 Step4 のトリアージ対象を明記した。

## 1.2.14 W4 Streaming / `parser_expected_summary` 再測定（20280115）
- Run `20280115-w4-diag-refresh`（`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/summary.md`）を実行したところ、Streaming 系 3 ケースが引き続き `metrics_ok=false` で停止し、`parser-metrics.(ocaml|rust).err.log` に `parser.expected_summary_presence < 1.0` が出力された。`triage.md:25-27` でも `parser.stream.*` バッグが未充足扱いであることが示されている。  
- Rust 側 `parser-metrics.rust.json` では `parser.stream_extension_field_coverage` 自体は 1.0 である一方、`parser.expected_summary_presence` が 0.09（11 件中 1 件のみ `expected` を持つ）に留まっている（例: `reports/.../stream_pending_resume/parser-metrics.rust.json`）。OCaml 側は Schema 検証で `diagnostics[0].expected` 欠落が発生し、`parser-metrics.ocaml.err.log` が同じ警告を発している。  
- **アクション**  
  1. `parser_expectation.ml` と Rust `Diagnostic.Builder` に Streaming Pending/Resume 用の `expected` フォールバックを実装し、`parser.expected_summary_presence=1.0` になるまで CLI を再計測する。該当処理は `collect-iterator-audit-metrics.py` の `summarize_diagnostics`（`1243-1304` 行）で直接評価されるため、診断 JSON 自体を修正しない限り解消しない。  
  2. `run_config.extensions.stream` の 6 フィールドと `flow.*` 情報を OCaml/Rust で揃え、`parser.stream_extension_field_coverage` が両言語で 1.0 になることを確認する。`tooling/ci/collect-iterator-audit-metrics.py:1462-1535` の要件一覧に従い、`parser_driver.ml` / `StreamingState` で同じ JSON キーを生成する。  
  3. `scripts/poc_dualwrite_compare.sh --mode diag` に Streaming ケース専用の再測定フローを追加し、`summary.json` の `metrics_ok` を `parser.expected_summary_presence` と `parser.stream_extension_field_coverage` の両方が `1.0` になった場合のみ `true` へ更新する。`collect-iterator-audit-metrics.py --section parser|streaming --require-success` を強制し、失敗時は `parser_expected_summary.json`（`parser-metrics.*.json` から抽出）を人手レビュー用に残す。  
- これらの作業が完了した時点で `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-DIAG-RUST-05` と `p1-front-end-checklists.csv`（Streaming/診断行）を「20280115 run で pass」ステータスへ更新し、P1 W4 の診断互換ゲートが閉じられる。
