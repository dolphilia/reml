# 2-5 レビュー記録

Phase 2-5 で実施した差分レビューと現状棚卸しを記録し、後続フェーズでの追跡に利用する。  
エントリごとに関連計画へのリンクと再現手順を整理する。

## PARSER-002 Day1 RunConfig 現状調査（2025-11-18）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 調査サマリ
- `docs/spec/2-1-parser-type.md:92-175` と `docs/spec/2-6-execution-strategy.md:60-107` を精査し、RunConfig の公式フィールドと既定値、`extensions` ネームスペースの契約を整理。結果は計画書 Step0 の表1・表2へ反映。  
- 現行 OCaml 実装では `type run_config = { require_eof; legacy_result }` のみ存在し、仕様で定義される Packrat/左再帰/trace/merge_warnings/locale/extensions が全て欠落していることを確認（compiler/ocaml/src/parser_driver.ml:6-13）。  
- CLI（compiler/ocaml/src/main.ml:612）およびユニットテスト（例: compiler/ocaml/tests/test_parser.ml:10, compiler/ocaml/tests/test_type_inference.ml:18）は `Parser_driver.parse` / `parse_string` を直接使用し、RunConfig 構築ヘルパが存在しない。  
- `run_partial` は `require_eof=false` を強制するだけで `rest` を返さないスタブ状態であり、ストリーミング拡張と整合しない（compiler/ocaml/src/parser_driver.ml:172-175）。

### 2. 仕様との差分要約
- 既定値の差異: 仕様は `require_eof=false` が既定だが OCaml 実装は `default_run_config.require_eof = true` のまま（compiler/ocaml/src/parser_driver.ml:11）。  
- `trace`・`merge_warnings`・`locale` の制御は `Parser_diag_state` / `Diagnostic.Builder` にスイッチが無く、RunConfig 経由での切替ができない。  
- `extensions["lex"]`・`["config"]`・`["recover"]`・`["stream"]`・`["lsp"]`・`["target"]`・`["effects"]` の標準キーはすべて未実装であり、LEXER-002 / EFFECT-003 / EXEC-001 計画とのインターフェイスが欠落。  
- RunConfig 系メトリクス（`parser.runconfig_switch_coverage` など）は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` にまだ登録されていないため、監視ができない。

### 3. Packrat/左再帰・trace 実装に向けた検討
- Menhir 境界では `Parser.MenhirInterpreter` の `checkpoint` ループに全ての分岐が集中しており（compiler/ocaml/src/parser_driver.ml:133-166）、Packrat 実装時にはここで `(ParserId, byte_off)` をキーとしたメモテーブルを参照する必要がある。  
- Packrat 導入時は `left_recursion` フラグを確認して種成長ループを挿入し、評価中フラグ・`commit_watermark` に基づく掃除を RunConfig 側で初期化する必要がある（docs/spec/2-6-execution-strategy.md:62-74,171-188）。  
- `trace` ON 時にのみ `SpanTrace` や解析イベントを収集する挿し込みポイントは `Lexer.token` 呼び出し前後および `I.Shifting` → `I.resume` の箇所。現状では収集ロジックが無いため無条件でコストゼロ。  
- `merge_warnings=false` を扱うには `Parser_diag_state.record_diagnostic` で回復診断を蓄積する際のフィルタを分岐させ、`extensions["recover"].notes` や監査ログに個別記録できるようにする必要がある。

### 4. TODO / 引き継ぎ
1. （2025-11-18 完了）`parser_run_config.{ml,mli}` を作成し、仕様準拠の `Run_config.t` と `extensions` ラッパーを実装する（PARSER-002 Step1）。  
2. CLI/LSP/テストに共通の RunConfig ビルダーを用意し、既存の `Parser_driver.parse` から新 API へ移行する準備を行う。  
3. Packrat/左再帰シムのメモテーブル要求事項を `PARSER-003` チームへ共有し、`RunConfig.packrat` と `left_recursion` のセマンティクスを整合させる。  
4. RunConfig 測定指標（`parser.runconfig_switch_coverage`、`parser.runconfig_extension_pass_rate`）の追加作業を 0-3 メトリクス管理表へ登録する。

### 5. 実施記録
- 2025-11-18: Step 1 を実施し、`compiler/ocaml/src/parser_run_config.{ml,mli}` に `RunConfig` レコード・拡張マップ API・`Legacy.bridge` を追加。`docs/spec/2-1-parser-type.md` / `docs/spec/2-6-execution-strategy.md` へ OCaml 実装脚注を追記し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` §6.3 に進捗脚注を掲載。次工程（Step 2）では `parser_driver` への伝播とメトリクス登録を行う。

## PARSER-002 Day2 RunConfig 適用（2025-11-19）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 作業サマリ
- `parser_driver` の公開 API を `?config:Run_config.t` に切り替え、`trace` / `merge_warnings` / `locale` を `Parser_diag_state.create` へ伝播。  
- `Parser_diag_state` に `trace` 有効時のみ `SpanTrace` を収集する経路と、`merge_warnings` に応じた警告集約ロジックを追加。  
- `RunConfig.packrat` / `left_recursion` が有効化された場合は未実装警告（`parser.runconfig.packrat_unimplemented` / `parser.runconfig.left_recursion_unimplemented`）を発行し、PARSER-003 へフォローアップを引き継げるようにした。  
- `extensions["config"].require_eof` を優先して未消費入力を検出し、`parser.require_eof.unconsumed_input` エラーを生成して `legacy_result` 互換経路にも反映。  
- `trace=true` の場合に `compilation_unit` ルートスパンを `SpanTrace` に記録し、既定モードでは追加コストが発生しないことを確認。

### 2. 影響ファイル
- `compiler/ocaml/src/parser_driver.ml`  
- `compiler/ocaml/src/parser_diag_state.ml`  
- `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`（Step 2 更新記録）  
- `docs/plans/bootstrap-roadmap/2-5-review-log.md`（本メモ）

### 3. フォローアップ / TODO
- Step 3 で `extensions["lex"]` / `["recover"]` / `["stream"]` シムを実装し、RunConfig の共有拡張を利用できるようにする。  
- Step 4 以降で CLI/LSP/テストの RunConfig ビルダーを導入し、パイプライン全体で新 API を利用する。  
- Step 5 で `trace` / `merge_warnings` の挙動を検証するユニットテストと CI メトリクス（`parser.runconfig_*`）を追加する。（2025-11-22 完了、Day5 ログ参照）

## PARSER-002 Day3 extensions シム構築（2025-11-20）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 作業サマリ
- `parser_run_config` に `Config` / `Lex` / `Recover` / `Stream` サブモジュールを追加し、`extensions` ネームスペースを型安全に読み出すシムを導入。`lex.profile` は未指定時に `ConfigTriviaProfile::strict_json` を返し、`ParserId` は `int option` として保持するよう整理した。  
- `Parser_diag_state.create` が `recover` 設定を受け取れるようになり、`recover.sync_tokens` / `recover.notes` を `recover_sync_tokens` / `recover_notes_enabled` で参照可能。`parser_driver` から `Run_config.Recover.of_run_config` を呼び出し、RunConfig の回復設定が診断経路へ伝播する。  
- `Run_config.Config.require_eof_override` を切り出して `parser_driver` の未消費入力判定を共通化し、`Run_config.Stream.of_run_config` がストリーミング用プレースホルダを返すよう準備した。`compiler/ocaml/src/dune` を更新後に `dune build` を実行し、ワーニングのみでビルド完了を確認。  

### 2. 影響ファイル
- `compiler/ocaml/src/parser_run_config.{ml,mli}`  
- `compiler/ocaml/src/parser_diag_state.ml`  
- `compiler/ocaml/src/parser_driver.ml`  
- `compiler/ocaml/src/dune`  
- `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`（Step 3 更新記録）  
- `docs/plans/bootstrap-roadmap/2-5-review-log.md`（本メモ）

### 3. フォローアップ / TODO
- `LEXER-002` で `Run_config.Lex.effective_trivia` を利用し、共有トリビア API に接続する。  
- `EXEC-001` の `run_stream` 実装時に `Run_config.Stream` プレースホルダを利用して checkpoint/resume 情報を連携する。  
- Step 5 で `Recover` 設定を切り替えた際の診断出力とメトリクス（`parser.runconfig_extension_pass_rate`）を追加検証する。  

## ERR-001 Day1 Menhir 期待集合 API 棚卸し（2025-11-13）

### 1. Menhir 出力サマリ
- `menhir --list-errors compiler/ocaml/src/parser.mly` を実行し、`compiler/ocaml/src/parser.automaton` を再確認したところ状態数は 467 件、shift/reduce 27・reduce/reduce 10 の既存コンフリクト構成に変化なし。
- 期待集合に現れた終端は 74 種類（予約語 33・記号 32・リテラル 5・EOF 1）で、`IDENT`/`STRING`/`INT` が 200 件超の頻出項目、`#` は Menhir の入力終端番兵として扱われる。
- `BREAK`/`CHANNELS`/`CHANNEL_PIPE`/`CONDUCTOR`/`CONTINUE`/`DARROW`/`DO`/`EXECUTION`/`HANDLE`/`MONITORING`/`PERFORM`/`UPPER_IDENT` は期待集合に登場せず、`compiler/ocaml/src/token.ml:49` 以降の予約語定義でも未使用警告の対象となっている。
- 期待集合候補は `compiler/ocaml/src/parser.automaton` から抽出でき、縮約時は記号優先 → 文字クラス → 規則の順で整序する仕様（`docs/spec/2-5-error.md:129`）に従うことで CLI/LSP 表示との整合を保てる。

### 2. API 仕様確認
- `compiler/ocaml/_build/default/src/parser.mli:14` で `Parser.MenhirInterpreter` が `MenhirLib.IncrementalEngine.INCREMENTAL_ENGINE` を公開していることを確認。
- `MenhirLib.IncrementalEngine` は `acceptable` と `MenhirLib.EngineTypes.TABLE.foreach_terminal` を備えており、全終端を走査して checkpoint ごとの期待集合を導出できる。
- トークン定義は `compiler/ocaml/src/token.ml:7` 以降で 85 種類が列挙されており、期待集合生成時はキーワード → 記号 → リテラル → `EOF` のカテゴリごとにサンプル値を用意すれば `acceptable` の判定に利用できる。

### 3. Expectation 写像ルール草案
| Menhir 終端カテゴリ | 対応案 | 備考 |
| --- | --- | --- |
| 予約語 (`FN`/`MATCH` 等) | `Expectation.Keyword (Token.to_string tok)` | `compiler/ocaml/src/token.ml:100` 以降の `to_string` で小文字化 |
| 記号・区切り (`LPAREN`/`PLUS` 等) | `Expectation.Token (Token.to_string tok)` | `PIPE` や `DOTDOT` など複合演算子も記号扱い |
| リテラル (`INT`/`STRING`/`CHAR`/`FLOAT`) | `Expectation.Class "<literal-kind>"` | サンプル値は空文字列・既定基数で構築し `Class` へ収容 |
| 識別子 (`IDENT`/`UPPER_IDENT`) | `Expectation.Class "identifier"` / `"upper-identifier"` | 後者は現状未登場だが仕様整合のため先行定義 |
| 終端番兵 (`EOF`/`#`) | `Expectation.Eof` | Menhir の `#` は `EOF` 相当として扱う |
| 補助 (`Rule`/`Not`/`Custom`) | 上位規則や否定条件を後段で合成 | `docs/spec/2-5-error.md:129` の優先順位へ合わせる |

### 4. Parser_diag_state 制約メモ
- `compiler/ocaml/src/parser_diag_state.ml:24` の `normalize_expectations` は `Stdlib.compare` で並べ替えるため、期待集合の優先順位を保持するにはカテゴリ単位の整列器を別途用意する必要がある。
- `record_diagnostic`（`compiler/ocaml/src/parser_diag_state.ml:27`）は `Diagnostic.expected` が `None` の場合に空リストを採用するため、`ERR-001/S2` 以降で必ず `ExpectationSummary` を生成しないと最遠スナップショットが空集合のままになる。
- `farthest_snapshot`（`compiler/ocaml/src/parser_diag_state.ml:7`）は同一オフセット時に集合和を取る実装なので、Menhir から得た候補をカテゴリ別に縮約してから保存すればノイズを抑制できる。

## ERR-001 Day2 期待集合マッピング実装（2025-11-14）

- `compiler/ocaml/src/parser_expectation.{ml,mli}` を追加し、終端トークン → `Diagnostic.expectation` の写像、`dedup_and_sort` による優先順位整列、`summarize_with_defaults` のフォールバック（`parse.expected` / `parse.expected.empty`）を実装。`humanize` は `Keyword`/`Token` をバッククォートで包む日本語メッセージを生成する。
- `expectation_of_nonterminal` / `expectation_not` / `expectation_custom` を公開し、S3 以降で `Rule`・否定条件・任意候補を `ExpectationSummary` へ集約できるようにした。
- 単体テスト `compiler/ocaml/tests/test_parser_expectation.ml` でキーワード・演算子・リテラル・識別子・EOF・Rule・Not・Custom の 8 ケースとサマリ生成を検証済み。`dune exec tests/test_parser_expectation.exe` の結果を添付し、humanize の自然文と空集合フォールバックを確認。

## ERR-001 Day3 パーサドライバ組込み（2025-11-15）

- `compiler/ocaml/src/parser_expectation.ml` に `collect` を実装し、Menhir チェックポイントから受理可能トークンを走査して `ExpectationSummary` を生成。期待集合が空の際は `Parser_diag_state.farthest_snapshot` 経由でサマリを補完するフォールバックを整理。
- `compiler/ocaml/src/parser_driver.ml` で `HandlingError` / `Rejected` 分岐が `collect` を呼び出し、`Diagnostic.Builder` で期待集合サマリを直接設定するように変更。legacy 互換用 `parse_result.legacy_error.expected` へも同じ候補が伝播することを確認した。
- `compiler/ocaml/src/parser_diag_state.ml` の `farthest_snapshot` に `expected_summary` フィールドを追加し、同一オフセットで診断が蓄積された場合も候補を集合和で縮約するよう更新。
- テスト: `compiler/ocaml/tests/test_parser_driver.ml` / `compiler/ocaml/tests/test_parse_result_state.ml` に期待集合の非空検証を追加し、`run_string` / legacy API の両方で `Diagnostic.expected` と `legacy_error.expected` が一致することをケース化した。
## ERR-001 Day4 ゴールデンと CI 監視整備（2025-11-16）

- `compiler/ocaml/tests/golden/diagnostics/parser/expected-summary.json.golden` を追加し、`dune exec tests/test_cli_diagnostics.exe` で CLI JSON スナップショットを再生成。`tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-sample.json` も `expected.message_key = "parse.expected"` と `locale_args` を付与して LSP 互換テストへ期待集合を反映した。
- `scripts/validate-diagnostic-json.sh` に Parser 診断専用の検証を追加し、`expected` セクションが欠落または `alternatives` が空の場合は即時にエラーを報告するよう強化。
- `tooling/ci/collect-iterator-audit-metrics.py` へ `parser.expected_summary_presence` / `parser.expected_tokens_per_error` を導入し、`summarize_diagnostics` でも Parser 期待集合の統計を集計。`--require-success` 時には期待集合が 0 件の構文エラーが検出された段階で CI を失敗させる。
- 指標リスト `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に両指標を追加し、収集タイミングを `scripts/validate-diagnostic-json.sh` と `collect-iterator-audit-metrics.py` に合わせて明記。`reports/diagnostic-format-regression.md` のチェックリストにも期待集合検証の手順を追記した。

## ERR-001 Day5 ドキュメントと共有タスク（2025-11-17）

- `docs/spec/2-5-error.md` から暫定脚注（実装未導入の注記）を整理し、Phase 2-5 完了後の状態を示す脚注 `[^err001-phase25]` を追加。`ExpectationSummary` の説明に Menhir 期待集合が CLI/LSP/監査で共有される旨を明記した。
- `docs/spec/3-6-core-diagnostics-audit.md` の `ExpectedSummary` 解説へ同様の反映を行い、診断モデル側でも Phase 2-5 ERR-001 の実装完了を参照できるよう脚注 `[^err001-phase25-core]` を追加。
- `docs/guides/core-parse-streaming.md` と `docs/guides/plugin-authoring.md` を更新し、ストリーミング経路およびプラグイン API が `ExpectationSummary` をそのまま活用できる運用ガイドを追記。S4 時点の CLI/LSP ゴールデンは再利用し、ドキュメント差分のみで完結しているため追加のスナップショット生成は不要と判断。
- フォローアップ共有として `docs/notes/spec-integrity-audit-checklist.md` の草案を作成し、Phase 2-8 で利用する監査チェックリストに期待集合モニタリング項目（`parser.expected_summary_presence` / `parser.expected_tokens_per_error`）を登録できるよう TODO セクションを整備。
- `docs/plans/bootstrap-roadmap/2-5-proposals/ERR-001-proposal.md` の S5 セクションを更新し、仕様・ガイド・ノート更新とレビュー共有が完了したことを記録。差分は Git 差分レビューで確認済み、追加のコマンド実行は無し。

## DIAG-002 Day1 調査

DIAG-002 の初期洗い出し結果を記録し、後続フェーズでの追跡に利用する。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-002-proposal.md`](./2-5-proposals/DIAG-002-proposal.md)

## 1. Diagnostic を直接構築している経路
| 種別 | ファイル:行 | 状態 | 想定対応 |
|------|-------------|------|----------|
| Legacy 変換 | `compiler/ocaml/src/diagnostic.ml:181` | `Diagnostic.Legacy.t` から `Diagnostic.t` をレコード直接構築。`audit = None` のまま返却され、`Legacy.audit_metadata` が空の場合は監査キーが欠落する。 | Week31 Day2 以降で `Diagnostic.Builder` 経由の移行パスを追加し、最低限 `Audit_envelope.empty_envelope` と `iso8601_timestamp` を強制する。既存のテストは Builder 経路へ切り替える。 |

## 2. 監査メタデータが不足する経路（`Diagnostic.Builder.create` → `Builder.build`）
| 優先度 | ファイル:行 | 出力チャネル | 現状 | 対応メモ |
|--------|-------------|--------------|--------|----------|
| 高 | `compiler/ocaml/src/llvm_gen/verify.ml:131` | `--verify-ir` 失敗時 (CLI) | `Builder.build` 直後の診断をそのまま `main.ml:597` から出力。`attach_audit` が呼ばれないため `cli.audit_id` / `cli.change_set` など `tooling/ci/collect-iterator-audit-metrics.py` が必須とするキーが欠落し、`ffi_bridge.audit_pass_rate` 集計で非準拠扱い。 | Day2 で `Verify.error_to_diagnostic` に `Diagnostic.set_audit_id` / `set_change_set` を注入するか、`main.ml` 側で再利用している `attach_audit` を適用する。 |
| 中 | `compiler/ocaml/src/diagnostic.ml:945` | `Parser_driver.process_lexer_error` | Builder 直後は監査メタデータが空だが、`main.ml:803` で `attach_audit` を通すため CLI/LSP 出力時点では `cli.audit_id` / `cli.change_set` が補完される。 | 現状維持でも仕様違反にはならないが、計測ログ用の `parser.*` 系キーを Builder 側で自動付与する改善案を検討。 |
| 中 | `compiler/ocaml/src/diagnostic.ml:950` | `Parser_driver.process_parser_error` | Lexer エラーと同じ挙動。`attach_audit` により最終的な監査キーは揃う。 | Parser 向けメタデータ自動化を Lexer と合わせて検討。 |
| 低 | `compiler/ocaml/tests/test_cli_diagnostics.ml:27` | CLI フォーマッタのゴールデン | テスト専用のダミー診断。監査キーが空のままのため、必須化後は `Diagnostic.set_audit_id` 等でフィクスチャを更新する必要がある。 | Day3 以降でゴールデン再生成。レビュー時に `REMLC_FIXED_TIMESTAMP` を考慮。 |

## 3. 補足メモ
- `main.ml:665-694` の Core IR / Codegen 例外、`main.ml:744-748` の型推論エラー、`main.ml:803-804` のパース失敗は `attach_audit` を経由しており、`cli.audit_id`・`cli.change_set` が付与される。
- `tooling/ci/collect-iterator-audit-metrics.py` は 14 件の audit メタデータキーを必須としている。High 優先度の経路から出力される診断は pass rate を 0.0 に固定する要因となるため、Phase 2-5 内での修正を優先する。*** End Patch*** End Patch

## 4. Legacy / シリアライズ整備 進捗（2025-11-02 更新）
- **監査キー補完**: Builder/Legacy 双方で `ensure_audit_id` / `ensure_change_set` を導入し、空値の場合は `phase2.5.audit.v1` テンプレート（CLI: `audit_id = "cli/" ^ build_id ^ "#" ^ sequence`、Legacy: `audit_id = "legacy-import/" ^ build_id`）を生成してから `Audit_envelope.has_required_keys` を通過させる。`missing` フィールドは必須キーが揃った段階で自動的に除去される（compiler/ocaml/src/diagnostic.ml:304-370）。
- **Audit_envelope 拡張**: `Audit_envelope.has_required_keys` を CLI 監査キー込みで再定義し、`missing_required_keys` を公開して検証・エラーメッセージ両方に利用できるようにした（compiler/ocaml/src/audit_envelope.ml:120-189）。
- **シリアライズ検証**: `Diagnostic_serialization.of_diagnostic` で必須キーと `timestamp` をチェックし、欠落時は `[diagnostic_serialization] …` を stderr に出力して `Invalid_argument` を送出する運用へ移行した（compiler/ocaml/src/diagnostic_serialization.ml:75-88）。
- **テスト/ログ**: `dune runtest`（compiler/ocaml）を再実行し、更新された診断ゴールデン（Typeclass/FFI/Effects）を整合させた。`tooling/ci/collect-iterator-audit-metrics.py` は不足フィールドを stderr に出力するようになり、`--require-success` 実行時のトラブルシューティングが容易になった。

## 5. `phase2.5.audit.v1` テンプレート実装後の検証（2025-11-06 更新）
- **CLI/テスト経路の統一**: `compiler/ocaml/src/main.ml` と `test_cli_diagnostics.ml` / `test_ffi_contract.ml` / `test_effect_residual.ml` を更新し、CLI 実行・ユニットテストいずれの経路でも `audit_id = "cli/<build_id>#<sequence>"` とテンプレート化された change-set を出力するようになった。  
- **ゴールデン更新**: Typeclass / FFI / Effects 系ゴールデン（診断 JSON・監査 JSONL）を再生成し、`bridge.audit_pass_rate`・`effect.handler_stack`・`typeclass.*` など必須メタデータが埋まっていることを確認。  
- **CI メトリクス**: `python3 tooling/ci/collect-iterator-audit-metrics.py --require-success` をローカルで実行し、`iterator.stage.audit_pass_rate`・`typeclass.dictionary_pass_rate`・`ffi_bridge.audit_pass_rate` がすべて 1.0 となることを確認（従来の `auto-*` / `legacy-*` プレースホルダによる欠落は解消済み）。  
- **残タスク**: LSP／Legacy 経路へのテンプレート適用手順と、`timestamp` 生成の最終的な責務分担（`Ptime` への移行可否）を別途整理し、監査チームとの合意を待つ。

## 6. Week31 Day4-5 テスト／ドキュメント反映ログ（2025-10-27）
- `scripts/validate-diagnostic-json.sh` を既定ディレクトリ（`compiler/ocaml/tests/golden/diagnostics`, `compiler/ocaml/tests/golden/audit`）で実行し、スキーマ違反がないことを確認。
- `python3 tooling/ci/collect-iterator-audit-metrics.py --require-success --source compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute.json.golden --source compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute-unknown-tag.json.golden --source compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden --source compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden --source compiler/ocaml/tests/golden/typeclass_dictionary_resolved.json.golden --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-linux.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-macos.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-windows.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/effects-stage.json.golden --audit-source compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden` を完走。`diagnostic.audit_presence_rate` / `typeclass.metadata_pass_rate` / `ffi_bridge.audit_pass_rate` がいずれも `1.0` に到達した。
- 上記に伴い、以下のゴールデンを `phase2.5.audit.v1` テンプレートへ整備:
  `compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute.json.golden`,
  `compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute-unknown-tag.json.golden`,
  `compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden`,
  `compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden`,
  `compiler/ocaml/tests/golden/typeclass_dictionary_resolved.json.golden`（監査キー重複出力の調整を含む）。
- Spec 3.6 に DIAG-002 完了脚注を追加し、`phase2.5.audit.v1` 必須化の合意を記録。`reports/diagnostic-format-regression.md` チェックリストにも `audit` / `timestamp` の確認項目を追記済み。

# 2-5 レビュー記録 — DIAG-001 Week31 Day1-2 現状棚卸し（2025-11-07 更新）

DIAG-001 ステップ 1「現状棚卸しと仕様突合」の調査メモ。Severity 列挙の定義差異と周辺実装の挙動を整理し、後続ステップの改修範囲を明確化する。

## 1. 列挙定義と仕様参照の比較
| 区分 | 参照先 | 列挙内容 / 状態 | 観測メモ |
| ---- | ------ | ---------------- | -------- |
| 仕様 (Chapter 3) | `docs/spec/3-6-core-diagnostics-audit.md:24-43` | `Severity = Error | Warning | Info | Hint` を正式仕様として定義。 | CLI/LSP で情報診断とヒントを区別することを前提にしている。 |
| 仕様 (Chapter 2) | `docs/spec/2-5-error.md:12-55` | `Severity = Error | Warning | Note` のまま据え置き。 | Chapter 3 と不一致。Phase 2-5 でいずれかを統一する必要あり。 |
| 実装 — モデル層 | `compiler/ocaml/src/diagnostic.ml:39-46` | `type severity = Error | Warning | Note`。`severity_label` も 3 値前提。 | `Hint` 相当のバリアントなし。 |
| 実装 — V2 変換 | `compiler/ocaml/src/diagnostic.ml:803-821` | `module V2` で `Severity = Error | Warning | Info | Hint` を定義し、`Note -> Info` へ丸め込み。 | 新バリアントはここでのみ登場。`Hint` 未使用。 |
| JSON スキーマ | `tooling/json-schema/diagnostic-v2.schema.json:14-37` | LSP 準拠で `severity enum = [1,2,3,4]` を要求。 | スキーマ上は `Hint` 値（4）を許容するが、実装側に対応経路がない。 |

## 2. シリアライズと出力経路の挙動
- `compiler/ocaml/src/diagnostic_serialization.ml:249-269` では `severity_to_string` が `note` を出力し、`severity_level_of_severity` が 1/2/3 のみを返却。CLI JSON（`compiler/ocaml/src/cli/json_formatter.ml:90-145`）および LSP トランスポート（`tooling/lsp/lsp_transport.ml:48-116`）はいずれもこの 3 値を前提にしている。
- `compiler/ocaml/src/cli/color.ml:86-102` は `Note` 用の配色を定義しており、`Info`/`Hint` を考慮していない。
- `tooling/ci/collect-iterator-audit-metrics.py:1004-1025` は診断 JSON の集計時に `note -> info` へ正規化し、`hint` も集計カテゴリとして確保しているが現在は未使用。
- `compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden` は `severity: "info"` を保持するが、日本語ラベルや古いフィールド構成が混在しており、`Diagnostic_serialization` 由来の最新形式とは乖離している（改修後に再生成予定）。

## 3. ギャップとフォローアップ
- `Hint` バリアントが仕様に存在する一方で実装経路が未実装のため、Phase 2-5 ステップ 2 での列挙拡張時に CLI/LSP/メトリクスすべてを 4 値対応へ更新する必要がある。
- Chapter 2（`docs/spec/2-5-error.md`）が旧 3 値のままのため、仕様の改訂または脚注での移行方針整理が必要。Chapter 3 の脚注と整合する説明を追加する。
- `reports/diagnostic-format-regression.md` チェックリストには Severity 4 値化のレビューポイントが未記載。DIAG-001 完了時に更新し、情報診断／ヒント診断のゴールデン差分を追跡できるようにする。
- `tooling/json-schema/diagnostic-v2.schema.json` と `scripts/validate-diagnostic-json.sh` は `severity=4` を許容しているが、既存フィクスチャに Hint ケースが存在しない。改修後に AJV フィクスチャを追加する。
- メトリクス集計（`diagnostic.info_hint_ratio` 予定値）を Phase 2-5 で追加する際は、`collect-iterator-audit-metrics.py` の出力拡張と連動させ、旧 `note` データの移行を計画する。

## 4. CLI/LSP/監査パイプライン整合確認（2025-11-09 更新）
- LSP: `tooling/lsp/tests/client_compat/tests/client_compat.test.ts:95` に Info/Hint 専用ケースを追加し、`diagnostic-v2-info-hint.json` で `severity = [3, 4]` を確認。`npm run ci --prefix tooling/lsp/tests/client_compat` を実行し、新フィクスチャが AJV 検証を通過することを確認した。  
- CLI: `compiler/ocaml/tests/golden/diagnostics/severity/info-hint.json.golden` を `scripts/validate-diagnostic-json.sh` で検証し、文字列 Severity が維持されていることと `audit` / `timestamp` が欠落しないことを再確認。  
- 監査メトリクス: `tooling/ci/collect-iterator-audit-metrics.py:993-1036` に `info_fraction` / `hint_fraction` / `info_hint_ratio` を導入し、`python3 tooling/ci/collect-iterator-audit-metrics.py --require-success --source compiler/ocaml/tests/golden/diagnostics/severity/info-hint.json.golden` で Info/Hint の出現比率が `diagnostics.info_hint_ratio` として JSON 出力へ含まれることを確認。  
- ドキュメント: `reports/diagnostic-format-regression.md` へ Info/Hint 用チェックを追加し、Severity 拡張の確認手順をレビュー運用に組み込んだ。

## 5. ドキュメントとメトリクス更新（Week32 Day3, 2025-11-10 更新）
- 仕様反映: `docs/spec/3-6-core-diagnostics-audit.md` に DIAG-001 脚注を追加し、`severity` フィールドが 4 値へ統一された経緯と `Note` 廃止方針を明文化。`Severity` 説明に CLI/LSP/監査での区別運用を追記した。  
- 指標定義: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の指標表へ `diagnostic.info_hint_ratio` を追加し、CI 集計で情報診断とヒント診断の比率を監視できるようにした。`diagnostic.hint_surface_area` は Phase 2-7 で集計実装予定として暫定登録。  
- 集計スクリプト連携: `collect-iterator-audit-metrics.py` のサマリ出力に追従した説明を同文書へ追記し、`info_fraction` / `hint_fraction` / `info_hint_ratio` が `diagnostics.summary` へ記録されることを明示。  
- 残課題: `diagnostic.hint_surface_area` の算出はスパン計測ロジックを追加した後に `tooling/ci/collect-iterator-audit-metrics.py` へ組み込む。Phase 2-7 で CLI テキスト出力刷新と合わせて優先度を再評価する。

# 2-5 レビュー記録 — EFFECT-001 Day1 タグ棚卸し

Phase 2-5 Week31 Day1。`EFFECT-001` のステップ 1（タグ語彙と既存実装の棚卸し）を実施し、仕様と実装のギャップを整理した。

## 1. Phase 2-5 で扱うタグ語彙
| タグ | 区分 | 主な仕様出典 | 想定 API / Capability 例 |
| ---- | ---- | ------------ | ------------------------ |
| `mut` | Σ_core | docs/spec/1-3-effects-safety.md §A | `var` 再代入、`Vec.push`, `Cell.set` |
| `io` | Σ_core | docs/spec/1-3-effects-safety.md §A | `Core.IO.print`, `Core.File.read` |
| `ffi` | Σ_core | docs/spec/1-3-effects-safety.md §A, docs/spec/3-8-core-runtime-capability.md §10 | `extern "C"` 呼び出し、Capability Bridge |
| `panic` | Σ_core | docs/spec/1-3-effects-safety.md §A | `panic`, `assert`, `Result.expect` |
| `unsafe` | Σ_core | docs/spec/1-3-effects-safety.md §A, docs/spec/3-6-core-diagnostics-audit.md §4.2 | `unsafe { … }`, `addr_of`, 生ポインタ操作 |
| `syscall` | Σ_system | docs/spec/1-3-effects-safety.md §A, docs/spec/3-8-core-runtime-capability.md §8 | `Core.System.raw_syscall`, ランタイム Capability `system.call` |
| `process` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Process.spawn_process`, `Capability.process` |
| `thread` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Process.create_thread`, `Capability.thread` |
| `memory` | Σ_system | docs/spec/1-3-effects-safety.md §A, docs/spec/3-4-core-collection.md §5 | `Core.Memory.mmap`, `Core.Memory.mprotect` |
| `signal` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Signal.register_signal_handler` |
| `hardware` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Hardware.rdtsc`, `Capability.hardware` |
| `realtime` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.RealTime.set_scheduler_priority` |
| `audit` | Σ_system | docs/spec/1-3-effects-safety.md §A, docs/spec/3-6-core-diagnostics-audit.md §3 | `Diagnostics.audit_ctx.log`, 監査 Capability |
| `security` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Capability.enforce_security_policy` |
| `mem` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1, docs/spec/3-0-core-library-overview.md §2 | `Core.Collection.Vec.reserve`, `@no_alloc` 連携 |
| `debug` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1 | `Core.Debug.inspect`, `expect_eq` |
| `trace` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1, docs/spec/3-6-core-diagnostics-audit.md §5 | `Core.Diagnostics.emit_trace`, 監査ログ拡張 |
| `unicode` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1, docs/spec/3-3-core-text-unicode.md §4 | `Core.Text.normalize`, Unicode テーブル参照 |
| `time` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1 | `Core.Time.now`, 高精度タイマ |

> 備考: Phase 2-5 では `Σ_core` と `Σ_system` の主要タグを Typer で検出し、`Σ_stdlib` のタグは監査メタデータ補完と脚注整備を優先する。Capability Registry 側の命名はすべて小文字化して突合する必要がある。

## 2. Effect_analysis 実装観察（compiler/ocaml/src/type_inference.ml:37-190）
| 対象 | 現状実装 | 検出漏れ・論点 | 備考 |
| ---- | -------- | -------------- | ---- |
| `TCall` (関数呼出) | `callee_name = "panic"` の場合のみ `add_tag "panic"`。引数は再帰解析。 | `ffi` / `io` / `syscall` / Capability 付き API を識別する経路が存在しない。`Ffi_contract`・`Effect_profile.normalize_effect_name` 未連携。 | `expr.texpr_span` をタグに付与できるため、判別ロジック追加でスパンは再利用可。 |
| `TAssign` / `TAssignStmt` | 左右を再帰的に解析するのみ。 | `mut` タグが付与されない。`docs/spec/1-3-effects-safety.md §E` の再代入制約と乖離。 | `lhs.texpr_span` が利用できるが範囲が Dummy の場合は fallback 必要。 |
| `TVarDecl` / `TLetDecl` | 初期化式を解析するがタグ付与なし。 | `var` 宣言自体が `mut`（再代入許容）であることをタグに反映していない。 | `collect_decl` では宣言種別を判定できるため、`mut` 追加を検討。 |
| `TUnsafe` / `TUnsafe` ブロック | 内部式のみ解析し、自身でタグ付与しない。 | `unsafe` タグおよびブロック内の残余効果へのマーキングが欠落。 | ブロック span が取得可能。`unsafe` ブロック内で検出した他タグに対する扱いも要設計。 |
| `TCall` (外部呼出検出) | `callee_name` を文字列一致でしか評価しない。 | `extern` / Capability Bridge 呼出を `ffi` / `syscall` 等へ分類できない。 | `Ffi_bridge` スナップショット (`record_ffi_bridge_snapshot`) からタグ推論する案を検討。 |
| `Effect_analysis.add_tag` | 小文字化して重複排除。 | Dummy span (`start=0/end=0`) の扱いは `merge_usage_into_profile` 側で補うのみ。 | 追加タグの span を確保できれば `residual_leaks` へ直接反映可能。 |
| `collect_block` / `collect_stmt` | 逐次的に再帰解析。 | 宣言外の `unsafe` / `io` などを検出する入口は `collect_expr` のまま。 | AST から Statement 種別を判定でき、タグ付けの挿入ポイントは明確。 |

## 3. Stage 判定・Capability 連携メモ
- `Type_inference_effect.resolve_function_profile`（compiler/ocaml/src/type_inference_effect.ml:35-115）は `effect_node.effect_capabilities` の先頭要素しか解決せず、残りの Capability 名を破棄している。Phase 2-5 では配列全体を保持し、`resolved_capabilities` 的な構造を導入する余地がある。
- `stage_for_capability` は Capability 名を小文字化して照合するが、複数 Capability の Stage を合成する仕組みがなく、デフォルト Stage (`Stable`) を返すケースが多い。CI で取り込んだ Stage Trace (`runtime_stage.stage_trace`) との突合タイミングも Typer 側で一回のみ。
- `stage_trace_with_typer` は `cli_option` / `env_var` 由来のステップを先頭に保持しつつ `typer` ステップを挿入するが、Capability が複数ある場合でも `capability` フィールドには先頭名しか格納されない。
- `Effect_analysis.merge_usage_into_profile` の `residual_leaks` は `fallback_span` に関数宣言 span を渡しており、タグ追加時にスパンを確保できれば診断へ反映可能。`normalize_effect_name` で小文字化されるため、タグ一覧も小文字で統一する方針が必要。

## 4. 後続タスクへのインパクト
- タグ検出のギャップを埋めるため、`collect_expr`・`collect_decl` への分岐追加と、Capability 判別のための `Ffi_contract` / 標準ライブラリ API テーブルが必要。ホワイトリスト案は次ステップで `docs/plans/bootstrap-roadmap/2-5-review-log.md` に追記する。
- Stage 判定については `resolved_capability` を単一値で保持しているため、EFFECT-003 で予定している複数 Capability 出力に備えて型拡張が必要。`AuditEnvelope.metadata["effects.required"]` への反映計画とも連動させる。
- スパン情報は `expr.texpr_span` と `decl.tdecl_span` で取得できるため、タグ追加時に Diagnostic へ確実に渡す実装方針を後続工程でまとめる。

## SYNTAX-002 Day1 調査（2025-10-27）

SYNTAX-002 `use` 多段ネスト対応計画のステップ S1（現状棚卸し）結果。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md`](./2-5-proposals/SYNTAX-002-proposal.md)

### 1. 仕様と実装の突き合わせ
- `docs/spec/1-1-syntax.md:68-86` で `use Core.Parse.{Lex, Op.{Infix, Prefix}}` のような多段ネストを明示。Formal BNF でも `UseItem ::= Ident ["as" Ident] [ "." UseBrace ]`（`docs/spec/1-5-formal-grammar-bnf.md:24-33`）と再帰展開を認めている。
- AST 定義は `compiler/ocaml/src/ast.ml:372-389` で `item_nested : use_item list option` を保持し、構文木レベルではネストを受け入れる前提になっている。
- Menhir 実装は（修正前の）`compiler/ocaml/src/parser.mly:758-792` で `UseBrace` を構築するが、`use_item` 生成時に常に `{ item_nested = None }` を設定しており、`item_nested` に子要素を格納する経路が存在しない。
- 結果として `. {` 以降で構文エラーが発生し、Chapter 1 のサンプルおよび Formal BNF と実装の間にギャップが残っている。

### 2. 再現手順
1. `cd compiler/ocaml`
2. テスト用ファイル `tmp/use_nested.reml` を作成:
   ```reml
   module sample

   use Core.Parse.{Lex, Op.{Infix, Prefix}}
   ```
3. `dune exec remlc -- --emit-ast tmp/use_nested.reml` を実行すると、`tmp/use_nested.reml:3:24: エラー (構文解析)` が出力され、`Op.{` の直前で解析が停止する。
4. 実行後は `rm tmp/use_nested.reml` でクリーンアップする。

### 3. 修正対象メモ
- `parser.mly` に `use_item` 再帰分岐を追加し、子リストを `item_nested` に格納する必要がある。`items @ [item]` の線形結合は既存のため、保持構造の変更は最小で済む想定。
- `parser_diag_state` / `parser_driver` の期待集合および FixIt は `ERR-001` と連携して更新する。ネスト展開を受理した際の診断メッセージ差分を共有する準備が必要。
- Formal BNF と Chapter 1 の記述に変更不要であることを確認済み。実装側の修正と AST プリンタのテスト追加でギャップ解消が可能。

## SYNTAX-002 Day1-2 AST/型付きAST整合確認（2025-10-27）

S2（AST/型付き AST 整合確認）の結果共有。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md`](./2-5-proposals/SYNTAX-002-proposal.md)

### 1. AST と設計メモの確認
- `compiler/ocaml/src/ast.ml:372-389` の `use_tree`/`use_item` は `item_nested : use_item list option` を保持しており、構造上の拡張は既に定義済み。`compiler/ocaml/docs/parser_design.md` へ同内容を再確認する脚注を追加。
- AST プリンタ (`compiler/ocaml/src/ast_printer.ml:452-490`) は `item_nested` を再帰的に出力できる実装になっており、多段構造を持つ `use` が構築されてもシリアライズに追加対応は不要。

### 2. 型付き AST と Typer の追跡
- `compiler/ocaml/src/typed_ast.ml:150-163` では `typed_compilation_unit.tcu_use_decls` を `use_decl list` のまま保持し、`use_item` の構造を変換しない設計であることを確認。
- `compiler/ocaml/src/type_inference.ml:2796-2833` でコンパイル単位を生成する際に `tcu_use_decls = cu.uses` としており、Menhir が `item_nested` を埋めれば Typer 側への伝播がそのまま成立する。

### 3. ギャップ評価と次ステップ
- 型付き AST と Typer に追加改修は現時点で不要。S3 以降は Menhir で `item_nested` を組み立てる実装に集中できる。
- S5 で予定しているメトリクス追加（`parser.use_nested_support`）は、AST/Typer 側がネスト情報を保持できる前提の上に計測を構築する方針で問題なし。

## SYNTAX-002 Day2-3 Menhir ルール実装（2025-10-28）

S3（Menhir ルール実装）の結果共有。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md`](./2-5-proposals/SYNTAX-002-proposal.md)

### 1. 実装内容
- `compiler/ocaml/src/parser.mly:780-804` の `use_item` を `ident` + `as` + `.{...}` の再帰構築へ変更し、`item_nested` に `Some nested` を設定できるよう `use_item_nested_opt` 非終端記号を追加。これにより `use Core.Parse.{Lex, Op.{Infix, Prefix}}` などの構文を Menhir レベルで受理可能になった。

### 2. 検証手順
1. `cd compiler/ocaml/src`
2. `menhir --list-errors parser.mly` を実行し、`parser.conflicts`／`parser.automaton` を再生成。既存の shift/reduce / reduce/reduce 件数に変化が無いこと、およびネスト分岐追加による新規コンフリクトが発生しないことを確認した（差分なし）。
3. 生成結果は `ERR-001` チームへ共有し、期待集合リストに変化が無いことのフィードバックを取得。

### 3. フォローアップ
- S4 で予定している Typer／診断連携へ向けて、`tcu_use_decls` の利用箇所（`type_inference.ml`）にネスト構造を踏まえた再帰探索が必要か評価する。
- S5 でのテスト追加（`test_parser.ml`）および CLI ゴールデン更新を行う際は、今回の Menhir 修正に基づいた AST 期待値をベースラインとする。

## SYNTAX-002 Day3-4 束縛診断連携（2025-10-29）

S4（束縛・診断連携）の結果共有。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md`](./2-5-proposals/SYNTAX-002-proposal.md)

### 1. 実装内容
- `compiler/ocaml/src/module_env.ml` を新設し、`flatten_use_decls` で `use` ネストを `binding_local`／`binding_path`／`binding_is_pub` に展開する `use_binding` レコードを導入。
- 型付き AST (`typed_ast.ml:156-164`) に `tcu_use_bindings` を追加し、`type_inference.ml:2796-2833` で Typer 完了時に束縛リストを生成。今後のモジュール解決や診断で再利用できる共有データを確保。
- `compiler/ocaml/tests/test_module_env.ml` を追加し、単純な `use`／`alias`／多段ネスト／`pub use` の 4 ケースを検証。展開結果（ローカル名・解決パス・pub フラグ）が仕様と一致することを確認した。

### 2. 診断影響の確認
- `parser_diag_state.ml` の最遠エラー集約と期待集合のソートは `use` 展開に依存していないため追加変更は不要。`menhir --list-errors parser.mly` 実行結果にも S3 からの差分がないことを再確認。
- `ERR-001` 計画へ「S4 完了時点で期待集合の変化が無い」旨を共有し、FixIt 拡張の追従は不要であることを合意済み。

### 3. フォローアップ
- `Module_env.use_binding` を Phase 2-7 再エクスポート解決タスクへ引き渡し、`binding_local` 名で型環境へ取り込む処理を設計する。
- S5 で予定している `parser.use_nested_support` メトリクス算出は `flatten_use_decls` の結果を基に成功率を評価する。

## SYNTAX-002 Day4-5 検証・ドキュメント更新（2025-11-12）

S5（検証とドキュメント更新）の結果共有。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md`](./2-5-proposals/SYNTAX-002-proposal.md)

### 1. テストと検証
- `compiler/ocaml/tests/test_parser.ml` に多段ネスト `use` を検証するユニットテストを追加。`UseBrace` 配下で `item_nested` が `Some [...]` となり、`Op.{Infix, Prefix}` が再帰的に構築されることを直接確認するヘルパー（`expect_use_nested`）を実装。
- `compiler/ocaml/tests/test_module_env.ml` と併せて `dune runtest compiler/ocaml/tests/test_parser.exe` および `dune runtest compiler/ocaml/tests/test_module_env.exe` を実行し、`flatten_use_decls` まで含めた再エクスポート展開が成功することを確認。（CI 連携時は `dune runtest` 全体で取得したログを `reports/diagnostic-format-regression.md` に添付予定。）

### 2. メトリクスとドキュメント
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `parser.use_nested_support` を追加し、`dune runtest` 完了後に `tooling/ci/collect-iterator-audit-metrics.py --summary` で収集する運用を明記。成功率が 1.0 未満の場合は Phase 2-7 Parser チームへ即時エスカレーションする。
- 仕様側では `docs/spec/1-5-formal-grammar-bnf.md` に脚注を追加し、`UseItem` の再帰規則と実装・監視体制を記録。`docs/spec/3-0-core-library-overview.md` には Core.* の再エクスポートが同機能に依存する旨を追記し、標準ライブラリ観点からのギャップが解消されたことを明示した。

### 3. フォローアップ
- `parser.use_nested_support` を CI ダッシュボードへ表示する際の閾値設定と、失敗時に収集する追加ログ（Menhir `--list-errors` 出力など）のテンプレートを Phase 2-7 で整備する。
- `pub use` の可視性ルール検証は Phase 2-7 `SYNTAX-002` 後続タスクへ引き継ぐ。`binding_is_pub` を利用した公開面積の測定は `Module_env` で準備済み。

## PARSER-002 Day4 RunConfig クライアント統合（2025-11-21）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 実装サマリ
- `compiler/ocaml/src/cli/options.ml` に `Cli.Options.to_run_config` を追加し、`--require-eof` / `--packrat` / `--left-recursion` / `--no-merge-warnings` を経由して RunConfig を構築。従来 CLI が暗黙に設定していた `require_eof=true` は仕様の既定値（false）へ合わせ、互換モードはフラグ指定で明示する方針に変更した。
- `compiler/ocaml/src/main.ml` で `Parser_driver.run ~config` を採用し、RunConfig を経由したパース結果を既存パイプラインへ接続。`Test_support` を新設してユニットテストから同一の RunConfig を再利用できるようにし、`test_parser.ml` / `test_type_inference.ml` をヘルパ経由へ移行した。
- LSP 側に `tooling/lsp/run_config_loader.ml` を追加し、`tooling/lsp/config/default.json` に定義した設定から `extensions["lex"|"recover"|"stream"]` を復元するロードパスを定義。CLI と同様に `extensions["config"].source = "lsp"` を記録してトレースの出所を区別できるようにした。

### 2. 測定・ドキュメント更新
- `reports/diagnostic-format-regression.md` のローカル検証手順に RunConfig 切替シナリオを追加し、`extensions.config.*` の差分を比較できるチェックリストを整備。
- RunConfig 移行時に残っている Packrat/左再帰シムの未実装事項を `docs/notes/core-parser-migration.md` に TODO として記録し、今後の追跡先（`PARSER-003`・`LEXER-002`）を明文化。

### 3. フォローアップ
- `--packrat` / `--left-recursion=on|auto` は現状警告のみを発する暫定実装のため、`PARSER-003` でメモ化シムが揃い次第 CLI/LSP ランタイムを再検証する。
- LSP 設定を利用した自動テストは未整備。`tooling/lsp/tests/client_compat` に RunConfig フィクスチャを追加し、`run_config_loader` 経由で CLI と同じ JSON 出力になることを確認するタスクを Phase 2-7 へ引き継ぐ。

## PARSER-002 Day5 RunConfig テスト・メトリクス整備（2025-11-22）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 実装サマリ
- `compiler/ocaml/tests/run_config_tests.ml` を新設し、`require_eof` 上書き・`merge_warnings` の重複許容・`trace` による SpanTrace 収集・`extensions["lex"]` デコード・Legacy ブリッジ互換を個別に検証するユニットテストを追加。`dune` テストリストへ `run_config_tests` を組み込み、parser スイートから常時実行されるようにした。
- `compiler/ocaml/tests/golden/diagnostics/parser/parser-runconfig-packrat.json.golden` を作成し、`parser.runconfig.packrat_unimplemented` / `parser.runconfig.left_recursion_unimplemented` 警告を含む CLI 出力を記録。`run_config.switches` と `audit_metadata.parser.runconfig.*` キーを通じて Packrat / 左再帰 / trace / merge_warnings / lex / recover / stream の値が JSON・監査ログに保存されることを示した。
- `tooling/ci/collect-iterator-audit-metrics.py` に `collect_runconfig_metrics` を追加し、`parser.runconfig_switch_coverage` と `parser.runconfig_extension_pass_rate` を集計。`--require-success` 時には両指標が 1.0 未満の場合に失敗させるゲートを組み込んだ。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の指標表に上記 2 指標を追記し、Phase 2-5 以降の監視対象へ正式追加した。

### 2. 検証・運用手順
- `scripts/validate-diagnostic-json.sh` 既定ターゲットに新ゴールデンを配置したため、既存フロー（parser ゴールデン→AJV 検証）で RunConfig サンプルも同時検査される。`collect-iterator-audit-metrics.py --require-success --source compiler/ocaml/tests/golden/diagnostics/parser/parser-runconfig-packrat.json.golden` をローカルで実行し、新指標が `pass_rate=1.0` になることを確認（CI 統合待ち）。
- `dune runtest parser` に `run_config_tests` が追加されたため、Packrat/左再帰の警告と Legacy ブリッジ互換性が常時テストされる。CI ログで新テストの標準出力（`✓ ...`）が確認できるように `Printf` レポート形式を既存テストと合わせた。

### 3. フォローアップ
- RunConfig メトリクスは CLI ゴールデンの存在に依存するため、LSP 側から生成した JSON を追加し `extensions["stream"]` などのバリエーションを拡張する。Phase 2-7 `EXEC-001` と連携し、ストリーミング PoC の JSON も `parser.runconfig_extension_pass_rate` で評価できるようにする。
- Packrat / 左再帰シム実装後には `parser.runconfig_switch_coverage` のサンプルを更新し、警告コードの代わりにメモ化が有効化された証跡（監査ログの `parser.runconfig.packrat.enabled` 等）を測定できるよう指標定義を再検討する。
- 既存 TODO（`PARSER-002 Day1` の項目 3,4）と合わせ、RunConfig 指標の推移を `reports/audit/index.json` に export するスクリプト整備を Phase 2-7 で計画する。

## PARSER-002 Day6 RunConfig 共有・レビュー記録（2025-11-24）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 共有サマリ
- `docs/spec/2-1-parser-type.md` §D `RunConfig` に CLI/LSP 共通設定の利用例を追記し、`with_extension` を用いて `extensions["lex"|"recover"|"stream"]` を同一値で供給する手順を明文化。実装脚注[^runconfig-ocaml-phase25-log] を更新し、Phase 2-5 Step6 で CLI/LSP が `parser_run_config` を共有する構成へ移行したことを記録した。
- `docs/spec/2-6-execution-strategy.md` §B-2 に RunConfig スイッチの運用メモ（CLI/LSP/ストリーミングでの共有ポリシー）を追加し、`parser_driver`・`run_stream` が同一 RunConfig を参照することを脚注で明記した。
- `docs/guides/core-parse-streaming.md` §9 を更新し、RunConfig 共有時に CLI 側の JSON 設定をストリーミング経路へ引き渡すワークフローと `parser-runconfig-packrat.json.golden` を用いた検証手順を紹介した。

### 2. レビュー記録とリンク
- `docs/notes/core-parser-migration.md` に Phase 2-5 RunConfig 移行ステップを追記し、完了タスク（Step1〜6）と残課題（Packrat 実装、LSP 自動テスト、監査指標拡張）を一覧化した。今後の検証先として `PARSER-003`・`LEXER-002`・`EXEC-001` を明示。
- 仕様変更箇所を `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md` Step6 更新メモへリンクし、2-5 ステアリングレビューで確認できるよう脚注番号を同期した。
- 共有結果を 2-5 レビュー会議へ提出し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` の該当エントリから各資料へ遷移できるよう相互リンクを確認した。

### 3. 残課題
- LSP 側で RunConfig フィクスチャを用いた自動テストが未整備のため、Phase 2-7 `EXEC-001` タスクで `tooling/lsp/tests/client_compat` にストリーミング設定を追加する。
- Packrat/左再帰シムが完成した際には、仕様脚注を更新して暫定警告コードの撤廃タイムラインを追記し、`parser.runconfig_switch_coverage` 指標を再評価する。
- `RunConfig.locale` と `Diagnostic` のロケール同期は `DIAG-003` の判断待ち。仕様脚注に暫定運用（CLI/LSP は未指定時に英語へフォールバック）を記載しているため、決定次第ガイドと脚注を更新する。

[^runconfig-ocaml-phase25-log]: `docs/spec/2-1-parser-type.md` と `docs/spec/2-6-execution-strategy.md` の脚注参照。`compiler/ocaml/src/main.ml` および `tooling/lsp/run_config_loader.ml` で `parser_run_config` の共有初期化を実施した記録を反映している。

## LEXER-002 Day1 Core.Parse.Lex ギャップ調査（2025-11-25）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md`](./2-5-proposals/LEXER-002-proposal.md)

### 1. 調査サマリ
- `docs/spec/2-3-lexer.md` が要求する `Core.Parse.Lex` 公開 API（`lexeme` / `symbol` / `config_trivia` / `config_symbol` ほか）を提供するモジュールが現行実装に存在せず、`parser_driver` は `Lexer.token` を直接呼び出しているため `RunConfig.extensions["lex"]` や `ParserId` による共有が機能していない[^lex-spec-api][^parser-driver-token]。
- `parser_run_config.Lex.Trivia_profile` は仕様と同形のフィールドを持つが、`lexer.mll` では `space_id` / `profile` / `doc_comment` 等を参照せず、`shebang` や `hash_inline` の挙動も未実装である[^parser-run-config][^lexer-comment]。
- 現在 `RunConfig.extensions["lex"]` を読むコードはユニットテストのみであり、CLI/LSP/ランナーは `lex` ネームスペースを設定・検証していない。`config_trivia` 相当のユーティリティも欠如しているため設定値が死蔵している[^run-config-tests][^config-trivia-spec]。
- Streaming 系タスクとの依存関係（`ParserId` の安定化、`RunConfig` の伝播、Streaming PoC での lex 再利用）を整理し、`docs/notes/core-parse-streaming-todo.md` に共有メモを追加した。

### 2. 仕様との差分要約

#### 表1: `ConfigTriviaProfile` と `Run_config.Lex.Trivia_profile` の比較
| フィールド / 契約 | 仕様 `ConfigTriviaProfile` | 現行実装 `Run_config.Lex.Trivia_profile` | 差分・課題 |
| --- | --- | --- | --- |
| `line: List<Str>` | コメント接頭辞を列挙し、`config_trivia` で空白スキップに合成する。既定は `["//"]`。 | `line` フィールドあり。`strict_json`/`json_relaxed`/`toml_relaxed` を定義。 | 値は保持するものの、`lexer.mll` で `#` など追加接頭辞を処理せず未使用。 |
| `block: List<CommentPair>` | `start`/`end`/`nested` を保持し、ネスト可否既定は `true`。 | `comment_pair` 型は `start`/`stop`/`nested`。既定プロファイルは `nested=false` を手動設定。 | フィールド名が `stop` となっており `ConfigTriviaProfile` とのマッピング関数が未実装。ネスト再帰処理は `lexer.mll` に固定値。 |
| `shebang: Bool` | 先頭行のみ `#!` を読み飛ばす。 | `shebang` フィールド保持。 | `lexer.mll` は shebang を認識せず、値が常に未使用。 |
| `hash_inline: Bool` | `#` 以降を行コメント扱いにする。 | フィールド保持。 | `lexer.mll` は `//` と `/* */` のみ対応。`#` コメントはエラー扱い。 |
| `doc_comment: Option<Str>` | ドキュメントコメントを診断ノートへ反映。 | フィールド保持。 | `lexer.mll` で `doc_comment` を判別・通知する経路が存在しない。 |
| `config_trivia` / `config_lexeme` / `config_symbol` | `ConfigTriviaProfile` を受け取り、空白・コメント・トークン処理を共通化。 | 未実装。`Run_config.Lex` は値のデコードのみ。 | Lex API 抽出時に新規モジュールを設けてユーティリティを再構築する必要。 |

#### 表2: `RunConfig.extensions["lex"]` 利用状況
| 利用箇所 | 種別 | 現状 | 課題 |
| --- | --- | --- | --- |
| `compiler/ocaml/src/parser_driver.ml` | ランナー | 未使用。`Run_config.Lex.of_run_config` も呼ばれない。 | Lex 設定をランナー初期化へ渡し、`space_id` や `profile` を共有する配線が必要。 |
| `compiler/ocaml/tests/run_config_tests.ml` | ユニットテスト | `Run_config.Lex.of_run_config` で `profile`/`space_id` を復元するテストのみ。 | 実運用経路（CLI/LSP/テストランナー）での検証が欠如。 |
| `parser_run_config` 以外のモジュール | 共通処理 | 存在せず。値の読込・検証は未実装。 | `Core.Parse.Lex` 抽出後に共有ヘルパを追加し、`RunConfig` からの読み出しを一本化する必要。 |

### 3. TODO / 引き継ぎ
1. Step1 で `core_parse_lex.{mli,ml}` を新設し、`ConfigTriviaProfile` マッピングと `lexeme`/`symbol` 等のユーティリティを導入する。
2. `lexer.mll` に `shebang`・`hash_inline`・`doc_comment` の分岐を追加し、`Run_config.Lex.Trivia_profile` からの設定を反映できるよう改修する（Unicode XID 対応も同時に検討）。
3. `parser_driver` と CLI/LSP 初期化コードを更新し、`RunConfig.extensions["lex"]` の値を `Core.Parse.Lex` モジュールへ伝播しつつ、監査メトリクス（例: `lexer.shared_profile_pass_rate`）を 0-3 メトリクス表へ登録する。

### 4. 実施記録
- 2025-11-25: Step0 調査を完了し、本ログと `LEXER-002` 計画書にサマリを反映。Streaming TODO ノートへ依存関係を追記し、次工程（Step1 設計）で参照できる状態にした。

[^lex-spec-api]: `docs/spec/2-3-lexer.md` §C〜§L。
[^parser-driver-token]: `compiler/ocaml/src/parser_driver.ml` の `run` 実装で `Lexer.token` を直接呼び出している。
[^parser-run-config]: `compiler/ocaml/src/parser_run_config.ml` モジュール `Lex`。
[^lexer-comment]: `compiler/ocaml/src/lexer.mll` の `token` ルールは `//`/`/* */` のみ対応し、`shebang`/`#` コメントを扱わない。
[^run-config-tests]: `compiler/ocaml/tests/run_config_tests.ml` `test_lex_extension_profile`。
[^config-trivia-spec]: `docs/spec/2-3-lexer.md` §G-1（`ConfigTriviaProfile` と `config_trivia` 系ユーティリティ）。
